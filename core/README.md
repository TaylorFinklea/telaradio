# core/

Rust workspace root. Houses the cross-cutting types and traits used by
every other crate.

Contents (planned for Phase 1):

- `core/recipe/` — recipe struct, JSON (de)serialization, schema validator
- `core/error/` — shared error types
- `core/audio/` — `WavBuffer` type, sample-rate conventions

The `dsp/`, `model-adapter/`, and `library/` crates depend on `core/`.
