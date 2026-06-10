//! [`Gain`]: a broadband gain, the proof-of-concept implementation of the
//! [`Processor`] / [`Controller`] split.
//!
//! It applies the same gain to every channel of its layout, passing the layout
//! straight through to the output. Layout-agnostic in behaviour but, like every
//! processor, built for one concrete input layout. The gain is a smoothed
//! [`FloatParam`], so changes pushed through [`GainController::update`] ramp in
//! per sample rather than clicking.

use std::sync::Arc;

use crate::common::{
    Buffer, ChannelMask, Controller, FloatParam, Param, Processor, Sample, SmoothingStyle, Spec,
};

/// Configuration for [`Gain`].
///
/// The layout is not configured here — it comes from the [`Spec`]'s input
/// layout, which gain passes straight through to its output.
#[derive(Clone, Debug, Default)]
pub struct GainConfig {
    /// Gain in decibels (0 dB = unity, the default).
    pub gain_db: f64,
}

/// Time over which a gain change ramps to its new value, in milliseconds.
const GAIN_SMOOTHING_MS: f64 = 10.0;

/// Largest finite linear gain a [`FloatParam`] target is clamped to (≈ +60 dB).
/// Bounds the smoother so a wild config can't produce a non-finite ramp.
const MAX_LINEAR_GAIN: f64 = 1_000.0;

/// Parameters shared between a [`Gain`] and its [`GainController`].
///
/// Smoothed in the linear-amplitude domain: the control thread converts dB to a
/// linear target, the audio thread reads the smoothed factor per sample.
struct GainParams<S: Sample> {
    gain: FloatParam<S>,
}

impl<S: Sample> GainParams<S> {
    fn new(gain_db: f64) -> Self {
        Self {
            gain: FloatParam::new(
                db_to_linear(gain_db),
                0.0..=MAX_LINEAR_GAIN,
                SmoothingStyle::Linear(GAIN_SMOOTHING_MS),
            ),
        }
    }
}

/// The control-thread handle for a [`Gain`]. Reconfigures the running processor
/// through the shared parameters.
pub struct GainController<S: Sample> {
    params: Arc<GainParams<S>>,
    sample_rate: f64,
}

impl<S: Sample> Controller for GainController<S> {
    type Config = GainConfig;

    fn update(&mut self, config: &Self::Config) {
        self.params
            .gain
            .set(self.sample_rate, db_to_linear(config.gain_db));
    }

    fn reset(&mut self) {
        self.params.gain.reset();
    }
}

/// A broadband gain (realtime half).
pub struct Gain<S: Sample> {
    spec: Spec,
    params: Arc<GainParams<S>>,
}

impl<S: Sample> Processor for Gain<S> {
    type Sample = S;
    type Config = GainConfig;
    type Controller = GainController<S>;

    const INPUT_LAYOUTS: &'static [ChannelMask] = GAIN_LAYOUTS;

    const OUTPUT_LAYOUTS: &'static [ChannelMask] = GAIN_LAYOUTS;

    fn new(spec: Spec, config: &Self::Config) -> (Self::Controller, Self) {
        debug_assert!(
            Self::INPUT_LAYOUTS.contains(&spec.layout),
            "gain built for unsupported input layout {:?}",
            spec.layout,
        );

        let params = Arc::new(GainParams::new(config.gain_db));
        let controller = GainController {
            params: Arc::clone(&params),
            sample_rate: spec.sample_rate,
        };
        let processor = Gain { spec, params };
        (controller, processor)
    }

    fn process(&mut self, buffer: &mut Buffer<'_, S>) {
        let frames = buffer.frames();
        let channels = buffer.out_channels();
        let gain = &self.params.gain;

        if gain.is_smoothing() {
            // The gain moves within the block: advance the ramp once per frame
            // and apply the same factor across every channel of that frame.
            for i in 0..frames {
                let g = gain.next();
                for ch in 0..channels {
                    let x = buffer.input(ch)[i];
                    buffer.output_mut(ch)[i] = x * g;
                }
            }
        } else {
            // Settled: one constant factor, iterate channel-major for locality.
            let g = gain.next();
            for ch in 0..channels {
                for i in 0..frames {
                    let x = buffer.input(ch)[i];
                    buffer.output_mut(ch)[i] = x * g;
                }
            }
        }
    }

    fn input_spec(&self) -> &Spec {
        &self.spec
    }

    fn output_spec(&self) -> &Spec {
        &self.spec
    }
}

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
