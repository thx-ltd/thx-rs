//! Signal helpers for `thx-dsp`'s integration tests: deterministic generators
//! and analyzers.
//!
//! Included by a test file with `#[path = "utils/signal.rs"] mod signal;`.
//! Not every test uses every helper, so unused-code warnings are silenced here.
#![allow(dead_code)]

use thx_dsp::Sample;

/// A constant ("DC") signal of `frames` samples at amplitude `value`.
pub fn dc<S: Sample>(value: f64, frames: usize) -> Vec<S> {
    vec![S::from_f64(value); frames]
}

/// A unit impulse: `1.0` at sample 0, silence after.
pub fn impulse<S: Sample>(frames: usize) -> Vec<S> {
    let mut v = vec![S::ZERO; frames];
    if frames > 0 {
        v[0] = S::ONE;
    }
    v
}

/// A sine wave of `frequency` Hz at `sample_rate`, amplitude `amplitude`.
pub fn sine<S: Sample>(frequency: f64, sample_rate: f64, frames: usize, amplitude: f64) -> Vec<S> {
    (0..frames)
        .map(|i| {
            let phase = core::f64::consts::TAU * frequency * (i as f64) / sample_rate;
            S::from_f64(amplitude * phase.sin())
        })
        .collect()
}

/// Peak absolute value of a buffer.
pub fn peak<S: Sample>(buffer: &[S]) -> f64 {
    buffer.iter().fold(0.0, |m, &s| m.max(s.to_f64().abs()))
}

/// Root-mean-square level of a buffer.
pub fn rms<S: Sample>(buffer: &[S]) -> f64 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = buffer.iter().map(|&s| s.to_f64() * s.to_f64()).sum();
    (sum_sq / buffer.len() as f64).sqrt()
}
