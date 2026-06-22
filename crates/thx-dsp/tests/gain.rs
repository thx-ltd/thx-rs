//! Integration tests: the gain block through `spawn` and its controller.

#[path = "utils/runner.rs"]
mod runner;
#[path = "utils/signal.rs"]
mod signal;

use thx_dsp::block::gain::{Gain, GainConfig};
use thx_dsp::{DspBlock, ChannelMask, Error, Smooth, Spec};

const SPEC: Spec = Spec {
    sample_rate: 48_000.0,
    layout: ChannelMask::MASK_STEREO,
};

fn db(db: f64) -> f32 {
    10.0_f32.powf(db as f32 / 20.0)
}

fn cfg(gain_db: f64) -> GainConfig {
    GainConfig {
        gain_db: Smooth::new(gain_db),
    }
}

#[test]
fn applies_initial_gain_without_ramp() {
    // The smoother starts settled at the initial config: the very first block
    // must already sit at -20 dB.
    let (mut gain, _controller) = Gain::<f32>::spawn(&SPEC, 256, &cfg(-20.0)).unwrap();

    let input = vec![signal::dc::<f32>(1.0, 256); 2];
    let output = runner::run_offline(&mut gain, &input, SPEC, 2, 256);

    assert!((output[0][0] - db(-20.0)).abs() < 1e-6, "got {}", output[0][0]);
    assert_eq!(output[0], output[1]);
}

#[test]
fn update_ramps_in() {
    // Start settled at unity, then move to -20 dB: the first block must ramp
    // down monotonically instead of jumping.
    let (mut gain, mut controller) = Gain::<f32>::spawn(&SPEC, 256, &cfg(0.0)).unwrap();
    controller.update(&cfg(-20.0)).unwrap();

    let input = vec![signal::dc::<f32>(1.0, 256); 2];
    let output = runner::run_offline(&mut gain, &input, SPEC, 2, 256);

    assert!(output[0][0] > 0.9, "ramp should start near the old value");
    assert!(
        output[0].windows(2).all(|w| w[1] <= w[0]),
        "ramp must decrease monotonically"
    );
    assert_eq!(output[0], output[1], "channels must see the identical ramp");
}

#[test]
fn update_settles_at_target() {
    let (mut gain, mut controller) = Gain::<f32>::spawn(&SPEC, 64, &cfg(0.0)).unwrap();
    controller.update(&cfg(-6.0)).unwrap();

    // 4096 frames at 48 kHz is well past the 10 ms ramp.
    let input = vec![signal::dc::<f32>(1.0, 4096); 2];
    let output = runner::run_offline(&mut gain, &input, SPEC, 2, 64);

    assert!((output[0][4095] - db(-6.0)).abs() < 1e-6, "got {}", output[0][4095]);
}

#[test]
fn controller_reset_snaps_past_the_ramp() {
    let (mut gain, mut controller) = Gain::<f32>::spawn(&SPEC, 256, &cfg(0.0)).unwrap();
    controller.update(&cfg(-20.0)).unwrap();
    controller.reset();

    // The pending config is applied and the reset consumed at the top of the
    // next process call: the whole first block already sits at the target.
    let input = vec![signal::dc::<f32>(1.0, 256); 2];
    let output = runner::run_offline(&mut gain, &input, SPEC, 2, 256);
    assert!((output[0][0] - db(-20.0)).abs() < 1e-6, "got {}", output[0][0]);
}

#[test]
fn update_reports_effective_clamped_config() {
    let (_gain, mut controller) = Gain::<f32>::spawn(&SPEC, 64, &GainConfig::default()).unwrap();

    let effective = controller.update(&cfg(100.0)).unwrap();
    assert_eq!(effective.gain_db.target(), 30.0, "gain must clamp to +30 dB");
    assert_eq!(controller.config().gain_db.target(), 30.0);
}

#[test]
fn spawn_rejects_unsupported_layout() {
    let bad = Spec::new(48_000.0, ChannelMask::FRONT_LEFT);
    let Err(err) = Gain::<f32>::spawn(&bad, 64, &GainConfig::default()) else {
        panic!("layout must be rejected");
    };
    assert!(matches!(err, Error::UnsupportedLayout(_)));
}
