//! # thx-dsp
//!
//! THX's Digital Signal Processing framework.
//!
//! Every DSP block is built with [`Block::new`], which splits it along the
//! realtime boundary into two halves:
//!
//! * a realtime [`Processor`] (moved to the audio thread) whose only live
//!   operation is [`process`] — and which must be realtime-safe;
//! * a [`Controller`] (kept on a control thread) whose [`update`] / [`reset`]
//!   may do heavy work and must never touch the audio thread.
//!
//! The two halves share lock-free, smoothing [`params`]: the controller sets
//! targets, the processor reads smoothed values, with no channel to wire by hand.
//!
//! A block is built for a fixed [`Spec`] (sample rate, block size, input layout)
//! plus a block-specific [`Config`](Block::Config); the static
//! [`INPUT_LAYOUTS`] / [`OUTPUT_LAYOUTS`] describe the block type.
//!
//! ## Crate layout
//!
//! * [`common`]: The reusable framework ([`Sample`], [`Buffer`], [`ChannelMask`],
//!   and smoothing [`params`]).
//! * [`block`]: The [`Block`]/[`Processor`]/[`Controller`] traits and concrete
//!   blocks built on them.
//!
//! The most-used items are re-exported at the crate root.
//!
//! ## Quick start
//!
//! ```
//! use thx_dsp::{Block, Buffer, ChannelMask, Controller, Gain, GainConfig, Processor, Spec};
//!
//! // The spec fixes the input layout; gain passes it through to its output.
//! // `new` returns the realtime processor and the control-thread handle.
//! let spec = Spec::new(48_000.0, 512, ChannelMask::MASK_STEREO);
//! let (mut gain, mut controller) = Gain::<f32>::new(&spec, &GainConfig { gain_db: -6.0 });
//!
//! let input = vec![vec![1.0_f32; 512]; 2];
//! let mut output = vec![vec![0.0_f32; 512]; 2];
//! let in_refs: Vec<&[f32]> = input.iter().map(Vec::as_slice).collect();
//! let mut out_refs: Vec<&mut [f32]> = output.iter_mut().map(Vec::as_mut_slice).collect();
//! let mut buffer = Buffer::new(&in_refs, &mut out_refs, 512);
//!
//! gain.process(&mut buffer);                                        // audio thread
//! controller.update(&spec, &GainConfig { gain_db: 0.0 }).unwrap(); // control thread: ramps in
//! ```
//!
//! [`process`]: Processor::process
//! [`update`]: Controller::update
//! [`reset`]: Controller::reset
//! [`INPUT_LAYOUTS`]: Block::INPUT_LAYOUTS
//! [`OUTPUT_LAYOUTS`]: Block::OUTPUT_LAYOUTS

pub mod block;
pub mod common;

pub use block::{
    Block, Controller, Error, Gain, GainConfig, GainController, Processor, Result, Spec,
};
pub use common::params;
pub use common::{
    BoolParam, Buffer, ChannelMask, FloatParam, Param, Sample, Smoother, SmoothingStyle,
};
