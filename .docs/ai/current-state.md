# Current state

**Date**: 2026-04-27
**Phase**: Phase 1a complete; Phase 1b not yet started.
**Build status**: `cargo test` green (14/14), `cargo clippy --all-targets
-- -D warnings` clean, `cargo fmt --check` clean.

## Last session summary

Phase 1a — Recipe Core. Bootstrapped the Cargo workspace, implemented the
recipe schema v1 types and parser following TDD (fixtures + 14 failing
tests, then minimal implementation). Added a realistic example recipe at
`recipes/example-foggy-lofi.json`. Initialized git; first commit was the
Phase 0 scaffold; second commit is the Phase 1a build. See
[`phases/phase-1a-recipe-core-report.md`](phases/phase-1a-recipe-core-report.md).

## What exists

- Phase 0 scaffold (CLAUDE.md, ARCHITECTURE.md, ROADMAP.md, README.md,
  PHASE_0_REPORT.md, LICENSE, CLA.md, `.github/`, module READMEs,
  `.docs/ai/` handoff)
- Cargo workspace at project root; `lockstep-core` crate under `core/`
- `Recipe`, `Modulation`, `ModelRef`, `Envelope`, `RecipeError` types
- `Recipe::parse` (strict, deny_unknown_fields) and `Recipe::serialize`
- 14 integration tests covering schema v1
- One realistic recipe at `recipes/example-foggy-lofi.json`
- GitHub repo `TaylorFinklea/lockstep` (public)

## Blockers

None.

## What does NOT exist yet

- `model-adapter/` ACE-Step Python subprocess wrapper (Phase 1b)
- HF model first-launch download (Phase 1b)
- `dsp/` amplitude modulation (Phase 1c)
- `apple/` macOS Swift app shell (Phase 1d)
- Background buffer queue (Phase 1e)
- The remaining ~19 starter recipes (Phase 1f)
- Settings UI: preset / 3-tier / advanced panel (Phase 1g)

## Pointers

- [`next-steps.md`](next-steps.md) — exact next actions
- [`decisions.md`](decisions.md) — index of decision records
- [`phases/`](phases/) — phase specs and reports
- [`../../ROADMAP.md`](../../ROADMAP.md) — phases 1–4
