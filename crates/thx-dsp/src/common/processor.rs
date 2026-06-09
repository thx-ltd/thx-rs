//! The [`Processor`] trait: the shared contract for every DSP block.
//!
//! A processor is built for a fixed [`Spec`] (sample rate, max block size, and
//! input layout) plus a processor-specific [`Config`](Processor::Config). From
//! those it fixes its [`output_layout`](Processor::output_layout). After that it
//! just [`process`](Processor::process)es buffers in place, can be reconfigured
//! with [`update`](Processor::update), and cleared with
//! [`reset`](Processor::reset).
//!
//! [`Config`]: Processor::Config

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
    /// [`output_layout`](Processor::output_layout) at construction.
    pub input_layout: ChannelMask,
}

impl Spec {
    /// Convenience constructor.
    #[must_use]
    pub fn new(sample_rate: f64, max_frames: usize, input_layout: ChannelMask) -> Self {
        Self {
            sample_rate,
            max_frames,
            input_layout,
        }
    }
}

/// The shared behaviour of every DSP processor.
pub trait Processor: Send + Sized {
    /// The scalar sample type this processor operates on.
    type Sample: Sample;

    /// User-facing configuration.
    type Config: Clone + Send;

    /// Construct a processor for `spec`'s input layout, configured by `config`.
    /// This fixes the [`output_layout`](Self::output_layout) for the lifetime of
    /// the processor.
    ///
    /// `spec.input_layout` is expected to be one of
    /// [`supported_input_layouts`](Self::supported_input_layouts), and the
    /// resulting output layout one of
    /// [`supported_output_layouts`](Self::supported_output_layouts).
    fn new(spec: Spec, config: &Self::Config) -> Self;

    /// Reconfigure in place from a new `config`.
    fn update(&mut self, config: &Self::Config);

    /// Process one [`Buffer`] in place.
    fn process(&mut self, buffer: &mut Buffer<'_, Self::Sample>);

    /// Clear internal state (filter memory, smoothing, etc.) as if freshly
    /// started, e.g. after a transport stop or stream discontinuity.
    fn reset(&mut self);

    /// The spec this processor was built with.
    fn spec(&self) -> &Spec;

    /// The input speaker layout this processor consumes, taken from its
    /// [`Spec`]. Always one of
    /// [`supported_input_layouts`](Self::supported_input_layouts).
    fn input_layout(&self) -> ChannelMask {
        self.spec().input_layout
    }

    /// The output speaker layout this processor produces, fixed at construction
    /// from the [`Spec`]'s input layout and the [`Config`](Self::Config). For a
    /// layout-preserving block (e.g. gain) this equals
    /// [`input_layout`](Self::input_layout); for a format-changing block (e.g.
    /// an upmix) the config decides it (stereo in, 5.1 out). Always one of
    /// [`supported_output_layouts`](Self::supported_output_layouts).
    ///
    /// There is no output layout without a configuration: the only way to obtain
    /// one is to call this on a processor already built via [`new`](Self::new),
    /// which is what fixes it.
    fn output_layout(&self) -> ChannelMask;

    /// The input layouts this processor type can be built for. Static: it
    /// describes the processor *type*, independent of any instance or config —
    /// useful for negotiating a layout before construction.
    fn supported_input_layouts() -> &'static [ChannelMask];

    /// The output layouts this processor type can produce across all of its
    /// configurations. Static: it describes the processor *type*, independent of
    /// any instance or config.
    fn supported_output_layouts() -> &'static [ChannelMask];

    /// Processing latency in frames (0 for a zero-delay block like gain). Useful
    /// for delay compensation and for asserting timing in tests.
    fn latency_frames(&self) -> usize {
        0
    }
}
