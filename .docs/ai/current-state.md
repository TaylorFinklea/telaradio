# Current state

**Date**: 2026-04-26
**Phase**: Phase 1c (AM modulation DSP) complete on branch `phase-1c`.
Phase 1b2 (real ACE-Step + HF download) building in parallel on
`phase-1b2`.
**Build status**: `cargo test --workspace` green (40/40 across
audio, generator, protocol, recipe, end-to-end, am_apply, dsp_pipeline);
`cargo clippy --all-targets -- -D warnings` clean (pedantic);
`cargo fmt --check` clean. Python `ruff check` and `ty check` clean.

## Last session summary

Phase 1c — AM modulation DSP. Bootstrapped the `dsp/` workspace member
(`telaradio-dsp`) with `apply_am(buffer, rate_hz, depth, envelope) ->
WavBuffer`, a pure transform per Woods et al. 2024 §Methods. Added a
DSP-side `Envelope` enum (Square / Sine / Triangle) decoupled from
`core::recipe::Envelope`, with a small `From` bridge. Square gate gets
a 1 ms linear crossfade centered on each transition to suppress
audible clicks at high depth. Stereo channels are modulated identically
(paper-faithful). Sample-rate-aware phase: `phase = (i / sr) *
rate_hz`.

10 new integration tests in `dsp/tests/am_apply.rs` cover depth=0
identity, depth=1 trough floor, rate-locked phase, Sine smoothness,
Triangle piecewise-linearity, stereo invariant, anti-click ramp, mono
support, and metadata preservation. 1 end-to-end smoke test in
`model-adapter/tests/dsp_pipeline.rs` proves the mock generator →
apply_am pipeline alters the sample distribution as expected.

See [`phases/phase-1c-am-modulation-report.md`](phases/phase-1c-am-modulation-report.md).

## What exists

- Phase 0 scaffold (CLAUDE.md, ARCHITECTURE.md, ROADMAP.md, README.md,
  PHASE_0_REPORT.md, LICENSE, CLA.md, `.github/`, module READMEs,
  `.docs/ai/` handoff)
- Cargo workspace at project root (members: `core`, `dsp`,
  `model-adapter`)
- `telaradio-core` crate (`core/`):
  - `recipe::*` — schema v1 types + strict parser
  - `audio::WavBuffer` + `DEFAULT_SAMPLE_RATE_HZ` / `DEFAULT_CHANNELS`
  - `generator::Generator` trait + `GeneratorError` enum
- `telaradio-dsp` crate (`dsp/`) — Phase 1c:
  - `dsp::Envelope` (Square / Sine / Triangle) + `From<core::recipe::Envelope>`
  - `dsp::apply_am(buffer, rate_hz, depth, envelope) -> WavBuffer`
- `telaradio-model-adapter` crate (`model-adapter/`):
  - `protocol::Request` / `protocol::Response` (NDJSON)
  - `subprocess::SubprocessGenerator` (mock-sine)
  - `python/telaradio_subprocess.py` + `python/pyproject.toml`
- 40 Rust integration tests across 7 test files
- `recipes/example-foggy-lofi.json` — realistic schema v1 example
- GitHub repo `TaylorFinklea/telaradio` (public)

## Blockers

None.

## What does NOT exist yet

- Real ACE-Step inference (Phase 1b2 — building in parallel)
- HF first-launch model download (Phase 1b2)
- `apple/` macOS Swift app (Phase 1d)
- Background buffer queue (Phase 1e)
- Remaining ~19 starter recipes (Phase 1f)
- Settings UI (Phase 1g)
- CLI smoke binary `telaradio-modulate` (deferred from Phase 1c —
  optional, defer until felt need)
- Configurable ramp-time field on `recipe.modulation` (Phase 2 candidate)

## Pointers

- [`next-steps.md`](next-steps.md) — exact next actions
- [`decisions.md`](decisions.md) — index of decision records
- [`phases/`](phases/) — phase specs and reports
- [`../../ROADMAP.md`](../../ROADMAP.md) — phases 1–4
