//! Concrete DSP processors built on the [`crate::common`] framework.
//!
//! Each submodule implements [`Processor`](crate::common::Processor) for one
//! kind of block. [`Gain`] is the reference implementation.

pub mod gain;

pub use gain::{Gain, GainConfig, GainController};
