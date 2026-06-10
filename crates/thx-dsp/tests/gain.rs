//! Integration test: the gain processor through its public API.

#[path = "utils/runner.rs"]
mod runner;
#[path = "utils/signal.rs"]
mod signal;

use thx_dsp::{Buffer, ChannelMask, Controller, Gain, GainConfig, Processor, Spec};

#[test]
fn gain_applies_and_updates() {
    let spec = Spec::new(48_000.0, 256, ChannelMask::MASK_STEREO);
    let (mut controller, mut gain) = Gain::<f32>::new(spec, &GainConfig { gain_db: 0.0 });

    // Reconfigure to -20 dB (0.1 linear), then snap past the ramp so we can
    // assert the settled value rather than a point on the smoothing curve.
    controller.update(&GainConfig { gain_db: -20.0 });
    controller.reset();

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

#[test]
fn gain_change_ramps_in() {
    let spec = Spec::new(48_000.0, 256, ChannelMask::MASK_STEREO);
    // Start at unity (settled), then move to -20 dB without resetting: the first
    // block should ramp from ~1.0 toward 0.1 rather than jumping.
    let (mut controller, mut gain) = Gain::<f32>::new(spec, &GainConfig { gain_db: 0.0 });
    controller.update(&GainConfig { gain_db: -20.0 });

    let input = vec![vec![1.0_f32; 256]; 2];
    let mut output = vec![vec![0.0_f32; 256]; 2];
    {
        let in_refs: Vec<&[f32]> = input.iter().map(Vec::as_slice).collect();
        let mut out_refs: Vec<&mut [f32]> = output.iter_mut().map(Vec::as_mut_slice).collect();
        let mut buffer = Buffer::new(&in_refs, &mut out_refs, 256);
        gain.process(&mut buffer);
    }

    // Ramp is monotonically decreasing and starts near unity.
    assert!(output[0][0] > output[0][255], "expected a downward ramp");
    assert!(output[0][0] > 0.9, "ramp should start near the old value");
    // Both channels see the identical ramp.
    assert_eq!(output[0], output[1]);
}

#[test]
fn gain_settles_through_offline_driver() {
    use runner::run_offline;

    // Drives the processor in fixed blocks via `run_offline`, which also asserts
    // `process` never allocates (the no-alloc guard is always on in tests).
    let spec = Spec::new(48_000.0, 64, ChannelMask::MASK_MONO);
    let (mut controller, mut gain) = Gain::<f32>::new(spec, &GainConfig { gain_db: -6.0 });
    controller.update(&GainConfig { gain_db: -6.0 });

    let input = vec![vec![1.0_f32; 4096]];
    let output = run_offline(&mut gain, &input, 1, 64);

    // After the 10 ms ramp settles, the tail should sit at the -6 dB factor.
    let expected = 10.0_f32.powf(-6.0 / 20.0);
    assert!(
        (output[0][4095] - expected).abs() < 1e-4,
        "expected settled -6 dB, got {}",
        output[0][4095]
    );
}
