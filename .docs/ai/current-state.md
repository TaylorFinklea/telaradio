# Current state

**Date**: 2026-04-28
**Phase**: Phase 1b2 (real ACE-Step + HF model download) complete on
branch `phase-1b2`. Phase 1c (AM modulation DSP) is being built in
parallel on `phase-1c`. Both branches expect the parent session to
merge.
**Build status**: `cargo test` green (45 passed / 1 ignored across
audio, generator, recipe, ace_step_smoke, end_to_end (mock),
hf_download, model_install, protocol_serde). The 1 ignored test is
the real-ACE-Step e2e — opt in with `--include-ignored` and a primed
model dir. `cargo clippy --all-targets -- -D warnings` clean
(pedantic). `cargo fmt --check` clean. Python `uv sync` resolves
171 deps; `uv run ruff check .` and `uv run ty check .` clean.

## Last session summary

Phase 1b2 — real ACE-Step generator + Hugging Face model installer.

- `AceStepGenerator` lives alongside `SubprocessGenerator`. Both
  compose a private `ipc::IpcChannel` so the NDJSON-over-stdio
  plumbing is shared. `AceStepGenerator::spawn(model_dir)` runs
  `model-adapter/python/telaradio_ace_step.py` from the project's
  uv venv with `TELARADIO_MODEL_DIR` exported to the child. Generator
  id: `"ace-step-1.5-xl"`, version: `"1.5.0"`.
- `model_install::ensure_model(install_dir, artifacts, mode)`
  populates a canonical install dir from either Hugging Face
  (`InstallMode::Download` — resumable HTTP via `hf_download`) or a
  user-supplied directory (`InstallMode::UseExisting`). Writes a
  `manifest.json` of per-artifact sha256s; subsequent calls are no-ops
  when validation succeeds.
- `hf_download::download_with_resume` is a synchronous resumable HTTP
  downloader: `Range` header resume, sha256 validation,
  `CancellationToken`, optional progress callback. Pure Rust
  (`reqwest` + `rustls-tls`).
- `prompt_install_mode_cli` parses one line of stdin for the
  first-launch UX (Phase 1d will replace it with a real UI).
- `model-adapter/python` is now a real uv-managed project with
  `ace-step` (git-pinned because the PyPI sdist is broken upstream)
  and `huggingface-hub`. The mock subprocess still runs from this
  venv unchanged.
- TDD coverage with `httpmock` — no real network in any test. Real
  ACE-Step round-trip is `#[ignore]`d.

See [`phases/phase-1b2-ace-step-report.md`](phases/phase-1b2-ace-step-report.md).

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
  - `ace_step::AceStepGenerator` (real ACE-Step)
  - `hf_download::*` — resumable HTTP + sha256
  - `model_install::*` — `ensure_model`, `prompt_install_mode_cli`
  - `ipc::IpcChannel` (private — shared NDJSON plumbing)
  - `python/telaradio_subprocess.py` (mock)
  - `python/telaradio_ace_step.py` (ACE-Step)
  - `python/pyproject.toml` + `python/uv.lock` (real uv project)
- 45 Rust integration tests across 9 test files (1 ignored e2e)
- `recipes/example-foggy-lofi.json` — realistic schema v1 example
- GitHub repo `TaylorFinklea/telaradio` (public)

## Blockers

None.

## What does NOT exist yet

- `dsp/` amplitude modulation — being built in parallel as Phase 1c
- `apple/` macOS Swift app (Phase 1d)
- Background buffer queue (Phase 1e)
- Remaining ~19 starter recipes (Phase 1f)
- Settings UI (Phase 1g)

## Pointers

- [`next-steps.md`](next-steps.md) — exact next actions
- [`decisions.md`](decisions.md) — index of decision records
- [`phases/`](phases/) — phase specs and reports
- [`../../ROADMAP.md`](../../ROADMAP.md) — phases 1–4
