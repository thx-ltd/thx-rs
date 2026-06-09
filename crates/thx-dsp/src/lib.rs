//! # thx-dsp
//!
//! THX's Digital Signal Processing framework.
//!
//! Every DSP block implements one trait, [`Processor`], which captures the
//! behaviour shared across all of them:
//!
//! * Construction [`new`] for a fixed [`Spec`] (sample rate, block size, input
//!   layout).
//! * Processing: [`process`]
//! * Live reconfiguration: [`update`]
//! * Channel topology: [`input_layout`] / [`output_layout`] of an instance, and
//!   the static [`supported_input_layouts`] / [`supported_output_layouts`]
//!   describing the processor type.
//!
//! ## Crate layout
//!
//! * [`common`]: The reusable framework ([`Processor`] trait, [`Sample`],
//!   [`Buffer`], [`ChannelMask`], and test instrumentation).
//! * [`processors`]: Concrete processors built on it
//!
//! The most-used items are re-exported at the crate root.
//!
//! ## Quick start
//!
//! ```
//! use thx_dsp::{Buffer, ChannelMask, Gain, GainConfig, Processor, Spec};
//!
//! // The spec fixes the input layout; gain passes it through to its output.
//! let spec = Spec::new(48_000.0, 512, ChannelMask::MASK_STEREO);
//! let mut gain = Gain::<f32>::new(spec, &GainConfig { gain_db: -6.0 });
//!
//! let input = vec![vec![1.0_f32; 512]; 2];
//! let mut output = vec![vec![0.0_f32; 512]; 2];
//! let in_refs: Vec<&[f32]> = input.iter().map(Vec::as_slice).collect();
//! let mut out_refs: Vec<&mut [f32]> = output.iter_mut().map(Vec::as_mut_slice).collect();
//! let mut buffer = Buffer::new(&in_refs, &mut out_refs, 512);
//! gain.process(&mut buffer);
//!
//! gain.update(&GainConfig { gain_db: 0.0 }); // reconfigure in place
//! ```
//!
//! [`new`]: Processor::new
//! [`process`]: Processor::process
//! [`update`]: Processor::update
//! [`input_layout`]: Processor::input_layout
//! [`output_layout`]: Processor::output_layout
//! [`supported_input_layouts`]: Processor::supported_input_layouts
//! [`supported_output_layouts`]: Processor::supported_output_layouts

pub mod common;
pub mod processors;

pub use common::{Buffer, ChannelMask, Processor, Sample, Spec};
pub use processors::{Gain, GainConfig};
