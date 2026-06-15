//! Integration tests: the click-free bypass crossfade every block gets for free
//! through `BlockController::enable` — no author code, no wrapper to compose.

#[path = "utils/runner.rs"]
mod runner;
#[path = "utils/signal.rs"]
mod signal;

use thx_dsp::block::gain::{Gain, GainConfig};
use thx_dsp::{Block, ChannelMask, Smooth, Spec};

const SPEC: Spec = Spec {
    sample_rate: 48_000.0,
    layout: ChannelMask::MASK_STEREO,
};

/// 480 frames = 10 ms at 48 kHz = exactly one bypass crossfade.
const FADE: usize = 480;

fn cfg(gain_db: f64) -> GainConfig {
    GainConfig {
        gain_db: Smooth::new(gain_db),
    }
}

#[test]
fn disable_crossfades_to_passthrough_and_back() {
    let (mut processor, controller) = Gain::<f32>::spawn(&SPEC, FADE, &cfg(-20.0)).unwrap();

    let wet = 10.0_f32.powf(-20.0 / 20.0); // 0.1
    let input = vec![signal::dc::<f32>(1.0, FADE); 2];

    // Enabled: settled at the wet value.
    let output = runner::run_offline(&mut processor, &input, SPEC, 2, FADE);
    assert!((output[0][FADE - 1] - wet).abs() < 1e-6);

    // Disable: one block of monotone wet->dry crossfade, no jumps.
    controller.enable(false);
    let output = runner::run_offline(&mut processor, &input, SPEC, 2, FADE);
    assert!(
        (output[0][0] - wet).abs() < 0.05,
        "fade must start near the wet value, got {}",
        output[0][0]
    );
    assert!(
        output[0].windows(2).all(|w| w[1] >= w[0]),
        "fade must rise monotonically toward dry"
    );
    assert_eq!(output[0][FADE - 1], 1.0, "fade must end exactly at dry");

    // Fully bypassed: bit-exact passthrough.
    let output = runner::run_offline(&mut processor, &input, SPEC, 2, FADE);
    assert!(output[0].iter().all(|&s| s == 1.0));

    // Re-enable: fades back down and settles at the wet value again.
    controller.enable(true);
    let output = runner::run_offline(&mut processor, &input, SPEC, 2, FADE);
    assert!(
        output[0].windows(2).all(|w| w[1] <= w[0]),
        "fade must fall monotonically toward wet"
    );
    let output = runner::run_offline(&mut processor, &input, SPEC, 2, FADE);
    assert!((output[0][FADE - 1] - wet).abs() < 1e-6);
}
