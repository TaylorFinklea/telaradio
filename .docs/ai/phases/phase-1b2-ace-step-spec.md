# Phase Spec: Phase 1b2 — Real ACE-Step + HF Model Download

**Roadmap item:** Phase 1, item 3 (Python ACE-Step subprocess wrapper +
first-launch model download) — second half (real ACE-Step; the mock
shipped in Phase 1b)
**Date:** 2026-04-28
**Status:** ready to build

## Product

**Goal:** Real ACE-Step 1.5 XL inference replaces the mock engine, behind
the same `Generator` trait. First launch resolves the model: either
download it from Hugging Face (resumable HTTP) or import an existing
file the user already has on disk.

After this slice, a `Recipe` with `model.id = "ace-step-1.5-xl"` actually
generates real music; the mock-sine generator stays available for fast
tests that don't need the model.

### Acceptance criteria

- [ ] New `model-adapter::AceStepGenerator` implementing the `Generator`
      trait, alongside (not replacing) `SubprocessGenerator`
  - `id() = "ace-step-1.5-xl"`, `version() = "1.5.0"`
  - Spawns a Python subprocess running ACE-Step (same NDJSON IPC
    protocol as Phase 1b)
- [ ] `model_install` module: `ensure_model(install_dir,
      mode: InstallMode) -> Result<PathBuf>`
  - `InstallMode::Download` — resumable HTTP from Hugging Face;
    progress callback for future UI
  - `InstallMode::UseExisting(PathBuf)` — copy or symlink user-supplied
    weights into the canonical install dir
  - Idempotent: subsequent launches skip work if the model is already
    present and validates (sha256 of a manifest file is sufficient)
- [ ] `hf_download` module: resumable HTTP client (pure Rust via
      `reqwest` is preferred; falling back to invoking `huggingface_hub`
      from Python is acceptable if it saves complexity)
  - Resumes partial downloads via `Range` header
  - Validates checksum on completion
  - Cancellable (a `CancellationToken` parameter; not exercised in v1
    but the API is ready for a UI)
- [ ] First-launch CLI prompt (no UI yet): when `ensure_model` is called
      and the model is missing, prompt on stdin/stderr —
      `"download" | "use existing <path>"` (one line). UI surface lands
      in Phase 1d.
- [ ] `model-adapter/python/telaradio_ace_step.py`: ACE-Step variant of
      the subprocess. Speaks the same NDJSON protocol. Imports
      `acestep` (or the equivalent module name) and calls inference for
      each request.
- [ ] `model-adapter/python/pyproject.toml` becomes a real uv project:
      `[project.dependencies]` includes `acestep` (or whatever the pip
      name is) and `huggingface_hub` if used; `[dependency-groups]` has
      `dev = ["ruff", "ty"]`. The mock script `telaradio_subprocess.py`
      stays runnable from this same env.
- [ ] TDD coverage:
  - `model_install` unit tests with a temp dir + a fake HF server
    (httpmock or similar) — no real network
  - `hf_download` unit tests for resume behavior (mid-download
    interrupt → resume → completes)
  - Sha256 validation tests (corrupt file → error)
  - "Use existing" path test (point at a fake model file → it appears
    at the canonical install location)
  - `AceStepGenerator` end-to-end integration test marked `#[ignore]`
    (needs the real model; opt in with `cargo test -- --include-ignored`)
- [ ] Quality gates:
  - `cargo test` green (skipping `#[ignore]`)
  - `cargo clippy --all-targets -- -D warnings` clean
  - `cargo fmt --check` clean
  - `cd model-adapter/python && uv sync && uv run ruff check . && uv run ty check .` all clean
- [ ] Phase report at `.docs/ai/phases/phase-1b2-ace-step-report.md`

### Assumptions

- The ACE-Step Python package exists on PyPI with stable name and
  Apache-licensed weights on Hugging Face. If the package isn't on PyPI,
  the spec falls back to vendoring or a git dependency — flag it in the
  report and proceed with a placeholder that imports the right module.
- The download is gated behind `ensure_model` so existing tests that
  use `SubprocessGenerator` (the mock) keep running offline. CI doesn't
  need ACE-Step.
- Apple Silicon is the dev target; ACE-Step's CPU/MPS path is what we
  test against. CUDA is best-effort and out of scope for v1b2.
- Python venv lives at `model-adapter/python/.venv` (gitignored). The
  Rust subprocess invocation uses the venv's `python` binary.

### Out of scope (deferred)

- Model variant selection in UI (XL only for v1; abstraction supports
  it later)
- Download cancellation UI / progress display (Phase 1d)
- Model upgrade flow when ACE-Step releases a new version (Phase 2+)
- CUDA-specific tuning
- Watch HR adaptation, session timers, anything not strictly inference
- A graceful fallback when the model file is corrupt mid-session (just
  re-run `ensure_model` once)

### Open questions (resolved 2026-04-28)

1. **Engine selection**: separate `AceStepGenerator` class alongside
   the existing `SubprocessGenerator`. Two trait impls, two ids. Cleaner
   than parameterizing one subprocess.
2. **Test gating**: e2e ACE-Step test is `#[ignore]` by default
   (requires ~5 GB and a GPU/MPS-capable machine).
3. **Existing-file install path**: yes, support `InstallMode::UseExisting`.
4. **Python venv**: full uv-managed project under `model-adapter/python/`.

---

## Technical approach

### Scope

Create:
- `model-adapter/src/ace_step.rs` — `AceStepGenerator` impl
- `model-adapter/src/model_install.rs` — `InstallMode`, `ensure_model`,
  CLI prompt helper, sha256 validation
- `model-adapter/src/hf_download.rs` — resumable HTTP, progress
  callback, cancellation token
- `model-adapter/python/telaradio_ace_step.py` — ACE-Step engine
  variant of the subprocess (same NDJSON protocol)
- `model-adapter/tests/model_install_test.rs` — temp-dir + fake HF
  server tests
- `model-adapter/tests/ace_step_e2e.rs` — `#[ignore]` integration
- `.docs/ai/phases/phase-1b2-ace-step-report.md`

Modify:
- `Cargo.toml`: add `reqwest` (with `rustls-tls` feature, no openssl
  surface), `sha2`, `tokio` (for async download); add `httpmock` to
  dev-deps
- `model-adapter/Cargo.toml`: depend on the new workspace deps
- `model-adapter/src/lib.rs`: re-export `AceStepGenerator`,
  `model_install::*`
- `model-adapter/python/pyproject.toml`: replace minimal config with
  a real uv project — `[project.dependencies] = ["acestep", ...]`
- `model-adapter/README.md`: append "Implemented (Phase 1b2)" section
- `ROADMAP.md`: mark items 3+5 (subprocess + model download) as `[x]`
  on report
- `.docs/ai/current-state.md`, `next-steps.md`, `decisions.md`

### First-launch flow

```
fn ensure_model(install_dir: &Path, mode: InstallMode)
    -> Result<PathBuf, ModelInstallError>
{
    let manifest = install_dir.join("manifest.json");
    if manifest.exists() && validate(&manifest).is_ok() {
        return Ok(install_dir.to_owned());  // already installed
    }

    create_dir_all(install_dir)?;
    match mode {
        InstallMode::Download(progress_cb) => {
            hf_download::download_with_resume(
                ACE_STEP_HF_URL,
                install_dir,
                progress_cb,
            )?;
        }
        InstallMode::UseExisting(src) => {
            copy_model_files(&src, install_dir)?;
        }
    }
    write_manifest(install_dir)?;
    Ok(install_dir.to_owned())
}

fn prompt_install_mode_cli() -> InstallMode {
    eprintln!("Telaradio needs the ACE-Step 1.5 XL model (~5 GB).");
    eprintln!("  [1] Download from Hugging Face");
    eprintln!("  [2] Use an existing file (give path)");
    // ... read stdin, return appropriate variant
}
```

### Steps (TDD discipline mandatory)

1. **Python venv first.** Update `model-adapter/python/pyproject.toml`
   to a real uv project with ACE-Step dep. Run `uv sync` to verify it
   resolves; commit the resulting `uv.lock`.
2. Write a smoke `telaradio_ace_step.py` that imports the package and
   prints the version (no model load yet). Verify `uv run python
   telaradio_ace_step.py --probe` prints the version.
3. Write `model_install` failing tests (against a temp dir + httpmock
   fake HF server). Implement `model_install` until GREEN.
4. Write `hf_download` failing tests for the resume case. Implement.
5. Write sha256 validation failing tests. Implement.
6. Wire `AceStepGenerator` to spawn `python telaradio_ace_step.py`
   from the venv with the resolved model path. Use the same NDJSON
   protocol as `SubprocessGenerator`.
7. Write the `#[ignore]` e2e test. Don't actually run it without the
   model; just verify it compiles and the test passes when the model
   is present.
8. Run quality gates (Rust + Python). Fix.
9. Update handoff docs and write the phase report.
10. Commit but **DO NOT PUSH** — parent session merges.

### Verification

- `cargo test --workspace` — green (skips `#[ignore]`)
- `cargo test -- --include-ignored` — fails or runs depending on whether
  the model is installed; report should note this
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt --check` — clean
- `cd model-adapter/python && uv sync && uv run ruff check . && uv run ty check .` — clean
- Manual smoke (after merge, by user): set `TELARADIO_MODEL_DIR=/tmp/test`
  and run a binary that calls `ensure_model` in `Download` mode against a
  small test artifact — but the *real* ACE-Step download is for the user
  to do once after the phase merges

### Risk notes

- **ACE-Step package availability:** if the package name on PyPI isn't
  obvious, fall back to a git source in `pyproject.toml`. Document the
  resolution in the phase report.
- **uv sync slowness:** ACE-Step pulls torch + transformers; first
  `uv sync` is heavy. Don't block on it during dev; use `uv sync
  --no-install-project --frozen` after the lockfile is committed.
- **Apple Silicon / MPS:** ACE-Step's torch wheel selection should
  pick up MPS on M-series. If it falls back to CPU, the e2e test
  takes minutes instead of ~10 seconds. Note in report.

### Commit conventions

- `feat(model-adapter): real ACE-Step generator (phase 1b2)` for the main commit
- Smaller commits along the way (`feat(model-adapter/python): uv project`,
  `feat(model-adapter): hf download resumer`, etc.) are welcome
- Trailing footer: `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`

### Coordination with Phase 1c

Phase 1c lands first. This phase rebases on top of it. Both modify
workspace `Cargo.toml` (members list, `[workspace.dependencies]`) and
the handoff docs — expect minor merge conflicts; resolve by keeping
both sets of changes.
