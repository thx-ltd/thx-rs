//! The framework runtime: the lock-free pair every [`DspBlock`] is split into.
//!
//! [`spawn`] produces a [`DspBlockProcessor`] (audio thread) and a
//! [`DspBlockController`] (control thread) — the two ends of a lock-free wire.
//! Config snapshots cross via `triple_buffer`; enable/reset cross via atomics.
//! No locks, no audio-thread allocation.
//!
//! These are concrete types, not traits: there is exactly one of each per block.
//! A shared RT interface (`Box<dyn ...>`) only earns its place once the graph
//! needs to hold heterogeneous units, so it is deferred until then.

use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use triple_buffer::{Input, Output};

use crate::buffer::Buffer;
use crate::sample::Sample;
use crate::spec::Spec;

use super::{DspBlock, DspBlockSignal, Error, Result};

/// Lock-free control signals shared across the thread boundary.
struct Shared {
    enabled: AtomicBool,
    /// Monotonic count of reset requests; the audio thread resets whenever this
    /// runs ahead of the count it has already handled.
    reset_requests: AtomicU32,
}

/// Audio-thread half of a spawned block: the block plus the universal bypass
/// crossfade.
///
/// [`process`](Self::process) drains the latest config/reset/enable signals, then
/// runs the inner [`DspBlock::process`] — straight through when enabled, as a
/// bit-exact passthrough when bypassed, or as a click-free crossfade in between.
pub struct DspBlockProcessor<S: Sample, B: DspBlock<S>> {
    block: B,
    rx: Output<B::Config>,
    shared: Arc<Shared>,
    /// Scratch for the processed ("wet") signal during a crossfade.
    wet: DspBlockSignal<S>,
    /// Wet weight in `0.0..=1.0`: 1.0 = fully processed, 0.0 = fully bypassed.
    fade: f64,
    /// Per-sample fade increment.
    fade_step: f64,
    /// Reset count this processor has already applied, diffed against
    /// [`Shared::reset_requests`] each block.
    handled_resets: u32,
}

impl<S: Sample, B: DspBlock<S>> DspBlockProcessor<S, B> {
    /// Process one block, reading `input` and writing `output`. Audio thread.
    pub fn process(&mut self, input: &DspBlockSignal<S>, output: &mut DspBlockSignal<S>) {
        if !B::accepts(&input.spec) {
            // A spec the block can't handle. `process` has no fallible path, so
            // pass audio through untouched rather than produce garbage.
            output.spec = input.spec;
            output.buffer.copy_from(&input.buffer);
            return;
        }

        if self.rx.update() {
            let config = self.rx.output_buffer_mut();
            self.block.configure(config);
        }

        let requested = self.shared.reset_requests.load(Ordering::Acquire);
        if requested != self.handled_resets {
            self.handled_resets = requested;
            self.block.reset();
        }

        let enabled = self.shared.enabled.load(Ordering::Acquire);

        if enabled && self.fade >= 1.0 {
            // Fully enabled: process straight through.
            self.block.process(input, output);
        } else if !enabled && self.fade <= 0.0 {
            // Fully bypassed: bit-exact passthrough, block skipped.
            output.spec = input.spec;
            output.buffer.copy_from(&input.buffer);
        } else {
            // Crossfading: mix dry (input) and wet (processed) linearly.
            let frames = input.buffer.frames();
            self.wet.buffer.set_frames(frames);
            self.block.process(input, &mut self.wet);
            output.spec = self.wet.spec;
            output.buffer.set_frames(frames);

            let start = self.fade;
            let direction = if enabled { 1.0 } else { -1.0 };
            let delta = direction * self.fade_step;
            for ch in 0..output.buffer.channels() {
                let dry = input.buffer.channel(ch);
                let wet = self.wet.buffer.channel(ch);
                let out = output.buffer.channel_mut(ch);
                // TODO: SIMD Opportunity
                for k in 0..frames {
                    let g = (start + delta * (k + 1) as f64).clamp(0.0, 1.0);
                    out[k] = dry[k] * S::from_f64(1.0 - g) + wet[k] * S::from_f64(g);
                }
            }
            self.fade = (start + delta * frames as f64).clamp(0.0, 1.0);
        }
    }
}

/// Control-thread half of a spawned block: retune, bypass, reset, and inspect a
/// running block. Every method here drives the [`DspBlockProcessor`] lock-free.
pub struct DspBlockController<S: Sample, B: DspBlock<S>> {
    config: B::Config,
    tx: Input<B::Config>,
    shared: Arc<Shared>,
    _marker: PhantomData<S>,
}

impl<S: Sample, B: DspBlock<S>> DspBlockController<S, B> {
    /// Validate and apply a new config, returning the effective (normalized) one.
    pub fn update(&mut self, config: &B::Config) -> Result<B::Config> {
        let effective = B::validate(config)?;
        self.tx.write(effective.clone());
        self.config = effective.clone();
        Ok(effective)
    }

    /// Enable (process) or disable (bypass) the block, with a click-free fade.
    pub fn enable(&self, enabled: bool) {
        self.shared.enabled.store(enabled, Ordering::Release);
    }

    /// Reset the block's internal state at the start of its next block.
    pub fn reset(&self) {
        self.shared.reset_requests.fetch_add(1, Ordering::Release);
    }

    /// The last applied effective config.
    pub fn config(&self) -> &B::Config {
        &self.config
    }
}

/// Build the `(processor, controller)` pair for a block. See [`DspBlock::spawn`].
pub fn spawn<S: Sample, B: DspBlock<S>>(
    spec: &Spec,
    max_frames: usize,
    config: &B::Config,
) -> Result<(DspBlockProcessor<S, B>, DspBlockController<S, B>)> {
    if !B::accepts(spec) {
        return Err(Error::UnsupportedLayout(spec.layout));
    }

    let effective = B::validate(config)?;
    let block = B::new(spec, max_frames, &effective)?;
    let output_spec = B::output_spec(spec, &effective);

    let shared = Arc::new(Shared {
        enabled: AtomicBool::new(true),
        reset_requests: AtomicU32::new(0),
    });
    let (tx, rx) = triple_buffer::triple_buffer(&effective);
    let fade_len = (0.01 * spec.sample_rate).round().max(1.0) as usize;

    let processor = DspBlockProcessor {
        block,
        rx,
        shared: Arc::clone(&shared),
        wet: DspBlockSignal {
            spec: output_spec,
            buffer: Buffer::new(output_spec.channels(), max_frames),
        },
        fade: 1.0,
        fade_step: 1.0 / fade_len as f64,
        handled_resets: 0,
    };
    let controller = DspBlockController {
        config: effective,
        tx,
        shared,
        _marker: PhantomData,
    };
    Ok((processor, controller))
}
