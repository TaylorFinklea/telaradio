//! `AceStepGenerator` — Phase 1b2 real ACE-Step v1 3.5B generator.
//!
//! Spawns the Python subprocess at `model-adapter/python/telaradio_ace_step.py`
//! using the uv-managed venv at `model-adapter/python/.venv/bin/python`.
//! Same NDJSON IPC contract as the mock: write a JSON `Request` per
//! line, read a JSON `Response` per line. ACE-Step does the heavy work
//! on the Python side; on the Rust side, this is just a `Generator`
//! impl that delegates to `IpcChannel`.
//!
//! ## Test strategy
//!
//! Lightweight smoke tests (`tests/ace_step_smoke.rs`) point this
//! generator at the *mock* Python script so the Rust id/version/IPC
//! contract gets CI coverage without ACE-Step installed.
//!
//! The real ACE-Step round-trip lives in `tests/ace_step_e2e.rs` and is
//! `#[ignore]`d — it needs the model checkpoint (~7.7 GB, see
//! [`ACE_STEP_TOTAL_BYTES`]) plus a working torch / transformers install.
//! Opt in with `cargo test -- --include-ignored`.

use std::path::{Path, PathBuf};

use telaradio_core::audio::WavBuffer;
use telaradio_core::generator::{Generator, GeneratorError};

use crate::ipc::IpcChannel;
use crate::model_install::ModelArtifact;

/// Stable id under which Phase 1b2's ACE-Step engine surfaces in
/// `recipe.model.id`. Distinct from `mock-sine` so a recipe pinning
/// either generator routes unambiguously.
pub const ACE_STEP_GENERATOR_ID: &str = "ace-step-v1-3.5b";
pub const ACE_STEP_GENERATOR_VERSION: &str = "1.0.0";

/// Canonical artifact list for ACE-Step v1 3.5B from Hugging Face
/// (`ACE-Step/ACE-Step-v1-3.5B`). Each file lives at a known relative path
/// inside the checkpoint root.
///
/// sha256 values for the LFS-backed `*.safetensors` and `tokenizer.json`
/// files come from HF's `?blobs=true` API (the `lfs.sha256` field). Plain
/// JSON configs were downloaded once and hashed locally. Total uncompressed
/// footprint is ~7.7 GB — see `ACE_STEP_TOTAL_BYTES`.
///
/// Re-verify if the upstream repo is updated.
pub fn ace_step_artifacts() -> &'static [ModelArtifact] {
    static ARTIFACTS: std::sync::OnceLock<Vec<ModelArtifact>> = std::sync::OnceLock::new();
    ARTIFACTS.get_or_init(|| {
        let base = "https://huggingface.co/ACE-Step/ACE-Step-v1-3.5B/resolve/main";
        vec![
            ModelArtifact {
                url: format!("{base}/music_dcae_f8c8/config.json"),
                relative_path: "music_dcae_f8c8/config.json".into(),
                sha256: "b14a49a8c52a52c2c8050098af1a946810a8a1a0b6e50abc75ba81371383cf04".into(),
            },
            ModelArtifact {
                url: format!("{base}/music_dcae_f8c8/diffusion_pytorch_model.safetensors"),
                relative_path: "music_dcae_f8c8/diffusion_pytorch_model.safetensors".into(),
                sha256: "2b0cb469307ac50659d1880db2a99bae47d0df335cbb36853964662d4b80e8ee".into(),
            },
            ModelArtifact {
                url: format!("{base}/music_vocoder/config.json"),
                relative_path: "music_vocoder/config.json".into(),
                sha256: "39ddc4c417e01dc3be1862fcf315887358dd61b7f91c3cc8227f65072984bb55".into(),
            },
            ModelArtifact {
                url: format!("{base}/music_vocoder/diffusion_pytorch_model.safetensors"),
                relative_path: "music_vocoder/diffusion_pytorch_model.safetensors".into(),
                sha256: "c92c9b46e28ab7b37b777780cf4308ad7ddac869636bb77aa61599358c4bc1c0".into(),
            },
            ModelArtifact {
                url: format!("{base}/ace_step_transformer/config.json"),
                relative_path: "ace_step_transformer/config.json".into(),
                sha256: "4d78beb6afb4c7f3705256b44faaf60f3e1e2d78f4015ca87740f3695d7f5447".into(),
            },
            ModelArtifact {
                url: format!("{base}/ace_step_transformer/diffusion_pytorch_model.safetensors"),
                relative_path: "ace_step_transformer/diffusion_pytorch_model.safetensors".into(),
                sha256: "e810f16728d8a2e0d1b9c3a907aac8c9a427ce38edbd890cb3dce5ff92da5aad".into(),
            },
            ModelArtifact {
                url: format!("{base}/umt5-base/config.json"),
                relative_path: "umt5-base/config.json".into(),
                sha256: "afae5da9a35e2b293cee66536f03fc2581cfc3f3d5707d3a262b552748de1572".into(),
            },
            ModelArtifact {
                url: format!("{base}/umt5-base/model.safetensors"),
                relative_path: "umt5-base/model.safetensors".into(),
                sha256: "779cec0d210b2123e21d0a9cd8128f02b4d412627355028965a8be0b241cc3b6".into(),
            },
            ModelArtifact {
                url: format!("{base}/umt5-base/tokenizer.json"),
                relative_path: "umt5-base/tokenizer.json".into(),
                sha256: "20a46ac256746594ed7e1e3ef733b83fbc5a6f0922aa7480eda961743de080ef".into(),
            },
            ModelArtifact {
                url: format!("{base}/umt5-base/special_tokens_map.json"),
                relative_path: "umt5-base/special_tokens_map.json".into(),
                sha256: "456b58fd240a06c743a7c2cf8008bec501240d68ebd1fc4018ea569505fea270".into(),
            },
            ModelArtifact {
                url: format!("{base}/umt5-base/tokenizer_config.json"),
                relative_path: "umt5-base/tokenizer_config.json".into(),
                sha256: "ed9a3a8b0faa71a70a32847e0435fe036e6e112d4df4edb7bb48a921e344dc05".into(),
            },
        ]
    })
}

/// Sum of `content-length` for all `ace_step_artifacts()` (~7.7 GB).
/// Source of truth for download progress UIs that need a total.
pub const ACE_STEP_TOTAL_BYTES: u64 = 8_275_790_207;

pub struct AceStepGenerator {
    channel: IpcChannel,
}

impl AceStepGenerator {
    /// Spawn the ACE-Step subprocess from the canonical
    /// `model-adapter/python/.venv` next to the default script. This is
    /// the production constructor. Pass `model_dir` so the subprocess
    /// knows where to load weights from; it is communicated via the
    /// `TELARADIO_MODEL_DIR` env var on the child.
    ///
    /// # Errors
    ///
    /// Propagates [`GeneratorError`] for spawn / IPC failures.
    pub fn spawn(model_dir: &Path) -> Result<Self, GeneratorError> {
        let venv_python = default_venv_python();
        let script = default_ace_step_script();

        let mut command = std::process::Command::new(&venv_python);
        command
            .arg(&script)
            .env("TELARADIO_MODEL_DIR", model_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let channel = IpcChannel::spawn_with_command(command)?;
        Ok(Self { channel })
    }

    /// Spawn against an arbitrary script (no venv assumption, no env
    /// var). Used by smoke tests to point this generator at the mock
    /// script so the IPC contract gets coverage without ACE-Step
    /// installed.
    ///
    /// # Errors
    ///
    /// Propagates [`GeneratorError`] for spawn / IPC failures.
    pub fn spawn_with_script(script: &Path) -> Result<Self, GeneratorError> {
        let channel = IpcChannel::spawn(Path::new("python3"), script, &[])?;
        Ok(Self { channel })
    }
}

impl Generator for AceStepGenerator {
    fn id(&self) -> &str {
        ACE_STEP_GENERATOR_ID
    }

    fn version(&self) -> &str {
        ACE_STEP_GENERATOR_VERSION
    }

    fn generate(
        &self,
        prompt: &str,
        seed: u64,
        duration_seconds: u32,
    ) -> Result<WavBuffer, GeneratorError> {
        self.channel.generate(prompt, seed, duration_seconds)
    }
}

fn default_venv_python() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/.venv/bin/python")
}

fn default_ace_step_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/telaradio_ace_step.py")
}
