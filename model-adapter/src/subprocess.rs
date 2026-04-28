//! `SubprocessGenerator` — Phase 1b mock-sine generator. Spawns a Python
//! child process speaking the NDJSON IPC protocol from `protocol.rs` and
//! ships a 440 Hz sine.
//!
//! Phase 1b2 introduces `AceStepGenerator` as a parallel impl. Both
//! types share the IPC machinery in `crate::ipc`.

use std::path::Path;

use telaradio_core::audio::WavBuffer;
use telaradio_core::generator::{Generator, GeneratorError};

use crate::ipc::IpcChannel;

/// Stable id under which Phase 1b's mock surfaces in `recipe.model.id`.
pub const MOCK_GENERATOR_ID: &str = "mock-sine";
pub const MOCK_GENERATOR_VERSION: &str = "0.1.0";

pub struct SubprocessGenerator {
    channel: IpcChannel,
}

impl SubprocessGenerator {
    /// Spawn `python3 <script>` and prepare the IPC pipes. The script
    /// must implement the NDJSON protocol described in `protocol.rs`.
    pub fn spawn(script: &Path) -> Result<Self, GeneratorError> {
        let channel = IpcChannel::spawn(Path::new("python3"), script, &[])?;
        Ok(Self { channel })
    }
}

impl Generator for SubprocessGenerator {
    fn id(&self) -> &str {
        MOCK_GENERATOR_ID
    }

    fn version(&self) -> &str {
        MOCK_GENERATOR_VERSION
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
