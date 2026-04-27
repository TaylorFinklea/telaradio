//! Generator trait: the abstraction every model implementation conforms to.
//!
//! Recipes pin a specific `(id, version)` pair so audio reproduces
//! deterministically across model upgrades. Adding a new generator never
//! silently changes existing recipes.

use thiserror::Error;

use crate::audio::WavBuffer;

/// Synchronous generator contract. Async wrapping is the backend's job
/// (Phase 2+); keeping the trait sync means `core` stays runtime-free.
pub trait Generator {
    /// Stable identifier (e.g. `"ace-step-1.5-xl"`, `"mock-sine"`).
    fn id(&self) -> &str;

    /// Semantic version of this generator implementation.
    fn version(&self) -> &str;

    /// Generate audio for the given prompt at the given seed and duration.
    /// Returned buffer's sample rate / channel count is the implementation's
    /// canonical output (typically `DEFAULT_SAMPLE_RATE_HZ` /
    /// `DEFAULT_CHANNELS`).
    fn generate(
        &self,
        prompt: &str,
        seed: u64,
        duration_seconds: u32,
    ) -> Result<WavBuffer, GeneratorError>;
}

#[derive(Debug, Error)]
pub enum GeneratorError {
    #[error("generator I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("subprocess returned an error: {0}")]
    Subprocess(String),

    #[error("could not read generated WAV: {0}")]
    Wav(String),

    #[error("subprocess violated the IPC protocol: {0}")]
    ProtocolMismatch(String),
}
