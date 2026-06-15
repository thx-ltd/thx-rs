//! The offline processor driver for `thx-dsp`'s integration tests.
//!
//! Included by a test file with `#[path = "utils/runner.rs"] mod runner;`.
//! Not every test uses it, so unused-code warnings are silenced here.
#![allow(dead_code)]

use thx_dsp::{Block, BlockProcessor, BlockSignal, Buffer, Sample, Spec};

// Route this test binary's allocator through `assert_no_alloc` so that any heap
// allocation inside a guarded region (see `run_offline`) aborts. It only
// triggers inside the guard; allocation elsewhere behaves normally.
#[global_allocator]
static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

/// Drive `processor` over a planar `input` signal in fixed-size blocks at `spec`,
/// returning the planar output (`out_channels` channels).
///
/// Deterministic and single-threaded, so the same `input` always yields the same
/// output. `block` is the per-call frame count and must not exceed the
/// `max_frames` the processor was built for. Each `process` call runs inside an
/// `assert_no_alloc` guard, so a processor that allocates on the audio path
/// aborts the test.
pub fn run_offline<S: Sample, B: Block<S>>(
    processor: &mut BlockProcessor<S, B>,
    input: &[Vec<S>],
    spec: Spec,
    out_channels: usize,
    block: usize,
) -> Vec<Vec<S>> {
    let block = block.max(1);
    let in_channels = input.len();
    let total = input.iter().map(Vec::len).min().unwrap_or(0);

    // All allocation happens out here, before the realtime guard.
    let mut in_sig = BlockSignal {
        spec,
        buffer: Buffer::<S>::new(in_channels, block),
    };
    let mut out_sig = BlockSignal {
        spec,
        buffer: Buffer::<S>::new(out_channels, block),
    };
    let mut output: Vec<Vec<S>> = vec![Vec::with_capacity(2 * total); out_channels];

    let mut pos = 0;
    while pos < total {
        let n = block.min(total - pos);
        in_sig.buffer.set_frames(n);
        for (ch, channel) in input.iter().enumerate() {
            in_sig.buffer.channel_mut(ch).copy_from_slice(&channel[pos..pos + n]);
        }
        // Engine convention: output frames default to input frames; the block
        // may override (e.g. a resampler).
        out_sig.buffer.set_frames(n);

        assert_no_alloc::assert_no_alloc(|| processor.process(&in_sig, &mut out_sig));

        for (ch, collected) in output.iter_mut().enumerate() {
            collected.extend_from_slice(out_sig.buffer.channel(ch));
        }
        pos += n;
    }

    output
}
