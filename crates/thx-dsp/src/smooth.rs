//! Smoothing for config values.
//!
//! A config field is either smoothed ([`Smooth<T>`]) or instant (a bare scalar).
//! Both implement [`Param`], so a block treats them uniformly; only [`Smooth`]
//! actually ramps, one sample at a time via [`advance`](Smooth::advance).
//!
//! A [`Smooth`] is four scalars the block owns — no buffers, no allocation. Only
//! the `target` serializes, so a config still round-trips as a bare number.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::sample::Sample;

/// Default ramp duration, in milliseconds.
const DEFAULT_RAMP_MS: f64 = 10.0;

/// A value that ramps linearly toward its target over a fixed time.
///
/// Construct with [`new`](Self::new) (10 ms ramp) or [`with_time`](Self::with_time),
/// then [`prepare`](Self::prepare) once before processing to fix the ramp length.
#[derive(Clone, Debug)]
pub struct Smooth<T: Sample = f64> {
    target: T,
    current: T,
    /// Per-sample increment while ramping.
    step: T,
    /// Samples left until `current` reaches `target`.
    remaining: usize,
    /// Ramp length in samples (0 until prepared).
    ramp_samples: usize,
    time_ms: f64,
}

impl<T: Sample> Smooth<T> {
    /// A value settled at `target`, ramping over the default 10 ms.
    pub fn new(target: T) -> Self {
        Self::with_time(target, DEFAULT_RAMP_MS)
    }

    /// A value settled at `target`, ramping over `time_ms` milliseconds.
    pub fn with_time(target: T, time_ms: f64) -> Self {
        Self {
            target,
            current: target,
            step: T::ZERO,
            remaining: 0,
            ramp_samples: 0,
            time_ms,
        }
    }

    /// Fix the ramp length for `sample_rate` and settle at the target.
    pub fn prepare(&mut self, sample_rate: f64) {
        self.ramp_samples = (self.time_ms / 1000.0 * sample_rate).round().max(0.0) as usize;
        self.settle();
    }

    /// Current target.
    pub fn target(&self) -> T {
        self.target
    }

    /// Aim at a new target, ramping from the current value. No-op if unchanged.
    pub fn set_target(&mut self, target: T) {
        if target == self.target {
            return;
        }
        self.target = target;
        if self.ramp_samples == 0 {
            self.current = target;
            self.remaining = 0;
            return;
        }
        let distance = target.to_f64() - self.current.to_f64();
        self.step = T::from_f64(distance / self.ramp_samples as f64);
        self.remaining = self.ramp_samples;
    }

    /// Jump straight to the target, ending any ramp in progress.
    pub fn settle(&mut self) {
        self.current = self.target;
        self.step = T::ZERO;
        self.remaining = 0;
    }

    /// Advance one sample and return the new current value.
    pub fn advance(&mut self) -> T {
        if self.remaining > 0 {
            self.current = self.current + self.step;
            self.remaining -= 1;
            if self.remaining == 0 {
                self.current = self.target;
            }
        }
        self.current
    }

    /// A copy aimed at `target`, keeping the ramp time (used by `validate`).
    pub fn with_target(&self, target: T) -> Self {
        Self::with_time(target, self.time_ms)
    }
}

impl<T: Sample> Default for Smooth<T> {
    fn default() -> Self {
        Self::new(T::ZERO)
    }
}

impl<T: Sample + Serialize> Serialize for Smooth<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.target.serialize(serializer)
    }
}

impl<'de, T: Sample + Deserialize<'de>> Deserialize<'de> for Smooth<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Smooth::new)
    }
}

/// The control-side operations shared by smoothed and instant config values.
///
/// Implemented for [`Smooth<T>`] and for bare scalars (`f64`, `f32`), so a block
/// can drive either through the same calls. Per-sample access lives on
/// [`Smooth::advance`].
pub trait Param<T: Sample> {
    /// Current target.
    fn target(&self) -> T;
    /// Aim at a new target.
    fn set_target(&mut self, target: T);
    /// Jump straight to the target.
    fn settle(&mut self);
    /// Fix any ramp length for `sample_rate`. Control thread.
    fn prepare(&mut self, sample_rate: f64);
}

impl<T: Sample> Param<T> for Smooth<T> {
    fn target(&self) -> T {
        self.target()
    }
    fn set_target(&mut self, target: T) {
        self.set_target(target);
    }
    fn settle(&mut self) {
        self.settle();
    }
    fn prepare(&mut self, sample_rate: f64) {
        self.prepare(sample_rate);
    }
}

/// Instant (un-smoothed) parameter: the value is its own target.
macro_rules! impl_param_scalar {
    ($t:ty) => {
        impl Param<$t> for $t {
            fn target(&self) -> $t {
                *self
            }
            fn set_target(&mut self, target: $t) {
                *self = target;
            }
            fn settle(&mut self) {}
            fn prepare(&mut self, _sample_rate: f64) {}
        }
    };
}

impl_param_scalar!(f64);
impl_param_scalar!(f32);

#[cfg(test)]
mod tests {
    use super::*;

    fn take(s: &mut Smooth<f64>, n: usize) -> Vec<f64> {
        (0..n).map(|_| s.advance()).collect()
    }

    #[test]
    fn starts_settled_at_target() {
        let mut s = Smooth::<f64>::new(0.5);
        s.prepare(48_000.0);
        assert_eq!(take(&mut s, 8), vec![0.5; 8]);
    }

    #[test]
    fn ramps_monotonically_to_new_target() {
        let mut s = Smooth::<f64>::new(0.0);
        s.prepare(1_000.0); // 10 ms => 10-sample ramp
        s.set_target(1.0);

        let out = take(&mut s, 16);
        assert!(out.windows(2).all(|w| w[1] >= w[0]), "{out:?}");
        assert!((out[0] - 0.1).abs() < 1e-9, "first step is 1/10, got {}", out[0]);
        assert_eq!(out[15], 1.0, "settles exactly at target and holds");
    }

    #[test]
    fn settle_snaps_past_the_ramp() {
        let mut s = Smooth::<f64>::new(0.0);
        s.prepare(1_000.0);
        s.set_target(1.0);
        s.settle();
        assert_eq!(take(&mut s, 4), vec![1.0; 4]);
    }

    #[test]
    fn unchanged_target_is_a_noop() {
        let mut s = Smooth::<f64>::new(0.25);
        s.prepare(48_000.0);
        s.set_target(0.25);
        assert_eq!(take(&mut s, 4), vec![0.25; 4]);
    }

    #[test]
    fn serializes_as_a_bare_number() {
        let s = Smooth::<f64>::new(-6.0);
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "-6.0");
        let back: Smooth<f64> = serde_json::from_str("-6.0").unwrap();
        assert_eq!(back.target(), -6.0);
    }
}
