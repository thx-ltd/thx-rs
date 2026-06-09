//! Instrumentation helpers for testing processors.
//!
//! Gated behind the `testing` feature (always on for this crate's own tests).
//! It provides three things test suites repeatedly need:
//!
//! * **signal generators** — deterministic input ([`sine`], [`dc`], [`impulse`]);
//! * an **offline driver** ([`run_offline`]) that runs a [`Processor`] over a
//!   whole signal in fixed blocks, single-threaded and deterministically, so the
//!   same input always yields the same output;
//! * **analyzers** ([`peak`], [`rms`]) to make assertions about output.

use super::buffer::Buffer;
use super::processor::Processor;
use super::sample::Sample;

/// A constant ("DC") signal of `frames` samples at amplitude `value`.
#[must_use]
pub fn dc<S: Sample>(value: f64, frames: usize) -> Vec<S> {
    vec![S::from_f64(value); frames]
}

/// A unit impulse: `1.0` at sample 0, silence after.
#[must_use]
pub fn impulse<S: Sample>(frames: usize) -> Vec<S> {
    let mut v = vec![S::ZERO; frames];
    if frames > 0 {
        v[0] = S::ONE;
    }
    v
}

/// A sine wave of `frequency` Hz at `sample_rate`, amplitude `amplitude`.
#[must_use]
pub fn sine<S: Sample>(frequency: f64, sample_rate: f64, frames: usize, amplitude: f64) -> Vec<S> {
    (0..frames)
        .map(|i| {
            let phase = core::f64::consts::TAU * frequency * (i as f64) / sample_rate;
            S::from_f64(amplitude * phase.sin())
        })
        .collect()
}

/// Peak absolute value of a buffer.
#[must_use]
pub fn peak<S: Sample>(buffer: &[S]) -> f64 {
    buffer.iter().fold(0.0, |m, &s| m.max(s.to_f64().abs()))
}

/// Root-mean-square level of a buffer.
#[must_use]
pub fn rms<S: Sample>(buffer: &[S]) -> f64 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = buffer.iter().map(|&s| s.to_f64() * s.to_f64()).sum();
    (sum_sq / buffer.len() as f64).sqrt()
}

/// Drive `processor` over a planar `input` signal in fixed-size blocks,
/// returning the planar output (`out_channels` channels).
///
/// Deterministic and single-threaded: the same `input` always yields the same
/// output, which is what makes processor behaviour unit-testable. `block` is the
/// per-call frame count; it must not exceed the processor's `max_frames`.
#[must_use]
pub fn run_offline<P: Processor>(
    processor: &mut P,
    input: &[Vec<P::Sample>],
    out_channels: usize,
    block: usize,
) -> Vec<Vec<P::Sample>> {
    let block = block.max(1);
    let in_channels = input.len();
    let total = input.iter().map(Vec::len).min().unwrap_or(0);

    let mut output = vec![vec![<P::Sample as Sample>::ZERO; total]; out_channels];

    let mut pos = 0;
    while pos < total {
        let n = block.min(total - pos);

        let in_refs: Vec<&[P::Sample]> = (0..in_channels).map(|c| &input[c][pos..pos + n]).collect();
        let mut out_windows: Vec<&mut [P::Sample]> =
            output.iter_mut().map(|c| &mut c[pos..pos + n]).collect();

        let mut buffer_view = Buffer::new(&in_refs, &mut out_windows, n);
        processor.process(&mut buffer_view);

        pos += n;
    }

    output
}
