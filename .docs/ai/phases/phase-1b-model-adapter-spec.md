# Phase Spec: Phase 1b — Generator trait + mock subprocess

**Roadmap item:** Phase 1, item 3 (Python ACE-Step subprocess wrapper) — split.
This slice is the *contract and pipeline*; Phase 1b2 swaps the mock for real
ACE-Step inference and adds the HF model download.

**Date:** 2026-04-27

## Product

**Goal:** A working `Generator` trait in `telaradio-core`, plus a
`model-adapter` crate that spawns a Python subprocess implementing the
trait via a stable IPC protocol. In this slice the subprocess returns a
deterministic mock signal (a 440 Hz sine wave matching the requested
duration), so the entire pipeline — recipe → model-adapter → subprocess
→ `WavBuffer` — is exercised end-to-end without depending on a 5 GB
model or a GPU. Phase 1b2 swaps in real ACE-Step inference behind the
same trait.

### Acceptance criteria

- [ ] `core::audio::WavBuffer { sample_rate, channels, samples }` type
- [ ] `core::generator::Generator` trait (sync) + `GeneratorError`
- [ ] `model-adapter` workspace member crate bootstraps cleanly
- [ ] IPC request/response types defined in
      `model-adapter::protocol`, with serde round-trip tests
- [ ] `SubprocessGenerator` implements `Generator` by spawning, holding,
      and cleanly tearing down a Python child process
- [ ] `model-adapter/python/telaradio_subprocess.py` reads NDJSON
      requests on stdin, returns NDJSON responses, writes WAV to a temp
      file, returns the path
- [ ] Mock engine: 440 Hz sine wave at the agreed sample rate +
      channels for the requested duration
- [ ] Integration test (gated by `#[ignore]` only if Python is missing)
      asserts: generated buffer length = `duration_seconds * sample_rate
      * channels`, sample rate matches, content is non-zero
- [ ] Rust quality gates: `cargo test`, `cargo clippy --all-targets --
      -D warnings` (pedantic), `cargo fmt --check`
- [ ] Python quality gates: `ruff check`, `ty check` (per the global
      modern-python conventions)
- [ ] Phase 1b report at `.docs/ai/phases/phase-1b-model-adapter-report.md`

### Assumptions

- ACE-Step's eventual output format will be 44.1 kHz stereo WAV (this
  drives sample-rate / channel decisions). Mock matches.
- Python ≥3.11 is available on the developer's PATH. A `uv`-managed
  per-project venv lands in Phase 1b2.
- `Generator` is synchronous. An async wrapper is the backend's job in
  Phase 2; keeping the trait sync means `core` stays runtime-free.
- One subprocess per `SubprocessGenerator` instance, held open across
  multiple `generate()` calls. The cost of spawning Python (~200 ms)
  dominates ACE-Step inference cost and is paid once.
- The mock engine's job is *only* to verify the IPC contract.
  Acoustic quality is irrelevant; a 440 Hz sine is chosen because it's
  trivially aurally identifiable if anyone runs a smoke listen.

### Out of scope (deferred)

- Real ACE-Step inference — **Phase 1b2**
- HF resumable model download — **Phase 1b2**
- `uv`-managed Python venv + `pyproject.toml` for ACE-Step — **Phase 1b2**
- Amplitude modulation DSP — **Phase 1c**
- Audio playback — **Phase 1d**
- Backend HTTP/gRPC server / async wrapper — **Phase 2**
- Multiple concurrent generators / pooling — never (one model loaded
  at a time, one in-flight request at a time per process)

### Open questions (resolved 2026-04-27)

1. **IPC format** — *Newline-delimited JSON over stdio.* Debuggable
   (`cat | python3 | jq`), no extra deps; ACE-Step latency dwarfs IPC cost.
2. **Audio interchange** — *Temp WAV file.* Python writes, returns path;
   Rust reads, deletes. `Drop` handles cleanup. ~6 MB per track.
3. **Sample rate** — *44.1 kHz.* Music-industry default; ACE-Step output;
   what most users' tools expect.
4. **Channels** — *Stereo.* ACE-Step default and user expectation. AM
   modulation applies per-sample so the DSP stays channel-agnostic.

---

## Technical approach

### Scope

Create:
- `core/src/audio.rs` — `WavBuffer` type
- `core/src/generator.rs` — `Generator` trait + `GeneratorError`
- `model-adapter/Cargo.toml`
- `model-adapter/src/lib.rs` — module wiring + re-exports
- `model-adapter/src/protocol.rs` — `Request`, `Response` IPC types
- `model-adapter/src/subprocess.rs` — `SubprocessGenerator`
- `model-adapter/python/telaradio_subprocess.py` — IPC main loop + mock
- `model-adapter/python/pyproject.toml` — ruff + ty config
- `model-adapter/tests/protocol_serde.rs` — IPC type round-trip
- `model-adapter/tests/end_to_end.rs` — full pipeline
- `model-adapter/README.md` — append "Implemented now" section

Modify:
- `Cargo.toml` — add `model-adapter` workspace member
- `core/src/lib.rs` — re-export `Generator`, `GeneratorError`,
  `WavBuffer`
- `ROADMAP.md` — mark Phase 1 item 3 as `[~]` while in progress, then
  `[x]` on report
- `.docs/ai/{current-state,next-steps,decisions}.md`

### Steps

1. Add `model-adapter` to workspace members.
2. (TDD) Write a failing test for `WavBuffer` invariants (samples
   length matches `duration * rate * channels`).
3. Implement `WavBuffer` minimally to pass.
4. (TDD) Write a failing test for `Generator` trait shape (using a
   trivial in-memory implementation). Implement trait.
5. Bootstrap `model-adapter/Cargo.toml` and module skeletons.
6. (TDD) Write protocol round-trip tests in
   `tests/protocol_serde.rs`. Implement `Request` / `Response`.
7. Write Python `telaradio_subprocess.py` (mock 440 Hz sine engine,
   NDJSON loop). Add Python tests if a tiny pytest helper is justified.
8. (TDD) Write `end_to_end.rs` that asserts pipeline shape. Implement
   `SubprocessGenerator` until green.
9. Run Rust quality gates; fix until clean.
10. Run Python quality gates (ruff, ty); fix until clean.
11. Update READMEs, handoff docs, and write the phase report.
12. Commit, push.

### Verification

- `cargo test` — protocol round-trip + end-to-end (the e2e test
  spawns Python; it's gated only if `python3` is missing on PATH)
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --check`
- `ruff check model-adapter/python/`
- `ty check model-adapter/python/`
- Manual smoke: `python3 model-adapter/python/telaradio_subprocess.py`,
  paste a Request JSON, verify a Response with a temp WAV path; open
  the WAV in any audio tool and confirm a 440 Hz sine.
