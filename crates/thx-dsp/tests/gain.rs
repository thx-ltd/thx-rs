//! Integration test: the gain processor through its public API.

use thx_dsp::{Buffer, ChannelMask, Gain, GainConfig, Processor, Spec};

#[test]
fn gain_applies_and_updates() {
    let spec = Spec::new(48_000.0, 256, ChannelMask::MASK_STEREO);
    let mut gain = Gain::<f32>::new(spec, &GainConfig { gain_db: 0.0 });

    // Reconfigure to -20 dB (0.1 linear) before processing.
    gain.update(&GainConfig { gain_db: -20.0 });

    let input = vec![vec![1.0_f32; 256]; 2];
    let mut output = vec![vec![0.0_f32; 256]; 2];

    // Scope the borrows so they end before we read `output`.
    {
        let in_refs: Vec<&[f32]> = input.iter().map(Vec::as_slice).collect();
        let mut out_refs: Vec<&mut [f32]> = output.iter_mut().map(Vec::as_mut_slice).collect();
        let mut buffer = Buffer::new(&in_refs, &mut out_refs, 256);
        gain.process(&mut buffer);
    }

    let expected = 10.0_f32.powf(-20.0 / 20.0); // 0.1
    assert!(
        (output[0][128] - expected).abs() < 1e-4,
        "expected gain applied, got {}",
        output[0][128]
    );
}
