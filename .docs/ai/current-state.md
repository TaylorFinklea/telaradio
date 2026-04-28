# Current state

**Date**: 2026-04-28
**Phase**: Phases 1b2 (real ACE-Step + HF model download) and 1c (AM
modulation DSP) both complete and merged into `main`. Phase 1d (macOS
Swift player shell) not yet started.
**Build status**: `cargo test --workspace` green (45 passed / 1 ignored
across audio, generator, recipe, ace_step_smoke, end_to_end (mock),
hf_download, model_install, protocol_serde, am_apply, dsp_pipeline).
The 1 ignored test is the real-ACE-Step e2e — opt in with
`--include-ignored` once a model dir is primed. `cargo clippy
--all-targets -- -D warnings` clean (pedantic). `cargo fmt --check`
clean. Python `uv sync` resolves 171 deps; `uv run ruff check .` and
`uv run ty check .` clean.

## Last session summary

Two phases shipped in parallel via separate worktrees, merged
sequentially (`phase-1c` first as fast-forward, then `phase-1b2` with a
merge commit).

**Phase 1c — AM modulation DSP.** Bootstrapped the `dsp/` workspace
member (`telaradio-dsp`) with `apply_am(buffer, rate_hz, depth,
envelope) -> WavBuffer`, a pure transform per Woods et al. 2024
§Methods. DSP-side `Envelope` enum (Square / Sine / Triangle) decoupled
from `core::recipe::Envelope`, with a small `From` bridge. Square gate
gets a 1 ms linear crossfade centered on each transition to suppress
audible clicks at high depth. Stereo channels modulated identically
(paper-faithful). 10 new integration tests + 1 end-to-end pipeline
smoke test. See
[`phases/phase-1c-am-modulation-report.md`](phases/phase-1c-am-modulation-report.md).

**Phase 1b2 — real ACE-Step generator + HF installer.**
- `AceStepGenerator` lives alongside `SubprocessGenerator`; both
  compose a private `ipc::IpcChannel` so the NDJSON-over-stdio plumbing
  is shared. `AceStepGenerator::spawn(model_dir)` runs
  `model-adapter/python/telaradio_ace_step.py` from the project's uv
  venv with `TELARADIO_MODEL_DIR` exported. Id: `"ace-step-v1-3.5b"`,
  version: `"1.0.0"`.
- `model_install::ensure_model(install_dir, artifacts, mode)` populates
  a canonical install dir from Hugging Face (`InstallMode::Download` —
  resumable HTTP via `hf_download`) or a user-supplied directory
  (`InstallMode::UseExisting`). Writes a `manifest.json` of per-artifact
  sha256s; subsequent calls are no-ops on validation success.
- `hf_download::download_with_resume`: synchronous resumable HTTP
  downloader with `Range` header resume, sha256 validation, cancellation
  token, optional progress callback. Pure Rust (`reqwest` +
  `rustls-tls`).
- `prompt_install_mode_cli` parses one line of stdin for the
  first-launch UX (Phase 1d will replace it with a real UI).
- `model-adapter/python` is now a real uv-managed project with
  `ace-step` (git-pinned at commit `1bee4c9f` because the PyPI sdist is
  broken upstream) and `huggingface-hub`. The mock subprocess still
  runs from this venv unchanged.
- TDD coverage with `httpmock` — no real network in any test. Real
  ACE-Step round-trip is `#[ignore]`d.

See [`phases/phase-1b2-ace-step-report.md`](phases/phase-1b2-ace-step-report.md).

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
- `telaradio-dsp` crate (`dsp/`):
  - `dsp::Envelope` (Square / Sine / Triangle) + `From<core::recipe::Envelope>`
  - `dsp::apply_am(buffer, rate_hz, depth, envelope) -> WavBuffer`
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
