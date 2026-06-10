//! The offline processor driver for `thx-dsp`'s integration tests.
//!
//! Included by a test file with `#[path = "utils/runner.rs"] mod runner;`.
//! Not every test uses it, so unused-code warnings are silenced here.
#![allow(dead_code)]

use thx_dsp::{Buffer, Processor, Sample};

// Route this test binary's allocator through `assert_no_alloc` so that any heap
// allocation inside a guarded region (see `run_offline`) aborts. It only triggers
// inside the guard; allocation elsewhere behaves normally.
#[global_allocator]
static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

/// Drive `processor` over a planar `input` signal in fixed-size blocks,
/// returning the planar output (`out_channels` channels).
///
/// Deterministic and single-threaded: the same `input` always yields the same
/// output, which is what makes processor behaviour unit-testable. `block` is the
/// per-call frame count; it must not exceed the processor's `max_frames`. Each
/// `process` call runs inside an `assert_no_alloc` guard, so a processor that
/// allocates on the audio path aborts the test.
pub fn run_offline<P: Processor>(
    processor: &mut P,
    input: &[Vec<P::Sample>],
    out_channels: usize,
    block: usize,
) -> Vec<Vec<P::Sample>> {
    let block = block.max(1);
    let in_channels = input.len();
    let total = input.iter().map(Vec::len).min().unwrap_or(0);

    let mut output = vec![vec![<P::Sample as Sample>::ZERO; total]; out_channels];

    let mut pos = 0;
    while pos < total {
        let n = block.min(total - pos);

        let in_refs: Vec<&[P::Sample]> =
            (0..in_channels).map(|c| &input[c][pos..pos + n]).collect();
        let mut out_windows: Vec<&mut [P::Sample]> =
            output.iter_mut().map(|c| &mut c[pos..pos + n]).collect();

        let mut buffer_view = Buffer::new(&in_refs, &mut out_windows, n);

        // `process` must be realtime-safe; prove it allocates nothing by running
        // it inside the no-alloc guard.
        assert_no_alloc::assert_no_alloc(|| processor.process(&mut buffer_view));

        pos += n;
    }

    output
}
