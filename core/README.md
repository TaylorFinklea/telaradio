# core/

Rust workspace member crate (`telaradio-core`). Cross-cutting types and
traits used by every other crate.

## Implemented (Phase 1a)

- `recipe::Recipe` — schema v1 struct matching `ARCHITECTURE.md` exactly
- `recipe::Modulation`, `recipe::ModelRef`, `recipe::Envelope`
- `Recipe::parse(&str) -> Result<Recipe, RecipeError>` — strict
  (`deny_unknown_fields`); validates `schema_version == "1"`,
  `depth ∈ [0, 1]`, `rate_hz > 0`, `duration_seconds > 0`
- `Recipe::serialize() -> Result<String, RecipeError>` — pretty JSON
- `error::RecipeError` — typed error variants for every rejection case

## Implemented (Phase 1b)

- `audio::WavBuffer { sample_rate, channels, samples }` — interleaved
  PCM buffer with `duration_seconds()` derivation
- `audio::DEFAULT_SAMPLE_RATE_HZ` (44_100) and `audio::DEFAULT_CHANNELS`
  (2) — the conventions every Generator should target
- `generator::Generator` — synchronous trait `(id, version,
  generate(prompt, seed, duration_seconds) -> WavBuffer)`. Object-safe.
- `generator::GeneratorError` — typed variants: Io, Subprocess, Wav,
  ProtocolMismatch

Run quality gates from project root:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Planned

- `dsp/` — amplitude modulation per Woods et al. 2024 (Phase 1c)
- ACE-Step generator implementation alongside the mock (Phase 1b2)

The `model-adapter/`, `dsp/`, and `library/` crates depend on this one.
