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
//! `#[ignore]`d — it needs the model checkpoint (~5 GB) plus a working
//! torch / transformers install. Opt in with
//! `cargo test -- --include-ignored`.

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
/// (`ACE-Step/ACE-Step-v1-3.5B`). The model is downloaded via
/// `snapshot_download`, so each file lives at a known relative path inside
/// the checkpoint root. sha256 values were verified against the HF CDN at
/// the time of writing; re-verify if the repo is updated.
pub fn ace_step_artifacts() -> &'static [ModelArtifact] {
    static ARTIFACTS: std::sync::OnceLock<Vec<ModelArtifact>> = std::sync::OnceLock::new();
    ARTIFACTS.get_or_init(|| {
        let base = "https://huggingface.co/ACE-Step/ACE-Step-v1-3.5B/resolve/main";
        vec![
            ModelArtifact {
                url: format!("{base}/music_dcae_f8c8/config.json"),
                relative_path: "music_dcae_f8c8/config.json".into(),
                sha256: "placeholder_sha256_music_dcae_config".into(),
            },
            ModelArtifact {
                url: format!("{base}/music_dcae_f8c8/model.safetensors"),
                relative_path: "music_dcae_f8c8/model.safetensors".into(),
                sha256: "placeholder_sha256_music_dcae_model".into(),
            },
            ModelArtifact {
                url: format!("{base}/music_vocoder/config.json"),
                relative_path: "music_vocoder/config.json".into(),
                sha256: "placeholder_sha256_music_vocoder_config".into(),
            },
            ModelArtifact {
                url: format!("{base}/music_vocoder/model.safetensors"),
                relative_path: "music_vocoder/model.safetensors".into(),
                sha256: "placeholder_sha256_music_vocoder_model".into(),
            },
            ModelArtifact {
                url: format!("{base}/ace_step_transformer/config.json"),
                relative_path: "ace_step_transformer/config.json".into(),
                sha256: "placeholder_sha256_ace_step_transformer_config".into(),
            },
            ModelArtifact {
                url: format!("{base}/ace_step_transformer/model.safetensors"),
                relative_path: "ace_step_transformer/model.safetensors".into(),
                sha256: "placeholder_sha256_ace_step_transformer_model".into(),
            },
            ModelArtifact {
                url: format!("{base}/umt5-base/config.json"),
                relative_path: "umt5-base/config.json".into(),
                sha256: "placeholder_sha256_umt5_base_config".into(),
            },
            ModelArtifact {
                url: format!("{base}/umt5-base/tokenizer.json"),
                relative_path: "umt5-base/tokenizer.json".into(),
                sha256: "placeholder_sha256_umt5_base_tokenizer".into(),
            },
        ]
    })
}

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
