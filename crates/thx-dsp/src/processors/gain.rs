//! [`Gain`]: a broadband gain processor, the proof-of-concept implementation of
//! [`Processor`].
//!
//! It applies the same gain to every channel of its layout, passing the layout
//! straight through to the output. Layout-agnostic in behaviour but, like every
//! processor, built for one concrete input layout.

use std::marker::PhantomData;

use crate::common::{Buffer, ChannelMask, Processor, Sample, Spec};

/// Convert decibels to a linear amplitude factor.
#[inline]
#[must_use]
fn db_to_linear(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0)
}

/// Gain is layout-agnostic: it applies the same gain to every channel and
/// passes the layout through unchanged. These are the standard layouts it
/// accepts as input — and, being a passthrough, the layouts it produces.
const GAIN_LAYOUTS: &[ChannelMask] = &[
    ChannelMask::MASK_MONO,
    ChannelMask::MASK_STEREO,
    ChannelMask::MASK_QUAD,
    ChannelMask::MASK_5_1,
    ChannelMask::MASK_7_1,
    ChannelMask::MASK_7_1_4,
];

/// Configuration for [`Gain`].
///
/// The layout is not configured here — it comes from the [`Spec`]'s input
/// layout, which gain passes straight through to its output.
#[derive(Clone, Debug, Default)]
pub struct GainConfig {
    /// Gain in decibels (0 dB = unity, the default).
    pub gain_db: f64,
}

/// A broadband gain.
pub struct Gain<S: Sample> {
    spec: Spec,
    gain_linear: f64,
    _marker: PhantomData<S>,
}

impl<S: Sample> Processor for Gain<S> {
    type Sample = S;
    type Config = GainConfig;

    fn new(spec: Spec, config: &Self::Config) -> Self {
        debug_assert!(
            Self::supported_input_layouts().contains(&spec.input_layout),
            "gain built for unsupported input layout {:?}",
            spec.input_layout,
        );
        Self {
            spec,
            gain_linear: db_to_linear(config.gain_db),
            _marker: PhantomData,
        }
    }

    fn update(&mut self, config: &Self::Config) {
        self.gain_linear = db_to_linear(config.gain_db);
    }

    fn process(&mut self, buffer: &mut Buffer<'_, S>) {
        let g = S::from_f64(self.gain_linear);
        let frames = buffer.frames();
        let channels = buffer.out_channels();
        for ch in 0..channels {
            for i in 0..frames {
                let x = buffer.input(ch)[i];
                buffer.output_mut(ch)[i] = x * g;
            }
        }
    }

    fn reset(&mut self) {
        // Gain is memoryless: nothing to clear.
    }

    fn spec(&self) -> &Spec {
        &self.spec
    }

    fn output_layout(&self) -> ChannelMask {
        // Gain preserves its input layout.
        self.spec.input_layout
    }

    fn supported_input_layouts() -> &'static [ChannelMask] {
        GAIN_LAYOUTS
    }

    fn supported_output_layouts() -> &'static [ChannelMask] {
        // Passthrough: it produces exactly the layouts it can accept.
        GAIN_LAYOUTS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::testing;

    const SR: f64 = 48_000.0;

    fn spec(max_frames: usize) -> Spec {
        Spec::new(SR, max_frames, ChannelMask::MASK_STEREO)
    }

    #[test]
    fn applies_steady_gain() {
        let mut gain = Gain::<f32>::new(spec(512), &GainConfig { gain_db: 6.0 });

        let input = vec![vec![1.0_f32; 256]; 2];
        let out = testing::run_offline(&mut gain, &input, 2, 128);

        let expected = db_to_linear(6.0) as f32;
        for ch in &out {
            for &s in ch {
                assert!((s - expected).abs() < 1e-4, "got {s}, expected {expected}");
            }
        }
    }

    #[test]
    fn update_changes_gain() {
        let mut gain = Gain::<f32>::new(spec(512), &GainConfig { gain_db: 0.0 });
        gain.update(&GainConfig { gain_db: -20.0 });

        let input = vec![vec![1.0_f32; 256]; 2];
        let out = testing::run_offline(&mut gain, &input, 2, 128);

        let expected = db_to_linear(-20.0) as f32;
        assert!((out[0][200] - expected).abs() < 1e-4);
    }

    #[test]
    fn reports_layout_and_outputs() {
        // The input layout comes from the spec; gain passes it through to output.
        let g = Gain::<f32>::new(
            Spec::new(SR, 256, ChannelMask::MASK_5_1),
            &GainConfig::default(),
        );
        assert_eq!(g.input_layout(), ChannelMask::MASK_5_1);
        assert_eq!(g.output_layout(), ChannelMask::MASK_5_1);
        assert_eq!(g.input_layout().channel_count(), 6);

        // Supported layouts are static (type-level), not tied to this instance.
        assert!(Gain::<f32>::supported_input_layouts().contains(&ChannelMask::MASK_5_1));
        assert_eq!(
            Gain::<f32>::supported_input_layouts(),
            Gain::<f32>::supported_output_layouts(),
        );
    }
}
