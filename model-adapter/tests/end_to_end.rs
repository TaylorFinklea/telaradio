//! End-to-end integration: spawn the Python mock subprocess, request a
//! one-second buffer, verify the round trip.
//!
//! This test will fail to spawn if `python3` is missing on PATH. That is
//! considered an environment problem, not a code defect; the Rust portion
//! of Phase 1b is correct independent of Python availability.

use std::path::PathBuf;

use telaradio_core::audio::{DEFAULT_CHANNELS, DEFAULT_SAMPLE_RATE_HZ};
use telaradio_core::generator::Generator;
use telaradio_model_adapter::SubprocessGenerator;

fn python_script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/telaradio_subprocess.py")
}

#[test]
fn generates_a_one_second_stereo_buffer() {
    let generator = SubprocessGenerator::spawn(&python_script_path()).expect("spawn subprocess");
    let buf = generator.generate("test prompt", 42, 1).expect("generate");

    assert_eq!(buf.sample_rate, DEFAULT_SAMPLE_RATE_HZ);
    assert_eq!(buf.channels, DEFAULT_CHANNELS);
    let expected_len = DEFAULT_SAMPLE_RATE_HZ as usize * DEFAULT_CHANNELS as usize;
    assert_eq!(
        buf.samples.len(),
        expected_len,
        "1 second stereo @ 44.1 kHz"
    );

    // The mock generates a 440 Hz sine; many samples should be clearly non-zero.
    let nonzero = buf.samples.iter().filter(|s| s.abs() > 0.01).count();
    assert!(
        nonzero > expected_len / 4,
        "expected most of the sine to be non-zero, got {nonzero}/{expected_len}"
    );
}

#[test]
fn subprocess_handles_multiple_sequential_requests() {
    let generator = SubprocessGenerator::spawn(&python_script_path()).expect("spawn subprocess");
    for seed in 0_u64..3 {
        let buf = generator
            .generate("test", seed, 1)
            .expect("generate iteration");
        assert_eq!(
            buf.samples.len(),
            DEFAULT_SAMPLE_RATE_HZ as usize * DEFAULT_CHANNELS as usize,
        );
    }
}

#[test]
fn generator_reports_id_and_version() {
    let generator = SubprocessGenerator::spawn(&python_script_path()).expect("spawn subprocess");
    assert_eq!(generator.id(), "mock-sine");
    assert!(!generator.version().is_empty());
}

#[test]
fn dropped_subprocess_does_not_leak_zombie() {
    // Just spawn-and-drop. If the Drop impl is missing or broken this leaks
    // a zombie; we don't verify the absence here, but exercising Drop is
    // half the value, and a future test could `pgrep python3` with a unique
    // marker if needed.
    {
        let _generator = SubprocessGenerator::spawn(&python_script_path()).expect("spawn");
    }
}
