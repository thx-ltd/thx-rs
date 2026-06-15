//! [`Gain`]: a broadband gain — the reference [`Block`] implementation.

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use super::{Block, BlockDescription, BlockSignal, Error, Result};
use crate::channel_mask::ChannelMask;
use crate::sample::Sample;
use crate::smooth::Smooth;
use crate::spec::Spec;

/// Configuration for [`Gain`].
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GainConfig {
    /// Gain in decibels (0 dB = unity, the default), smoothed. Clamped to
    /// [-100 dB, +30 dB].
    pub gain_db: Smooth<f64>,
}

/// Broadband gain applied uniformly across all channels.
pub struct Gain<S: Sample> {
    config: GainConfig,
    _marker: PhantomData<S>,
}

impl<S: Sample> Block<S> for Gain<S> {
    type Config = GainConfig;

    fn new(spec: &Spec, _max_frames: usize, config: &GainConfig) -> Result<Self> {
        if !LAYOUTS.contains(&spec.layout) {
            return Err(Error::UnsupportedLayout(spec.layout));
        }
        let mut config = Self::validate(config)?;
        config.gain_db.prepare(spec.sample_rate);
        Ok(Self {
            config,
            _marker: PhantomData,
        })
    }

    fn description() -> &'static BlockDescription {
        &DESCRIPTOR
    }

    fn validate(config: &GainConfig) -> Result<GainConfig> {
        let clamped = config.gain_db.target().clamp(-100.0, 30.0);
        Ok(GainConfig {
            gain_db: config.gain_db.with_target(clamped),
        })
    }

    fn configure(&mut self, config: &GainConfig) {
        self.config.gain_db.set_target(config.gain_db.target());
    }

    fn process(&mut self, input: &BlockSignal<S>, output: &mut BlockSignal<S>) {
        output.spec = input.spec;
        let frames = input.buffer.frames();
        let channels = output.buffer.channels();
        for k in 0..frames {
            // One shared gain per frame, applied across every channel.
            let gain = S::from_f64(db_to_linear(self.config.gain_db.advance()));
            for ch in 0..channels {
                output.buffer.channel_mut(ch)[k] = input.buffer.channel(ch)[k] * gain;
            }
        }
    }

    fn reset(&mut self) {
        self.config.gain_db.settle();
    }
}

static DESCRIPTOR: BlockDescription = BlockDescription {
    name: "gain",
    description: "Broadband gain applied uniformly across all channels.",
    input_layouts: LAYOUTS,
    output_layouts: LAYOUTS,
};

const LAYOUTS: &[ChannelMask] = &[
    ChannelMask::MASK_MONO,
    ChannelMask::MASK_STEREO,
    ChannelMask::MASK_QUAD,
    ChannelMask::MASK_5_1,
    ChannelMask::MASK_7_1,
    ChannelMask::MASK_7_1_4,
];

/// Convert decibels to a linear amplitude factor.
fn db_to_linear(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0)
}
