# Phase Report: Phase 1a — Recipe Core

**Date:** 2026-04-27
**Outcome:** pass
**Spec:** [`phase-1a-recipe-core-spec.md`](phase-1a-recipe-core-spec.md)

## Changes

- `Cargo.toml` (new) — workspace root; `[workspace.dependencies]` pins
  serde, serde_json, thiserror, chrono, uuid; pedantic clippy lints
- `core/Cargo.toml` (new) — `lockstep-core` package, deps from workspace
- `core/src/lib.rs` (new) — re-exports `Recipe`, `Envelope`, `ModelRef`,
  `Modulation`, `RecipeError`
- `core/src/error.rs` (new) — `RecipeError` enum (Json, UnsupportedSchemaVersion,
  DepthOutOfRange, RateNonPositive, DurationZero)
- `core/src/recipe.rs` (new) — `Recipe`, `Modulation`, `ModelRef`,
  `Envelope` types; `Recipe::parse` and `Recipe::serialize` with
  `#[serde(deny_unknown_fields)]` everywhere; semantic validation
  (schema version, depth, rate, duration)
- `core/tests/recipe_parse.rs` (new) — 14 integration tests covering
  accept/reject/round-trip
- `recipes/example-foggy-lofi.json` (new) — realistic schema-v1 recipe
- `core/README.md` — appended "Implemented (Phase 1a)" section
- `.gitignore` (new) — Rust target/, macOS, editor, runtime data
- `.docs/ai/current-state.md` — updated session summary
- `.docs/ai/next-steps.md` — Phase 1b queued
- `.docs/ai/decisions.md` — appended Phase 1a decisions
- `ROADMAP.md` — Phase 1 items 1–2 marked `[x]`

## Decisions made

See `decisions.md` 2026-04-27 entry. Summary: GitHub repo public as
`TaylorFinklea/lockstep`; strict `uuid::Uuid`; single root workspace;
strict schema parsing. Plus a few mid-build judgment calls (inline test
JSON over fixture files; serde-then-semantic validation order; edition
2024 / rust-version 1.85).

## Verification results

```
$ cargo test
running 14 tests
test result: ok. 14 passed; 0 failed; 0 ignored

$ cargo clippy --all-targets -- -D warnings
Checking lockstep-core v0.0.1
Finished `dev` profile [unoptimized + debuginfo]

$ cargo fmt --check
(clean)
```

### Manual verification checklist

- [x] `cargo test` from project root passes 14/14 integration tests
- [x] `cargo clippy --all-targets -- -D warnings` clean (pedantic enabled)
- [x] `cargo fmt --check` clean
- [x] Round-trip test passes against `recipes/example-foggy-lofi.json`
- [x] Strict parsing rejects unknown top-level field (`extra_field`)
- [x] Strict parsing rejects malformed UUID
- [x] Each envelope variant (square, sine, triangle) parses

## Follow-up items

- [ ] Decide audio sample rate + mono/stereo at start of Phase 1b
- [ ] Decide IPC format with the Python subprocess at start of Phase 1b
- [ ] Once `Generator` trait lives in `core::audio`, revisit whether
      `Recipe.duration_seconds` should also be enforced against a
      maximum (today 0 is rejected; 99,999 is accepted)
- [ ] Eventually: a CLI smoke binary (`cargo run -p lockstep-core
      --bin recipe-validate -- path/to/recipe.json`) to make manual
      validation cheap. Skipped this phase to keep scope tight.

## Context for next phase

- TDD discipline held: fixtures + 14 failing tests written first; cargo
  test compiled with `unresolved import lockstep_core` as the expected
  RED, then minimal implementation made all 14 green on the first run.
- `core/Cargo.toml` is set up to inherit workspace deps cleanly. New
  member crates can pattern-match on it.
- Pedantic clippy lints are enabled at the workspace level; expect to
  bump into stylistic ones (already saw `unreadable_literal`,
  `float_cmp`). Don't disable them — fix the call sites.
- Edition 2024 is pinned. If a future contributor's toolchain is older
  than rustc 1.85, they'll get a friendly error.
