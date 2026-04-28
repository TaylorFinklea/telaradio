//! Mocked `AceStepGenerator` smoke tests. Spawns the *mock* Python
//! subprocess (Phase 1b's 440 Hz sine engine) but exercises the
//! `AceStepGenerator` Rust wrapper. The real ACE-Step model is not
//! exercised here; it has its own `#[ignore]` integration test.
//!
//! These tests are non-ignored so the Rust IPC + id/version contract
//! has CI coverage even when ACE-Step is not installed.

use std::path::PathBuf;

use telaradio_core::audio::{DEFAULT_CHANNELS, DEFAULT_SAMPLE_RATE_HZ};
use telaradio_core::generator::Generator;
use telaradio_model_adapter::ace_step::{ACE_STEP_GENERATOR_ID, ACE_STEP_GENERATOR_VERSION};
use telaradio_model_adapter::{AceStepGenerator, SubprocessGenerator};

fn mock_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/telaradio_subprocess.py")
}

#[test]
fn ace_step_generator_id_and_version_match_constants() {
    // We use a generator wired to the *mock* script so the test runs
    // offline. The id/version come from the Rust struct, not the script,
    // so this still tests the right thing.
    let generator =
        AceStepGenerator::spawn_with_script(&mock_script()).expect("spawn ace-step generator");
    assert_eq!(generator.id(), ACE_STEP_GENERATOR_ID);
    assert_eq!(generator.id(), "ace-step-v1-3.5b");
    assert_eq!(generator.version(), ACE_STEP_GENERATOR_VERSION);
    assert_eq!(generator.version(), "1.0.0");
}

#[test]
fn ace_step_generator_round_trips_via_mock_subprocess() {
    let generator =
        AceStepGenerator::spawn_with_script(&mock_script()).expect("spawn ace-step generator");
    let buf = generator
        .generate("a calm focus track", 7, 1)
        .expect("generate");

    assert_eq!(buf.sample_rate, DEFAULT_SAMPLE_RATE_HZ);
    assert_eq!(buf.channels, DEFAULT_CHANNELS);
    assert_eq!(
        buf.samples.len(),
        DEFAULT_SAMPLE_RATE_HZ as usize * DEFAULT_CHANNELS as usize,
    );
}

#[test]
fn ace_step_generator_does_not_share_id_with_mock() {
    // Sanity: the two generators have distinct ids so a recipe can pin
    // either one unambiguously.
    let mock = SubprocessGenerator::spawn(&mock_script()).expect("spawn mock");
    let ace =
        AceStepGenerator::spawn_with_script(&mock_script()).expect("spawn ace-step generator");
    assert_ne!(mock.id(), ace.id());
}
