//! The DSP block model: write one `Config` struct and one [`Block`], get a
//! realtime-correct, lock-free DSP unit.
//!
//! # Writing a block
//!
//! A block author writes pure DSP: a serde [`Config`] (its fields are the
//! parameters, smoothed via [`Smooth`](crate::Smooth) or instant), one state
//! struct, and one `impl Block`. [`Block::spawn`] then splits it into a
//! [`BlockProcessor`] (audio thread) and a [`BlockController`] (control thread)
//! that exchange config snapshots without locking.
//!
//! Bypass is free: every block gets a click-free passthrough⇄processed crossfade
//! through [`BlockController::enable`], with no author code.
//!
//! The [gain block](gain) is the reference implementation.

pub mod gain;
pub mod runtime;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::channel_mask::ChannelMask;
use crate::sample::Sample;
use crate::spec::Spec;

pub use runtime::{BlockController, BlockProcessor};

use crate::buffer::Buffer;

/// The error type for block operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A config value was outside an acceptable range and could not be normalized.
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    /// The block does not support the requested channel layout.
    #[error("unsupported layout: {0:?}")]
    UnsupportedLayout(ChannelMask),
}

/// A [`Result`](std::result::Result) with the module's [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;

/// A block configuration: a plain serde struct, so every config serializes for
/// free. Blanket-implemented; never implement manually.
pub trait Config: Clone + Send + Serialize + DeserializeOwned {}

impl<T: Clone + Send + Serialize + DeserializeOwned> Config for T {}

/// Static metadata describing a block type, for graph queries and UI layers.
#[derive(Debug, Clone, Serialize)]
pub struct BlockDescription {
    /// Unique, human-readable type name.
    pub name: &'static str,
    /// One-line description.
    pub description: &'static str,
    /// The input layouts this block type accepts.
    pub input_layouts: &'static [ChannelMask],
    /// The output layouts this block type can produce across all its configs.
    pub output_layouts: &'static [ChannelMask],
}

/// One signal at a block boundary: its [`Spec`] travels with its [`Buffer`], so a
/// block announces the layout/sample-rate it actually produced to the next block.
pub struct BlockSignal<S: Sample> {
    /// Specification of `buffer` (sample rate + layout).
    pub spec: Spec,
    /// The audio samples.
    pub buffer: Buffer<S>,
}

/// A DSP block: the one trait an author implements. See the [module docs](self)
/// for the model and [`gain`] for the reference.
///
/// The implementing type *is* the audio-thread state, built for one [`Spec`] and
/// never migrated between specs (build a new one and swap instead).
pub trait Block<S: Sample>: Send + Sized {
    /// High-level parameters controlling the processing.
    type Config: Config + Default;

    /// Build the block for `spec`, sized for at most `max_frames` per call.
    /// Control thread; may allocate.
    fn new(spec: &Spec, max_frames: usize, config: &Self::Config) -> Result<Self>;

    /// Static metadata for this block type.
    fn description() -> &'static BlockDescription;

    /// Validate and normalize a config (clamp ranges, reject nonsense), returning
    /// the *effective* config that will be applied. Control thread.
    fn validate(config: &Self::Config) -> Result<Self::Config> {
        Ok(config.clone())
    }

    /// The output spec for a given input spec and config. Defaults to the input
    /// spec (layout-preserving blocks); override for resamplers/up-down-mixers.
    fn output_spec(input: &Spec, _config: &Self::Config) -> Spec {
        *input
    }

    /// Apply a new (already validated) config. Audio thread; must be RT-safe.
    fn configure(&mut self, config: &Self::Config);

    /// Process one block of audio. Audio thread.
    fn process(&mut self, input: &BlockSignal<S>, output: &mut BlockSignal<S>);

    /// Return all internal state to silence. Audio thread.
    fn reset(&mut self);

    /// Split this block into its audio-thread processor and control-thread
    /// controller. Framework-provided; do not override.
    fn spawn(
        spec: &Spec,
        max_frames: usize,
        config: &Self::Config,
    ) -> Result<(BlockProcessor<S, Self>, BlockController<S, Self>)> {
        runtime::spawn::<S, Self>(spec, max_frames, config)
    }
}
