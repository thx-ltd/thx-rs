//! The [`Block`] trait and its two halves: [`Processor`] (audio thread) and
//! [`Controller`] (control thread).
//!
//! [`Block::new`] yields two halves:
//!
//! * the [`Processor`] — moved to the audio thread; its only live operation is
//!   [`process`](Processor::process), which must be realtime-safe (no heap
//!   allocation, no locking, no blocking);
//! * the [`Controller`] — kept on a control/UI thread; its operations
//!   ([`update`](Controller::update), [`reset`](Controller::reset)) may do heavy
//!   work and must never run on the audio thread.
//!
//! Splitting the API this way makes the contract structural rather than a
//! comment: the audio thread is handed a value whose surface is just `process`,
//! so there is no `update`/`reset` to call there by accident. The two halves
//! communicate through the shared, lock-free [parameters](crate::common::params)
//! handed out at construction — the controller sets targets, the processor reads
//! smoothed values, with no channel to wire up by hand.
//!
//! A block is built for a fixed [`Spec`] (sample rate, max block size, input
//! layout) plus a block-specific [`Config`](Block::Config), which together fix
//! the output layout for the processor's lifetime.

pub mod gain;

pub use gain::{Gain, GainConfig, GainController};

use crate::common::buffer::Buffer;
use crate::common::channel_mask::ChannelMask;
use crate::common::sample::Sample;

/// The error type for DSP block operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The [`Spec`] passed to [`Block::new`] or [`Controller::update`] uses a
    /// layout not listed in [`Block::INPUT_LAYOUTS`].
    #[error("unsupported input layout: {0:?}")]
    UnsupportedLayout(ChannelMask),
}

/// A [`Result`](std::result::Result) with the module's [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;

/// Signal specification for a block
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    /// Sample rate in Hz.
    pub sample_rate: f64,
    /// The largest block size (`frames`) that will ever be passed to
    /// [`Processor::process`]. Processors size their scratch buffers from this.
    pub max_frames: usize,
    /// The speaker layout of the input buffers fed to [`Processor::process`].
    /// Together with the block's [`Config`](Block::Config) it fixes the
    /// [`output_spec`](Processor::output_spec) at construction.
    pub layout: ChannelMask,
}

impl Spec {
    /// Convenience constructor.
    pub fn new(sample_rate: f64, max_frames: usize, layout: ChannelMask) -> Self {
        Self {
            sample_rate,
            max_frames,
            layout,
        }
    }
}

/// A DSP block type: static metadata, a factory ([`new`](Self::new)), and the
/// audio-thread half ([`Processor`]) tied to its control-thread half
/// ([`Controller`]).
///
/// Implement this on the type that will live on the audio thread (i.e., the
/// `Processor` itself). Call [`new`](Self::new) to split it into its two halves.
pub trait Block: Sized {
    /// The scalar sample type this block operates on.
    type Sample: Sample;

    /// User-facing configuration.
    type Config: Clone + Send;

    /// The off-thread handle used to reconfigure this block live.
    type Processor: Processor<Sample = Self::Sample>;

    /// The off-thread handle used to reconfigure this block live.
    type Controller: Controller<Config = Self::Config>;

    /// Human-readable name for this block type.
    const NAME: &'static str;

    /// Static metadata for this block type.
    const DESCRIPTION: &'static str;

    /// The input layouts this block supports.
    const INPUT_LAYOUTS: &'static [ChannelMask];

    /// The output layouts this block type can produce across all of its
    /// configurations.
    const OUTPUT_LAYOUTS: &'static [ChannelMask];

    /// Construct a block with an initial `spec` and `config`.
    /// Returns its [`Controller`] (for the control thread) and its realtime
    /// [`Processor`] half (to move to the audio thread). This fixes the output
    /// layout for the lifetime of the block.
    ///
    /// `spec.layout` is expected to be one of [`INPUT_LAYOUTS`](Self::INPUT_LAYOUTS),
    /// and the resulting output layout one of [`OUTPUT_LAYOUTS`](Self::OUTPUT_LAYOUTS).
    fn new(spec: &Spec, config: &Self::Config) -> (Self::Processor, Self::Controller);
}

/// The realtime half of a DSP block: lives on the audio thread and does nothing
/// but [`process`](Self::process).
///
/// Obtain one (paired with its [`Controller`]) from [`Block::new`].
pub trait Processor: Send {
    /// The scalar sample type this processor operates on.
    type Sample: Sample;

    /// Process one [`Buffer`] with a given [`Spec`] in place.
    ///
    /// **Realtime-safe:** no heap allocation, no locking, no blocking, no
    /// unbounded work. Runs on the caller's audio thread.
    fn process(&mut self, spec: &mut Spec, buffer: &mut Buffer<'_, Self::Sample>);

    /// Processing latency in frames (0 for a zero-delay block like gain). Useful
    /// for delay compensation and for asserting timing in tests.
    fn latency_frames(&self) -> usize {
        0
    }
}

/// The control half of a DSP block: the off-thread handle that reconfigures the
/// running [`Processor`] through their shared, lock-free parameters.
///
/// Every method here may do heavy work (allocation, recomputation) and must
/// **never** be called from the audio thread.
pub trait Controller: Send {
    /// User-facing configuration; matches [`Block::Config`] of the paired block.
    type Config: Clone + Send;

    /// Reconfigure from a new `spec` and `config`.
    ///
    /// Returns `Ok(effective_config)` — the configuration that was actually
    /// applied (values may be clamped or adjusted). Returns `Err` if the
    /// reconfiguration is invalid, e.g. an unsupported layout.
    fn update(&mut self, config: &Self::Config) -> Result<Self::Config>;

    /// Cancel any in-flight parameter smoothing, snapping values to their current
    /// targets. Heavy: control thread only.
    fn reset(&mut self);

    /// Current input spec of the processor.
    fn input_spec(&self) -> &Spec;

    /// Current output spec of the processor.
    fn output_spec(&self) -> &Spec;
}
