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

## Planned

- Phase 1b2: `AceStepGenerator` alongside the mock — same trait, real
  ACE-Step 1.5 XL inference behind it. Plus first-launch resumable HTTP
  download from Hugging Face into
  `~/Library/Application Support/Telaradio/models/`.

Future generators (MusicGen, YuE, ...) live alongside the existing
ones; recipes pin `model.id + model.version` for reproducibility.
