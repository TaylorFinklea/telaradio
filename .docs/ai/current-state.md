# Current state

**Date**: 2026-04-27
**Phase**: Phase 1b complete; Phase 1b2 (real ACE-Step + HF download)
not yet started.
**Build status**: `cargo test` green (29/29 across audio, generator,
protocol, recipe, end-to-end), `cargo clippy --all-targets -- -D
warnings` clean (pedantic), `cargo fmt --check` clean. Python `ruff
check` and `ty check` clean.

## Last session summary

Phase 1b — Generator trait + mock subprocess. Added `core::audio` (with
`WavBuffer` and the 44.1 kHz / stereo / DEFAULT constants) and
`core::generator` (the `Generator` trait + `GeneratorError`). Bootstrapped
the `model-adapter` workspace member crate with NDJSON-over-stdio IPC
types, a Python script speaking the protocol, and a `SubprocessGenerator`
that spawns/holds/drops the child process cleanly. End-to-end test
exercises the full pipeline: Rust spawns Python, sends a `Request`,
Python writes a 440 Hz sine WAV, Rust reads it and returns a `WavBuffer`.

Project rename from Lockstep landed earlier in the same date. See
[`phases/phase-1b-model-adapter-report.md`](phases/phase-1b-model-adapter-report.md).

## What exists

- Phase 0 scaffold (CLAUDE.md, ARCHITECTURE.md, ROADMAP.md, README.md,
  PHASE_0_REPORT.md, LICENSE, CLA.md, `.github/`, module READMEs,
  `.docs/ai/` handoff)
- Cargo workspace at project root
- `telaradio-core` crate (`core/`):
  - `recipe::*` — schema v1 types + strict parser
  - `audio::WavBuffer` + `DEFAULT_SAMPLE_RATE_HZ` (44_100) +
    `DEFAULT_CHANNELS` (2)
  - `generator::Generator` trait + `GeneratorError` enum
- `telaradio-model-adapter` crate (`model-adapter/`):
  - `protocol::Request` / `protocol::Response` (NDJSON)
  - `subprocess::SubprocessGenerator` (mock-sine)
  - `python/telaradio_subprocess.py` + `python/pyproject.toml`
- 29 Rust integration tests across 4 test files
- `recipes/example-foggy-lofi.json` — realistic schema v1 example
- GitHub repo `TaylorFinklea/telaradio` (public)

## Blockers

None.

## What does NOT exist yet

- Real ACE-Step inference (Phase 1b2)
- HF first-launch model download (Phase 1b2)
- `dsp/` amplitude modulation (Phase 1c)
- `apple/` macOS Swift app (Phase 1d)
- Background buffer queue (Phase 1e)
- Remaining ~19 starter recipes (Phase 1f)
- Settings UI (Phase 1g)

## Pointers

- [`next-steps.md`](next-steps.md) — exact next actions
- [`decisions.md`](decisions.md) — index of decision records
- [`phases/`](phases/) — phase specs and reports
- [`../../ROADMAP.md`](../../ROADMAP.md) — phases 1–4
