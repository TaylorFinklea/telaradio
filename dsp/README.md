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
