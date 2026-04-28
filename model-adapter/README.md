# model-adapter/

Rust crate (`telaradio-model-adapter`). Implements the `Generator`
trait (defined in `core::generator`) by spawning and managing a Python
subprocess.

```rust
pub trait Generator {
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn generate(&self, prompt: &str, seed: u64, duration: u32)
        -> Result<WavBuffer, GeneratorError>;
}
```

## Implemented (Phase 1b)

- `protocol::Request` and `protocol::Response` — the NDJSON-over-stdio
  IPC types between Rust and the Python subprocess. Audio crosses the
  boundary by temp file path.
- `subprocess::SubprocessGenerator` — spawns `python3 <script>`, holds
  it open across multiple `generate` calls, kills + reaps in `Drop`.
  Currently the `mock-sine` generator (id: `"mock-sine"`,
  version: `"0.1.0"`) returning a 440 Hz sine wave.
- `python/telaradio_subprocess.py` — stdlib-only NDJSON loop with the
  mock engine.

```bash
cargo test -p telaradio-model-adapter
cd python && uv run --with ruff ruff check . && uv run --with ty ty check .
```

## Implemented (Phase 1b2)

- `ace_step::AceStepGenerator` (id: `"ace-step-v1-3.5b"`,
  version: `"1.0.0"`) — real ACE-Step inference via
  `python/telaradio_ace_step.py` running in the project venv. Same
  NDJSON IPC as the mock; the engine swap is invisible to callers.
- `ipc::IpcChannel` (private) — shared NDJSON-over-stdio plumbing
  composed by both generators.
- `hf_download::download_with_resume` — synchronous resumable HTTP
  downloader with `Range` header resume, sha256 validation, an
  optional progress callback, and a `CancellationToken`. Pure Rust
  (`reqwest` + `rustls-tls` + `sha2`).
- `model_install::ensure_model` — installs ACE-Step weights from
  Hugging Face (`InstallMode::Download`) or copies from a
  user-supplied directory (`InstallMode::UseExisting`). Idempotent
  via a `manifest.json` of per-artifact sha256s.
- `model_install::prompt_install_mode_cli` — reader/writer-generic
  one-line stdin parser for first-launch UX (until Phase 1d ships
  a real UI).
- `python/telaradio_ace_step.py` — ACE-Step subprocess. Lazy pipeline
  load on first request; `--probe` prints engine version with no
  model load.
- `python/pyproject.toml` — real uv project. Dependencies: `ace-step`
  (pinned to GitHub commit `1bee4c9f` because the PyPI sdist is broken
  upstream), `huggingface-hub`. Dev deps: `ruff`, `ty`. The mock
  subprocess runs from this same venv unchanged.

```bash
# Rust
cargo test -p telaradio-model-adapter
cargo test -p telaradio-model-adapter -- --include-ignored  # opt in to e2e

# Python
cd python && uv sync && uv run ruff check . && uv run ty check .
uv run python telaradio_ace_step.py --probe   # prints engine metadata
```

## Planned

Future generators (MusicGen, YuE, ...) live alongside the existing
ones; recipes pin `model.id + model.version` for reproducibility.
