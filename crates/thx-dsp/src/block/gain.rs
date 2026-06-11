//! [`Gain`]: a broadband gain, the reference implementation of the
//! [`Block`] / [`Processor`] / [`Controller`] split.
//!
//! It applies the same gain to every channel of its layout, passing the layout
//! straight through to the output. Layout-agnostic in behaviour but, like every
//! block, built for one concrete input layout. The gain is a smoothed
//! [`FloatParam`], so changes pushed through [`GainController::update`] ramp in
//! per sample rather than clicking.

use std::sync::Arc;

use super::{Block, Controller, Error, Processor, Result, Spec};
use crate::common::{Buffer, ChannelMask, FloatParam, Param, Sample, SmoothingStyle};

/// Configuration for [`Gain`].
#[derive(Clone, Debug, Default)]
pub struct GainConfig {
    /// Gain in decibels (0 dB = unity, the default).
    pub gain_db: f64,
}

/// Internal parameters of a [`Gain`], shared between the processor and its
/// controller.
struct GainParams<S: Sample> {
    gain_lin: FloatParam<S>,
}

impl<S: Sample> GainParams<S> {
    /// Build parameters resting at `config` — settled, so there is no ramp on the
    /// first block.
    fn new(config: &GainConfig) -> Self {
        Self {
            gain_lin: FloatParam::new(
                db_to_linear(config.gain_db),
                0.0..=MAX_LINEAR_GAIN,
                SmoothingStyle::Linear(GAIN_SMOOTHING_MS),
            ),
        }
    }

    /// Retarget the parameters from `config`, ramping at `spec`'s sample rate.
    fn set(&self, spec: &Spec, config: &GainConfig) {
        self.gain_lin
            .set(spec.sample_rate, db_to_linear(config.gain_db));
    }
}

/// The control-thread handle for a [`Gain`].
pub struct GainController<S: Sample> {
    params: Arc<GainParams<S>>,
    spec: Spec,
}

impl<S: Sample> Controller for GainController<S> {
    type Config = GainConfig;

    fn update(&mut self, spec: &Spec, config: &Self::Config) -> Result<Self::Config> {
        if !GAIN_LAYOUTS.contains(&spec.layout) {
            return Err(Error::UnsupportedLayout(spec.layout));
        }
        self.spec = *spec;
        self.params.set(spec, config);
        Ok(config.clone())
    }

    fn reset(&mut self) {
        self.params.gain_lin.reset();
    }

    fn input_spec(&self) -> &Spec {
        &self.spec
    }

    fn output_spec(&self) -> &Spec {
        &self.spec
    }
}

/// The audio-thread processor for a [`Gain`].
pub struct Gain<S: Sample> {
    params: Arc<GainParams<S>>,
}

impl<S: Sample> Processor for Gain<S> {
    type Sample = S;

    fn process(&mut self, buffer: &mut Buffer<'_, S>) {
        let frames = buffer.frames();
        let channels = buffer.out_channels();

        // Reading the gain advances its smoothing, so take one value per frame
        // and apply it across that frame's channels. No smoothing state to manage.
        for i in 0..frames {
            let g = self.params.gain_lin.next();
            for ch in 0..channels {
                let x = buffer.input(ch)[i];
                buffer.output_mut(ch)[i] = x * g;
            }
        }
    }
}

impl<S: Sample> Block for Gain<S> {
    type Sample = S;
    type Config = GainConfig;
    type Processor = Gain<S>;
    type Controller = GainController<S>;

    const INPUT_LAYOUTS: &'static [ChannelMask] = GAIN_LAYOUTS;
    const OUTPUT_LAYOUTS: &'static [ChannelMask] = GAIN_LAYOUTS;
    const NAME: &'static str = "Gain";
    const DESCRIPTION: &'static str =
        "Broadband gain applied uniformly across all channels.";

    fn new(spec: &Spec, config: &Self::Config) -> (Gain<S>, GainController<S>) {
        debug_assert!(
            GAIN_LAYOUTS.contains(&spec.layout),
            "gain built for unsupported input layout {:?}",
            spec.layout,
        );

        let params = Arc::new(GainParams::new(config));
        let controller = GainController {
            params: Arc::clone(&params),
            spec: *spec,
        };
        let processor = Gain { params };
        (processor, controller)
    }
}

const GAIN_SMOOTHING_MS: f64 = 10.0;

const MAX_LINEAR_GAIN: f64 = 1_000.0;

const GAIN_LAYOUTS: &[ChannelMask] = &[
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
