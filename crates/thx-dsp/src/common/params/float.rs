//! [`FloatParam`]: a continuous, range-clamped parameter with built-in
//! per-sample smoothing.

use core::ops::RangeInclusive;

use crate::common::sample::Sample;

use super::Param;
use super::smoother::{Smoother, SmoothingStyle};

/// A continuous parameter (gain, cutoff, mix, …).
///
/// The control thread sets a plain `f64` target with [`set`](Self::set); it is
/// clamped to the configured range and handed to the embedded [`Smoother`]. The
/// audio thread reads the smoothed [`Sample`] stream with [`next`](Self::next) /
/// [`next_block`](Self::next_block), which are wait-free.
///
/// Because the parameter owns its smoother and all of that state is atomic, a
/// `FloatParam` can live in a struct shared (`Arc`) between the two threads — the
/// atomic target *is* the cross-thread publication mechanism, so no separate
/// channel is needed for independent scalar parameters.
pub struct FloatParam<S: Sample> {
    smoother: Smoother<S>,
    default: f64,
    range: RangeInclusive<f64>,
}

impl<S: Sample> FloatParam<S> {
    /// A parameter spanning `range`, resting at `default` (clamped into range),
    /// smoothed according to `style`.
    pub fn new(default: f64, range: RangeInclusive<f64>, style: SmoothingStyle) -> Self {
        let default = default.clamp(*range.start(), *range.end());
        Self {
            smoother: Smoother::new(style, default),
            default,
            range,
        }
    }

    /// Aim for a new `value` (clamped to range), ramped at `sample_rate`. Heavy
    /// path: control thread only, never the audio thread.
    pub fn set(&self, sample_rate: f64, value: f64) {
        let value = value.clamp(*self.range.start(), *self.range.end());
        self.smoother.set_target(sample_rate, value);
    }

    /// The next smoothed sample. Wait-free: audio thread only.
    pub fn next(&self) -> S {
        self.smoother.next()
    }

    /// Fill `out` with successive smoothed samples. Wait-free: audio thread only.
    pub fn next_block(&self, out: &mut [S]) {
        self.smoother.next_block(out);
    }

    /// Whether the value is still ramping toward its target.
    pub fn is_smoothing(&self) -> bool {
        self.smoother.is_smoothing()
    }

    /// The valid range of this parameter.
    pub fn range(&self) -> &RangeInclusive<f64> {
        &self.range
    }
}

impl<S: Sample> Param for FloatParam<S> {
    type Plain = f64;

    fn default_value(&self) -> f64 {
        self.default
    }

    fn value(&self) -> f64 {
        self.smoother.target()
    }

    fn reset(&self) {
        self.smoother.snap_to_target();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_default_and_set() {
        let p = FloatParam::<f64>::new(5.0, 0.0..=1.0, SmoothingStyle::None);
        assert_eq!(p.default_value(), 1.0); // clamped
        p.set(48_000.0, -3.0);
        assert_eq!(p.value(), 0.0); // clamped
        assert_eq!(p.next(), 0.0);
    }

    #[test]
    fn reset_snaps_to_target() {
        let p = FloatParam::<f64>::new(0.0, 0.0..=1.0, SmoothingStyle::Linear(1000.0));
        p.set(48_000.0, 1.0);
        assert!(p.is_smoothing());
        p.reset();
        assert!(!p.is_smoothing());
        assert_eq!(p.next(), 1.0);
    }
}
