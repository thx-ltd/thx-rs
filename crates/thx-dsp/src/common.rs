//! Shared framework code that every processor builds on: the [`Processor`]
//! trait and its [`Spec`], the scalar [`Sample`] abstraction, planar
//! [`Buffer`]s, [`ChannelMask`] topology, and (feature-gated) [`testing`]
//! instrumentation.

pub mod buffer;
pub mod channel_mask;
pub mod processor;
pub mod sample;

#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use buffer::Buffer;
pub use channel_mask::ChannelMask;
pub use processor::{Processor, Spec};
pub use sample::Sample;
