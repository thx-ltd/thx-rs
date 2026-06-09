# thx-dsp

A real-time DSP processor framework for THX audio blocks.

Every DSP block implements one trait, [`Processor`](src/common/processor.rs),
which encapsulates the logic shared across all of them.

The crate is split into two modules: [`common`](src/common.rs) (the reusable
framework) and [`processors`](src/processors.rs) (concrete blocks built on it).

## Design at a glance

| Concern | Where it lives |
| --- | --- |
| Sample type (generic `f32`/`f64`) | [`common/sample.rs`](src/common/sample.rs) |
| Channel topology (SMPTE / `WAVEFORMATEXTENSIBLE` masks) | [`common/channel_mask.rs`](src/common/channel_mask.rs) |
| Planar audio buffers | [`common/buffer.rs`](src/common/buffer.rs) |
| The `Processor` trait | [`common/processor.rs`](src/common/processor.rs) |
| Test instrumentation (feature `testing`) | [`common/testing.rs`](src/common/testing.rs) |
| Gain proof-of-concept | [`processors/gain.rs`](src/processors/gain.rs) |

## The `Processor` trait

- `new(spec, config)` — construct. The `Spec` carries the input layout;
  together with `config` it fixes the output layout.
- `process(&mut self, buffer)` — process one buffer in place.
- `update(&mut self, config)` — reconfigure in place.
- `input_layout()` / `output_layout()` — channel topology of this instance.
  `input_layout()` comes from the `Spec`; `output_layout()` is fixed at
  construction (gain passes it through; an upmix derives it from config, e.g.
  stereo in → 5.1 out). Channel counts come from `ChannelMask::channel_count()`.
- `supported_input_layouts()` / `supported_output_layouts()` — static
  (type-level) capability lists, queryable without an instance. A block may
  support several output layouts (e.g. an upmix to 5.1 *or* 7.1).
- `reset()` — clear internal state.

## Testing & instrumentation

Enable the `testing` feature for signal generators (`sine`, `dc`, `impulse`),
a `run_offline` helper that drives a `Processor` deterministically in fixed
blocks, and analyzers (`peak`, `rms`).

```sh
cargo test                      # unit + integration tests
cargo test --features testing   # also exposes testing utils to external crates
```

Future instrumentation worth adding: an FFT-based frequency-response sweep and
richer analyzers.
