# dsp/

Rust crate. Amplitude modulation per Woods et al. 2024
([doi:10.1038/s42003-024-07026-3](https://www.nature.com/articles/s42003-024-07026-3))
plus the audio graph that future stages slot into.

v1 stages:

```
WavBuffer  →  AM(rate_hz, depth, envelope)  →  WavBuffer
```

Default modulation params: 16 Hz square, depth 0.5. See
[`../ARCHITECTURE.md`](../ARCHITECTURE.md) §Modulation DSP stages for the
math and reasoning.

The DSP stages are pure functions: no side effects, no allocation beyond
the output buffer.

## Implemented (Phase 1c)

`apply_am(buffer, rate_hz, depth, envelope) -> WavBuffer` lives in
[`src/am.rs`](src/am.rs). The DSP-side envelope shape lives in
[`src/envelope.rs`](src/envelope.rs) as `dsp::Envelope` (decoupled from
`core::recipe::Envelope` so DSP can grow new shapes without bumping the
recipe schema). A `From<core::recipe::Envelope>` conversion bridges the
two for pipeline code.

```rust,ignore
use telaradio_core::WavBuffer;
use telaradio_dsp::{apply_am, Envelope};

let modulated: WavBuffer = apply_am(&input, 16.0, 0.5, Envelope::Square);
```

Behavior:

- depth = 0 returns samples unchanged (within f32 epsilon).
- Stereo channels receive identical gate values per frame
  (paper-faithful per Woods et al. 2024).
- A 1 ms linear crossfade is applied around each Square transition
  (peak ↔ trough) to suppress audible clicks at high depth. Sine and
  Triangle envelopes are continuous and need no ramp.
- Phase is reset at frame 0; identical `(i, sample_rate, rate_hz)`
  triples produce identical gate values regardless of channel count.

Test coverage: `dsp/tests/am_apply.rs` (10 tests) plus an end-to-end
smoke test in `model-adapter/tests/dsp_pipeline.rs`.
