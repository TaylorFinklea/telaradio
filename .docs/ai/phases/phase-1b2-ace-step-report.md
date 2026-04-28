# Phase Report: Phase 1b2 — Real ACE-Step + HF Model Download

**Date:** 2026-04-28
**Outcome:** pass (with one upstream caveat — see Caveats)
**Spec:** [`phase-1b2-ace-step-spec.md`](phase-1b2-ace-step-spec.md)
**Worktree:** `/Users/tfinklea/git/telaradio-phase-1b2` on branch
`phase-1b2`. Not pushed. Parent session merges.

## Summary

Real ACE-Step generation lives behind the same `Generator` trait as
the mock-sine engine; both subprocesses share a small private IPC
helper. First-launch model resolution is implemented in pure Rust:
resumable HTTP from Hugging Face (`Range`-based resume + sha256
validation + cancel token + progress callback), or copy from a
user-supplied weights directory. A small CLI prompt parses one line of
stdin to pick the mode. All of it is TDD-covered with `httpmock` —
no real network in tests.

The Python project is now a real uv-managed venv with `ace-step` and
`huggingface-hub` deps, lockfile committed.

## Changes

### New (Rust)

- `model-adapter/src/ace_step.rs` — `AceStepGenerator` (id
  `ace-step-1.5-xl`, version `1.5.0`) with `spawn(model_dir)` and
  `spawn_with_script(script)` constructors
- `model-adapter/src/hf_download.rs` — `download_with_resume`,
  `sha256_file`, `CancellationToken`, `DownloadError`,
  `ProgressCallback`
- `model-adapter/src/model_install.rs` — `ensure_model`,
  `prompt_install_mode_cli`, `InstallMode` (`Download` /
  `UseExisting`), `ModelArtifact`, `ModelInstallError`
- `model-adapter/src/ipc.rs` (private) — shared `IpcChannel` that
  both `SubprocessGenerator` and `AceStepGenerator` compose
- `model-adapter/tests/hf_download_test.rs` — 5 tests (full download,
  resume from partial, checksum mismatch, pre-cancelled token,
  monotone progress)
- `model-adapter/tests/model_install_test.rs` — 8 tests (download +
  manifest, idempotency, use-existing, corrupt re-download, three
  prompt parser cases, missing-source error)
- `model-adapter/tests/ace_step_smoke.rs` — 3 mocked smoke tests for
  the `AceStepGenerator` Rust contract (no model needed)
- `model-adapter/tests/ace_step_e2e.rs` — 1 `#[ignore]`d real-model
  integration test, opt-in via `--include-ignored`

### New (Python)

- `model-adapter/python/telaradio_ace_step.py` — ACE-Step variant of
  the subprocess. Same NDJSON IPC as the mock. Lazy pipeline load on
  first request; `--probe` prints engine version metadata without
  touching torch
- `model-adapter/python/uv.lock` — committed for reproducible installs

### Modified

- `Cargo.toml` — `[workspace.dependencies]` adds `reqwest` (rustls-tls,
  blocking, stream), `sha2`, `httpmock`
- `model-adapter/Cargo.toml` — depends on the new workspace deps;
  `httpmock` and `tempfile` in `[dev-dependencies]`
- `model-adapter/src/lib.rs` — re-exports `AceStepGenerator`,
  `ACE_STEP_GENERATOR_ID`, `ACE_STEP_GENERATOR_VERSION`; adds
  `hf_download`, `model_install` public modules and `ipc` private
  module
- `model-adapter/src/subprocess.rs` — slimmed to delegate to
  `IpcChannel` (no behavior change to the public API)
- `model-adapter/python/pyproject.toml` — real uv project with
  `[project.dependencies] = ["ace-step", "huggingface-hub>=0.20"]`,
  `[dependency-groups] dev = ["ruff", "ty"]`,
  `[tool.uv.sources]` pinning `ace-step` to a GitHub commit
- `.gitignore` — stop ignoring `**/uv.lock`

### Pending (will land in this same commit chain)

- `model-adapter/README.md` — append "Implemented (Phase 1b2)" section
- `ROADMAP.md` — items 3 and 5 of Phase 1 → `[x]`
- `.docs/ai/current-state.md`, `next-steps.md`, `decisions.md` —
  Phase 1b2 entries

## Verification results

All quality gates run from the worktree root.

```text
$ cargo test
... 45 passed; 0 failed; 1 ignored ...
```

Test counts per file:
- `core/tests/audio.rs`: 4
- `core/tests/generator.rs`: 2
- `core/tests/recipe.rs`: 14
- `model-adapter/tests/ace_step_e2e.rs`: 0 passed, 1 ignored
- `model-adapter/tests/ace_step_smoke.rs`: 3
- `model-adapter/tests/end_to_end.rs`: 4
- `model-adapter/tests/hf_download_test.rs`: 5
- `model-adapter/tests/model_install_test.rs`: 8
- `model-adapter/tests/protocol_serde.rs`: 5

```text
$ cargo clippy --all-targets -- -D warnings
... clean (pedantic) ...

$ cargo fmt --check
... clean ...

$ cd model-adapter/python && uv sync
... resolved 171 packages, all installed ...

$ uv run ruff check .
All checks passed!

$ uv run ty check .
All checks passed!

$ uv run python telaradio_ace_step.py --probe
{"engine": "ace-step", "version": "1.5.0"}
```

The `#[ignore]`d e2e test was *not* run — the real ACE-Step model
checkpoint is ~5 GB and lives outside the worktree. The user can opt
in via `TELARADIO_MODEL_DIR=/path/to/weights cargo test -- --include-ignored
ace_step_e2e` after the merge.

## Decisions / assumptions

1. **ACE-Step pip distribution name is `ace-step`; Python module is
   `acestep`.** The PyPI package exists (Apache-2.0); see Caveats for
   why we install from a git commit pin instead of the sdist.
2. **Model is `ACE-Step/ACE-Step-v1-3.5B` on Hugging Face**, not the
   "1.5 XL" name in the spec. The spec's terminology was a guess made
   before the package was inspected. We kept the constant
   `ACE_STEP_GENERATOR_ID = "ace-step-1.5-xl"` because that's the
   stable id a recipe might pin against, and the version string maps
   to ACE-Step's release rather than the parameter count. If we want
   the id to track the actual checkpoint name, that's a recipe-schema
   migration for a later phase.
3. **`AceStepGenerator` is a separate impl, not a parameterized
   variant of `SubprocessGenerator`.** Two trait impls, two ids. The
   shared NDJSON plumbing lives in a private `ipc::IpcChannel` so the
   duplication is structural, not behavioral.
4. **`#[ignore]` for the real round-trip.** Confirmed compiles; not
   run in CI. The mock-script smoke tests cover the Rust IPC contract.
5. **Reqwest blocking + rustls-tls.** No openssl on the dependency
   surface. Async `reqwest` would have forced a tokio runtime onto
   the otherwise-sync `Generator` trait; the spec defers async to
   Phase 2.
6. **Manifest = single `manifest.json` of `[ModelArtifact]`.** Cheaper
   than per-file checksums in a sidecar, easier to reason about than
   a manifest-per-artifact scheme.
7. **`UseExisting` copies, doesn't symlink.** Predictable, works
   regardless of source filesystem (network mount, foreign volume),
   and the install dir's manifest stays valid even if the source
   dir later disappears. ~5 GB extra disk is acceptable.
8. **`prompt_install_mode_cli` is reader/writer-generic** so it can
   be exercised in tests without manipulating `stdin`/`stderr`. Real
   callers pass `&mut std::io::stdin().lock()` and
   `&mut std::io::stderr()`.
9. **Lazy pipeline load in Python.** The subprocess starts cheap (no
   torch import on `--probe`) and only loads the model on the first
   real request. Lets the Rust caller spawn the subprocess
   speculatively without paying the model-load cost.

## Caveats

- **ACE-Step PyPI sdist is broken.** `pip install ace-step` fails
  because `setup.py` reads `requirements.txt` at build time and that
  file isn't included in the sdist tarball
  (`FileNotFoundError: requirements.txt`). We work around it by
  pinning the GitHub repo via `[tool.uv.sources]` to commit
  `1bee4c9f5b43e30995f8d4d33b3919197ce1bd68`, which matches main as of
  2026-02-15 and contains the requirements.txt. When upstream
  re-publishes, switch back to a plain PyPI dep. *(Spec risk note had
  this exact fallback path; flagging here per protocol.)*
- **`uv sync` is heavy on first install.** ~170 transitive deps,
  multi-GB venv (torch + transformers + diffusers + spacy with the
  Japanese unidic dict + …). Subsequent `uv sync --frozen` runs are
  fast.
- **Apple Silicon / MPS.** The torch wheel resolution picked
  `torch==2.11.0` for darwin-aarch64. We did not run the model to
  verify MPS pickup; that is for the user to confirm post-merge with
  the e2e test.
- **The 1.5 XL → 3.5B naming gap** in the spec vs. the actual model
  is documented above (decision 2). Not a blocker but worth flagging
  before recipes get authored.
- **`unidic-lite`-based Japanese tokenizer** ships in the venv even
  though we don't use Japanese yet. ACE-Step's own dependency tree
  pulls it in. We could narrow the deps in a follow-up if the venv
  size becomes a problem.

## What changed in handoff docs

- `current-state.md` — Phase 1b2 marked complete; build status / test
  counts updated; "what does NOT exist yet" trimmed.
- `next-steps.md` — Phase 1b2 entries removed; recommendation updated
  to point at Phase 1c (in parallel) or 1d.
- `decisions.md` — appended a 2026-04-28 "Phase 1b2" section
  capturing the nine decisions above, plus the upstream sdist-broken
  workaround.
- `ROADMAP.md` — Phase 1 item 3 → `[x]` (no longer `[~]`); item 5
  ("First-launch model download") → `[x]`. Item 4 (DSP) is being
  built in parallel as Phase 1c.

## Coordination with Phase 1c

Phase 1c (AM modulation DSP) is being built in parallel in a sibling
worktree. Both phases edit the workspace `Cargo.toml` (members list /
`[workspace.dependencies]`) and the same handoff docs. Resolution
plan when the parent session merges: keep both sets of dependency
additions; `members` should end up as `["core", "dsp", "model-adapter"]`
with the dsp crate from 1c included.
