//! The [`Sample`] abstraction over the scalar audio sample type.
//!
//! Processors are generic over `S: Sample` so the same DSP code can run in
//! `f32` or `f64` without duplication.

use core::ops::{Add, Mul, Sub};

/// A scalar audio sample.
///
/// Implemented for [`f32`] and [`f64`].
pub trait Sample:
    Copy
    + Send
    + Sync
    + 'static
    + core::fmt::Debug
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
{
    const ZERO: Self;
    const ONE: Self;

    /// Convert from an `f32`.
    fn from_f32(value: f32) -> Self;
    /// Convert from an `f64`.
    fn from_f64(value: f64) -> Self;
    /// Convert to an `f32`.
    fn to_f32(self) -> f32;
    /// Convert to an `f64`.
    fn to_f64(self) -> f64;
}

impl Sample for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;

    fn from_f32(value: f32) -> Self {
        value
    }

    fn from_f64(value: f64) -> Self {
        value as f32
    }

    fn to_f32(self) -> f32 {
        self
    }

    fn to_f64(self) -> f64 {
        self as f64
    }
}

impl Sample for f64 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;

    fn from_f32(value: f32) -> Self {
        value as f64
    }

    fn from_f64(value: f64) -> Self {
        value
    }

    fn to_f32(self) -> f32 {
        self as f32
    }

    fn to_f64(self) -> f64 {
        self
    }
}
