# core/

Rust workspace member crate (`lockstep-core`). Cross-cutting types and
traits used by every other crate.

## Implemented (Phase 1a)

- `recipe::Recipe` — schema v1 struct matching `ARCHITECTURE.md` exactly
- `recipe::Modulation`, `recipe::ModelRef`, `recipe::Envelope`
- `Recipe::parse(&str) -> Result<Recipe, RecipeError>` — strict
  (`deny_unknown_fields`); validates `schema_version == "1"`,
  `depth ∈ [0, 1]`, `rate_hz > 0`, `duration_seconds > 0`
- `Recipe::serialize() -> Result<String, RecipeError>` — pretty JSON
- `error::RecipeError` — typed error variants for every rejection case

14 integration tests cover accept/reject/round-trip across every
acceptance criterion. Run from project root:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Planned (later Phase 1 sub-slices)

- `audio::WavBuffer` — sample-rate conventions, mono/stereo (Phase 1c
  alongside the DSP crate)
- `Generator` trait — model adapter contract (Phase 1b)

The `dsp/`, `model-adapter/`, and `library/` crates will depend on this
one.
