//! Telaradio model adapter.
//!
//! Phase 1b: Generator trait impls backed by a Python subprocess running a
//! mock 440 Hz sine engine. Phase 1b2 swaps the mock for real ACE-Step.

pub mod hf_download;
pub mod model_install;
pub mod protocol;
pub mod subprocess;

pub use subprocess::{MOCK_GENERATOR_ID, MOCK_GENERATOR_VERSION, SubprocessGenerator};
