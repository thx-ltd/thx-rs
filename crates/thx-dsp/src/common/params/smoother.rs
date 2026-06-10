//! [`Smoother`]: the per-sample value ramp that gives every numeric parameter
//! its zipper-noise-free response.
//!
//! A smoother is the bridge across the realtime boundary. The **control thread**
//! sets a new *target* with [`set_target`](Smoother::set_target) (cheap, but only
//! ever called off the audio thread). The **audio thread** pulls the *smoothed*
//! value one sample at a time with [`next`](Smoother::next), which is wait-free:
//! no allocation, no locking, just relaxed atomic loads/stores.
//!
//! All state lives in atomics so a smoother can sit inside a parameter that is
//! shared (via `Arc`) between the controller and the processor. Only the audio
//! thread mutates the ramp in [`next`](Smoother::next), and only the control
//! thread retargets it, so the relaxed ordering is sufficient — a retarget that
//! races a `next` is simply observed on the following sample.

use core::marker::PhantomData;
use core::sync::atomic::{AtomicI32, AtomicU64, Ordering::Relaxed};

use crate::common::sample::Sample;

/// How a [`Smoother`] travels from its current value to a new target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SmoothingStyle {
    /// Jump to the target immediately, no ramp.
    None,
    /// Constant-rate linear ramp over the given duration in **milliseconds**.
    Linear(f64),
    /// One-pole exponential approach with the given time constant in
    /// **milliseconds** (the value covers ~63% of the remaining distance per
    /// time constant). Snaps to the target after ~5 time constants so smoothing
    /// reliably terminates.
    Exponential(f64),
}

/// A lock-free `f64` cell, stored as the `u64` bit pattern so it can live in an
/// atomic. All accesses are `Relaxed`; see the module docs for why that is sound
/// for the single-producer/single-consumer smoother.
struct AtomicF64(AtomicU64);

impl AtomicF64 {
    fn new(value: f64) -> Self {
        Self(AtomicU64::new(value.to_bits()))
    }

    fn load(&self) -> f64 {
        f64::from_bits(self.0.load(Relaxed))
    }

    fn store(&self, value: f64) {
        self.0.store(value.to_bits(), Relaxed);
    }
}

/// Ramps a value toward a target over time, producing the per-sample stream the
/// audio thread reads. Smooths internally in `f64` and yields the processor's
/// [`Sample`] type `S` from [`next`](Self::next).
///
/// See the [parameters module](crate::common::params) for the threading contract.
pub struct Smoother<S: Sample> {
    style: SmoothingStyle,
    /// The value being ramped toward.
    target: AtomicF64,
    /// The most recently produced value.
    current: AtomicF64,
    /// `Linear`: per-step delta. `Exponential`: the one-pole coefficient.
    increment: AtomicF64,
    /// Samples of ramp remaining; `<= 0` means settled (output == target).
    steps_left: AtomicI32,
    _sample: PhantomData<fn() -> S>,
}

impl<S: Sample> Smoother<S> {
    /// A smoother of the given `style`, settled at `initial`.
    pub fn new(style: SmoothingStyle, initial: f64) -> Self {
        Self {
            style,
            target: AtomicF64::new(initial),
            current: AtomicF64::new(initial),
            increment: AtomicF64::new(0.0),
            steps_left: AtomicI32::new(0),
            _sample: PhantomData,
        }
    }

    /// The current target value (what [`next`](Self::next) is ramping toward).
    pub fn target(&self) -> f64 {
        self.target.load()
    }

    /// Whether a ramp is still in progress. Once this is `false`,
    /// [`next`](Self::next) returns the target unchanged.
    pub fn is_smoothing(&self) -> bool {
        self.steps_left.load(Relaxed) > 0
    }

    /// Aim for a new `target`, computing the ramp for `sample_rate`. Control
    /// thread only — never call from the audio thread.
    pub fn set_target(&self, sample_rate: f64, target: f64) {
        self.target.store(target);
        let current = self.current.load();

        match self.style {
            SmoothingStyle::None => {
                self.current.store(target);
                self.steps_left.store(0, Relaxed);
            }
            SmoothingStyle::Linear(ms) => {
                let steps = duration_to_steps(ms, sample_rate);
                self.increment.store((target - current) / steps as f64);
                self.steps_left.store(steps, Relaxed);
            }
            SmoothingStyle::Exponential(ms) => {
                let tau = (ms / 1000.0) * sample_rate;
                if tau <= 0.0 {
                    self.current.store(target);
                    self.steps_left.store(0, Relaxed);
                } else {
                    // y[n] = y[n-1] + (x - y[n-1]) * (1 - e^{-1/tau}); settle at ~5 tau.
                    self.increment.store(1.0 - (-1.0 / tau).exp());
                    self.steps_left
                        .store((tau * 5.0).round().max(1.0) as i32, Relaxed);
                }
            }
        }
    }

    /// The next smoothed sample. Wait-free: audio thread only.
    pub fn next(&self) -> S {
        let steps = self.steps_left.load(Relaxed);
        if steps <= 0 {
            return S::from_f64(self.target.load());
        }

        let target = self.target.load();
        let value = if steps == 1 {
            // Land exactly on the target on the final step.
            target
        } else {
            let current = self.current.load();
            match self.style {
                SmoothingStyle::Linear(_) => current + self.increment.load(),
                SmoothingStyle::Exponential(_) => {
                    current + (target - current) * self.increment.load()
                }
                SmoothingStyle::None => target,
            }
        };

        self.current.store(value);
        self.steps_left.store(steps - 1, Relaxed);
        S::from_f64(value)
    }

    /// Fill `out` with successive smoothed samples. Equivalent to calling
    /// [`next`](Self::next) for each slot, with a fast path once settled.
    /// Wait-free: audio thread only.
    pub fn next_block(&self, out: &mut [S]) {
        if !self.is_smoothing() {
            let target = S::from_f64(self.target.load());
            out.fill(target);
            return;
        }
        for slot in out {
            *slot = self.next();
        }
    }

    /// Cancel any in-flight ramp, snapping the output to the current target.
    /// Control thread only.
    pub fn snap_to_target(&self) {
        self.current.store(self.target.load());
        self.steps_left.store(0, Relaxed);
    }
}

/// Convert a duration in milliseconds to a (clamped) number of sample steps.
fn duration_to_steps(ms: f64, sample_rate: f64) -> i32 {
    ((ms / 1000.0) * sample_rate).round().max(1.0) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f64 = 1000.0; // 1 step per ms keeps the arithmetic obvious.

    #[test]
    fn none_jumps_immediately() {
        let s = Smoother::<f64>::new(SmoothingStyle::None, 0.0);
        s.set_target(SR, 1.0);
        assert!(!s.is_smoothing());
        assert_eq!(s.next(), 1.0);
    }

    #[test]
    fn linear_ramps_then_lands_exactly() {
        let s = Smoother::<f64>::new(SmoothingStyle::Linear(4.0), 0.0);
        s.set_target(SR, 1.0); // 4 steps of +0.25
        assert!(s.is_smoothing());
        assert!((s.next() - 0.25).abs() < 1e-12);
        assert!((s.next() - 0.50).abs() < 1e-12);
        assert!((s.next() - 0.75).abs() < 1e-12);
        assert_eq!(s.next(), 1.0); // exact landing
        assert!(!s.is_smoothing());
        assert_eq!(s.next(), 1.0); // holds
    }

    #[test]
    fn exponential_approaches_and_terminates() {
        let s = Smoother::<f64>::new(SmoothingStyle::Exponential(2.0), 0.0);
        s.set_target(SR, 1.0);
        let first = s.next();
        assert!(
            first > 0.0 && first < 1.0,
            "monotonic partial step: {first}"
        );
        // Drain the ramp; it must terminate and finish on the target.
        let mut last = first;
        while s.is_smoothing() {
            last = s.next();
        }
        assert_eq!(last, 1.0);
    }

    #[test]
    fn next_block_matches_next() {
        let a = Smoother::<f32>::new(SmoothingStyle::Linear(8.0), 0.0);
        let b = Smoother::<f32>::new(SmoothingStyle::Linear(8.0), 0.0);
        a.set_target(SR, 1.0);
        b.set_target(SR, 1.0);

        let mut block = [0.0_f32; 5];
        a.next_block(&mut block);
        for expected in block {
            assert_eq!(b.next(), expected);
        }
    }

    #[test]
    fn snap_cancels_ramp() {
        let s = Smoother::<f64>::new(SmoothingStyle::Linear(100.0), 0.0);
        s.set_target(SR, 1.0);
        assert!(s.is_smoothing());
        s.snap_to_target();
        assert!(!s.is_smoothing());
        assert_eq!(s.next(), 1.0);
    }
}
