//! Telaradio model adapter.
//!
//! Phase 1b: Generator trait impls backed by a Python subprocess running a
//! mock 440 Hz sine engine. Phase 1b2 swaps the mock for real ACE-Step.

pub mod ace_step;
pub mod hf_download;
mod ipc;
pub mod model_install;
pub mod protocol;
pub mod subprocess;

pub use ace_step::{
    ACE_STEP_GENERATOR_ID, ACE_STEP_GENERATOR_VERSION, AceStepGenerator, ace_step_artifacts,
};
pub use hf_download::CancellationToken;
pub use model_install::{InstallMode, ModelArtifact, ensure_model};
pub use subprocess::{MOCK_GENERATOR_ID, MOCK_GENERATOR_VERSION, SubprocessGenerator};
