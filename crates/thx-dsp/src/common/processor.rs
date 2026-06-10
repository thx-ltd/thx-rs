//! The [`Processor`] / [`Controller`] pair: the shared contract for every DSP
//! block, split along the realtime boundary.
//!
//! Constructing a block with [`Processor::new`] yields **two halves**:
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
//! layout) plus a processor-specific [`Config`](Processor::Config), which
//! together fix the output layout for the processor's lifetime.

use super::buffer::Buffer;
use super::channel_mask::ChannelMask;
use super::sample::Sample;

/// Audio engine context fixed at construction time.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    /// Sample rate in Hz.
    pub sample_rate: f64,
    /// The largest block size (`frames`) that will ever be passed to
    /// [`Processor::process`]. Processors size their scratch buffers from this.
    pub max_frames: usize,
    /// The speaker layout of the input buffers fed to [`Processor::process`].
    /// Together with the processor's [`Config`](Processor::Config) it fixes the
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

/// The realtime half of a DSP block: lives on the audio thread and does nothing
/// but [`process`](Self::process).
///
/// Obtain one (paired with its [`Controller`]) from [`new`](Self::new).
pub trait Processor: Send + Sized {
    /// The scalar sample type this processor operates on.
    type Sample: Sample;

    /// User-facing configuration.
    type Config: Clone + Send;

    /// The off-thread handle used to reconfigure this processor live.
    type Controller: Controller<Config = Self::Config>;

    /// The input layouts this processor supports.
    const INPUT_LAYOUTS: &'static [ChannelMask];

    /// The output layouts this processor type can produce across all of its
    /// configurations.
    const OUTPUT_LAYOUTS: &'static [ChannelMask];

    /// Construct a block for `spec`'s input layout, configured by `config`,
    /// returning its [`Controller`] (for the control thread) and its realtime
    /// `Processor` (to move to the audio thread). This fixes the output layout
    /// for the lifetime of the block.
    ///
    /// Heavy: may allocate and otherwise do real work. Call off the audio thread.
    ///
    /// `spec.layout` is expected to be one of [`INPUT_LAYOUTS`](Self::INPUT_LAYOUTS),
    /// and the resulting output layout one of [`OUTPUT_LAYOUTS`](Self::OUTPUT_LAYOUTS).
    fn new(spec: Spec, config: &Self::Config) -> (Self::Controller, Self);

    /// Process one [`Buffer`] in place.
    ///
    /// **Realtime-safe:** no heap allocation, no locking, no blocking, no
    /// unbounded work. Runs on the caller's audio thread.
    fn process(&mut self, buffer: &mut Buffer<'_, Self::Sample>);

    /// Current input spec of the processor.
    fn input_spec(&self) -> &Spec;

    /// Current output spec of the processor.
    fn output_spec(&self) -> &Spec;

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
    /// User-facing configuration; matches [`Processor::Config`] of the paired
    /// processor.
    type Config: Clone + Send;

    /// Reconfigure from a new `config`. The change is published to the running
    /// processor through the shared parameters and (for smoothed parameters)
    /// ramps in rather than jumping. Heavy: control thread only.
    fn update(&mut self, config: &Self::Config);

    /// Cancel any in-flight parameter smoothing, snapping values to their current
    /// targets. Heavy: control thread only.
    fn reset(&mut self);
}
