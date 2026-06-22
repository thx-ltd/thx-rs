//! [`Spec`]: the static description of an audio stream at a block boundary.

use serde::{Deserialize, Serialize};

use crate::channel_mask::ChannelMask;

/// Signal specification at a block boundary: sample rate and speaker layout.
///
/// A spec describes the *stream*, so it can differ from boundary to boundary —
/// a resampler changes `sample_rate`, an upmixer changes `layout`. The
/// engine-wide maximum block size is deliberately **not** part of the spec: it
/// is fixed once for a whole engine and passed separately at construction time
/// (see [`DspBlock::new`](crate::block::DspBlock::new)).
///
/// A spec is **immutable for the lifetime of a processor**: blocks are built
/// for one concrete spec, which lets `process` pre-allocate everything and stay
/// realtime-safe. To change sample rate or layout, build a new processor for
/// the new spec and swap it in on the audio thread; do not mutate a running one.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spec {
    /// Sample rate in Hz.
    pub sample_rate: f64,
    /// Speaker layout.
    pub layout: ChannelMask,
}

impl Spec {
    /// Convenience constructor.
    pub fn new(sample_rate: f64, layout: ChannelMask) -> Self {
        Self {
            sample_rate,
            layout,
        }
    }

    /// Number of channels implied by [`layout`](Self::layout).
    pub const fn channels(&self) -> usize {
        self.layout.channel_count()
    }

    /// A copy of this spec with a different layout (same sample rate).
    pub fn with_layout(&self, layout: ChannelMask) -> Self {
        Self { layout, ..*self }
    }

    /// A copy of this spec with a different sample rate (same layout).
    pub fn with_sample_rate(&self, sample_rate: f64) -> Self {
        Self {
            sample_rate,
            ..*self
        }
    }
}
