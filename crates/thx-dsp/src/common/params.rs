//! Typed, thread-safe parameters with built-in smoothing.
//!
//! A processor exposes its tweakable values as a struct of parameters shared
//! (via `Arc`) between its [`Controller`](crate::common::Controller) and its
//! realtime [`Processor`](crate::common::Processor) half. Each parameter is the
//! crossing point of the realtime boundary:
//!
//! * the **control thread** writes a target — [`FloatParam::set`] /
//!   [`BoolParam::set`] — doing any heavy work (range checks, unit conversion);
//! * the **audio thread** reads the value — [`FloatParam::next`] /
//!   [`BoolParam::get`] — wait-free, with no allocation or locking.
//!
//! All parameter state is atomic, so the parameter itself *is* the lock-free
//! channel: storing a new target publishes it to the audio thread. Continuous
//! parameters ([`FloatParam`]) additionally carry a [`Smoother`] so changes ramp
//! per sample instead of clicking. Discrete parameters ([`BoolParam`]) read
//! instantaneously.
//!
//! This per-parameter model fits *independent* scalar controls. When several
//! derived values must change together atomically (e.g. a set of biquad
//! coefficients computed as a unit), publish them as one snapshot through a
//! lock-free buffer instead, rather than as separate parameters.
//!
//! # Parameter types
//!
//! | Type | Value | Smoothed |
//! | --- | --- | --- |
//! | [`FloatParam`] | `f64` | yes ([`Smoother`]) |
//! | [`BoolParam`] | `bool` | no |
//!
//! [`Smoother`]: smoother::Smoother

mod boolean;
mod float;
mod smoother;

pub use boolean::BoolParam;
pub use float::FloatParam;
pub use smoother::{Smoother, SmoothingStyle};

/// The behaviour shared by every parameter type, regardless of its value type.
///
/// The value-*setting* methods are deliberately left to the concrete types: a
/// [`FloatParam`] needs the sample rate (to size its ramp) while a [`BoolParam`]
/// does not, so a single uniform setter would be a poor fit. This trait captures
/// only what *is* uniform: the value type, its default, the current target, and
/// resetting in-flight smoothing.
pub trait Param: Send + Sync {
    /// The plain value type the control thread sets and reads.
    type Plain: Copy;

    /// The configured default (already clamped to any valid range).
    fn default_value(&self) -> Self::Plain;

    /// The current target value (what a smoothed param is ramping toward).
    fn value(&self) -> Self::Plain;

    /// Cancel any in-flight smoothing, snapping to the current target. Control
    /// thread only. A no-op for discrete params.
    fn reset(&self);
}
