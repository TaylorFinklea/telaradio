# Phase Report: Phase 1b — Generator trait + mock subprocess

**Date:** 2026-04-27
**Outcome:** pass
**Spec:** [`phase-1b-model-adapter-spec.md`](phase-1b-model-adapter-spec.md)

## Changes

- `Cargo.toml` — workspace members now `["core", "model-adapter"]`;
  added `hound`, `tempfile`, and the `telaradio-core` path dep to
  `[workspace.dependencies]`
- `core/src/audio.rs` (new) — `WavBuffer`, `DEFAULT_SAMPLE_RATE_HZ`
  (44_100), `DEFAULT_CHANNELS` (2), `WavBuffer::duration_seconds`
- `core/src/generator.rs` (new) — `Generator` trait (sync,
  object-safe) + `GeneratorError` (Io, Subprocess, Wav,
  ProtocolMismatch)
- `core/src/lib.rs` — re-export `audio::*`, `generator::*`
- `core/tests/audio.rs` (new) — 4 tests for `WavBuffer`
- `core/tests/generator.rs` (new) — 2 tests for trait + error
- `model-adapter/Cargo.toml` (new)
- `model-adapter/src/lib.rs` (new) — module wiring + re-exports
- `model-adapter/src/protocol.rs` (new) — `Request` and `Response`
  with `#[serde(tag = "kind", rename_all = "lowercase")]`
- `model-adapter/src/subprocess.rs` (new) — `SubprocessGenerator`
  with spawn / Mutex<IoState> / Drop cleanup; `read_wav` helper
- `model-adapter/tests/protocol_serde.rs` (new) — 5 round-trip tests
- `model-adapter/tests/end_to_end.rs` (new) — 4 end-to-end tests
  spawning the Python subprocess
- `model-adapter/python/telaradio_subprocess.py` (new) — stdlib NDJSON
  loop + mock 440 Hz sine engine, type-annotated, ruff/ty clean
- `model-adapter/python/pyproject.toml` (new) — minimal `[project]`
  + ruff + ty config
- `core/README.md` — appended "Implemented (Phase 1b)" section
- `model-adapter/README.md` — full rewrite for Phase 1b shape
- `.gitignore` — added Python venv / cache patterns
- `.docs/ai/current-state.md` — Phase 1b summary
- `.docs/ai/next-steps.md` — Phase 1c and Phase 1b2 queued
- `.docs/ai/decisions.md` — appended Phase 1b entry
- `ROADMAP.md` — Phase 1 item 3 marked `[~]` with split note

## Decisions made

See `decisions.md` 2026-04-27 entry "Phase 1b: Generator trait + mock
subprocess". Summary: trait is sync, IPC is NDJSON-over-stdio + temp
WAV file, mock generator id `mock-sine`, audio defaults 44.1 kHz
stereo, one subprocess per `SubprocessGenerator` instance.

## Verification results

```
$ cargo test
running 4 tests   (core/tests/audio.rs)         test result: ok. 4 passed
running 2 tests   (core/tests/generator.rs)     test result: ok. 2 passed
running 14 tests  (core/tests/recipe_parse.rs)  test result: ok. 14 passed
running 4 tests   (model-adapter end_to_end.rs) test result: ok. 4 passed
running 5 tests   (model-adapter protocol)      test result: ok. 5 passed

Total: 29 passed; 0 failed; 0 ignored

$ cargo clippy --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo]

$ cargo fmt --check
(clean)

$ uv run --with ruff ruff check .   (model-adapter/python)
All checks passed!

$ uv run --with ty ty check .   (model-adapter/python)
All checks passed!
```

### Manual verification checklist

- [x] `cargo test` from project root passes 29/29 across all crates
- [x] `cargo clippy --all-targets -- -D warnings` clean (pedantic)
- [x] `cargo fmt --check` clean
- [x] Python ruff clean (`select = ["ALL"]` minus formatter conflicts)
- [x] Python ty clean (`error-on-warning = true`)
- [x] End-to-end: Rust → Python → 1s mock WAV → Rust `WavBuffer` with
      88_200 samples (1s × 44_100 × 2)
- [x] Subprocess held open across multiple `generate` calls
- [x] `Drop` cleanly kills + reaps the child process

## Follow-up items

- [ ] **Decide Phase 1c vs 1b2 sequence.** Recommended in
      `next-steps.md`: Phase 1c (AM DSP) first because the mock
      already unblocks it.
- [ ] In Phase 1b2: switch the Python venv from ad-hoc
      `uv run --with ruff` to a proper uv-managed project under
      `model-adapter/python/` (the `pyproject.toml` is already there;
      just add ACE-Step + `huggingface_hub` runtime deps).
- [ ] Eventually: a `pgrep`-based test that proves `Drop` doesn't
      leak zombies (current test exercises `Drop` but doesn't verify
      absence).
- [ ] If subprocess startup latency becomes user-visible, add a
      "warm-up" call after `spawn` so the first user-facing
      `generate` doesn't pay the import cost.

## Context for next phase

- TDD discipline held. Every Rust module landed with: failing test
  asserting the public API → minimal impl → verify green. The
  `Generator` trait was designed by writing the in-memory test impl
  *first*, which confirmed object-safety before the trait was ever
  written.
- The Python script is stdlib-only by design — no numpy, no scipy.
  Keeps Phase 1b portable; Phase 1b2 can pull heavier deps under a
  `[project.dependencies]` block.
- `WavBuffer.samples` is `Vec<f32>` in [-1.0, 1.0], interleaved when
  `channels > 1`. The DSP crate (Phase 1c) operates on this shape.
- The IPC protocol is stable: Phase 1b2's real ACE-Step generator
  sends the same `Request` shape and returns the same `Response`
  shape. Any subprocess that speaks NDJSON + `kind: ok|err` is
  drop-in compatible with `SubprocessGenerator`.
- One landmine for future maintainers: `gen` is a reserved keyword in
  Rust 2024 edition, so don't name variables `gen` (use `generator`).
