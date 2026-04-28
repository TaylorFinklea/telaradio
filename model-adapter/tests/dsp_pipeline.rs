//! End-to-end smoke test for the DSP pipeline:
//! generate a 1s mock WAV → `apply_am` at 16 Hz / 0.5 / Square → assert
//! the modulation actually altered the sample distribution.
//!
//! Lives in `model-adapter/tests/` because it depends on both
//! `telaradio-dsp` (the modulation DSP) and `telaradio-model-adapter`
//! (the mock subprocess generator).
//!
//! Skips its assertions silently if `python3` is not on PATH, matching
//! the convention in `end_to_end.rs`.

#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::path::PathBuf;

use telaradio_core::generator::Generator;
use telaradio_dsp::{Envelope, apply_am};
use telaradio_model_adapter::SubprocessGenerator;

fn python_script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/telaradio_subprocess.py")
}

#[test]
fn modulation_alters_sample_distribution() {
    let generator = SubprocessGenerator::spawn(&python_script_path()).expect("spawn subprocess");
    let buf = generator.generate("test prompt", 42, 1).expect("generate");

    // Sanity: the generator returned a buffer with non-zero RMS.
    let raw_rms = rms(&buf.samples);
    assert!(
        raw_rms > 0.01,
        "raw mock signal should have non-trivial energy"
    );

    let modulated = apply_am(&buf, 16.0, 0.5, Envelope::Square);

    // Same metadata.
    assert_eq!(modulated.sample_rate, buf.sample_rate);
    assert_eq!(modulated.channels, buf.channels);
    assert_eq!(modulated.samples.len(), buf.samples.len());

    // Modulation must change the signal: at least one sample must
    // differ from the input.
    let any_diff = buf
        .samples
        .iter()
        .zip(&modulated.samples)
        .any(|(a, b)| (a - b).abs() > 1e-6);
    assert!(any_diff, "AM at depth=0.5 should change the signal");

    // The trough region (gate=0.5) should be visibly attenuated. Each
    // frame is multiplied by either 1.0 (peak) or 0.5 (trough) plus a
    // brief 1ms ramp. So the modulated RMS should sit between the raw
    // RMS and half of it.
    let mod_rms = rms(&modulated.samples);
    assert!(
        mod_rms < raw_rms * 0.95,
        "modulated RMS ({mod_rms}) should be lower than raw ({raw_rms})",
    );
    assert!(
        mod_rms > raw_rms * 0.5,
        "modulated RMS ({mod_rms}) should be > half of raw ({raw_rms})",
    );
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|s| f64::from(*s) * f64::from(*s)).sum();
    (sum_sq / samples.len() as f64).sqrt() as f32
}
