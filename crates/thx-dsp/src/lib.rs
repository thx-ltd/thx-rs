//! # thx-dsp
//!
//! THX's Digital Signal Processing framework: composable DSP blocks and the
//! graph that connects them (Soon).

pub mod block;
pub mod buffer;
pub mod channel_mask;
pub mod sample;
pub mod smooth;
pub mod spec;

pub use block::{
    Block, BlockController, BlockDescription, BlockProcessor, BlockSignal, Config, Error, Result,
};
pub use buffer::Buffer;
pub use channel_mask::ChannelMask;
pub use sample::Sample;
pub use smooth::{Param, Smooth};
pub use spec::Spec;
