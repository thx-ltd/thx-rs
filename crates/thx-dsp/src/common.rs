//! Shared framework code that every processor builds on: the [`Processor`] /
//! [`Controller`] pair and their [`Spec`], the scalar [`Sample`] abstraction,
//! planar [`Buffer`]s, [`ChannelMask`] topology, and typed smoothing [`params`].

pub mod buffer;
pub mod channel_mask;
pub mod params;
pub mod processor;
pub mod sample;

pub use buffer::Buffer;
pub use channel_mask::ChannelMask;
pub use params::{BoolParam, FloatParam, Param, Smoother, SmoothingStyle};
pub use processor::{Controller, Processor, Spec};
pub use sample::Sample;
