//! Integration tests for the public `telaradio-dsp` API.
//!
//! TDD: every public function/type/trait gets a failing test before
//! implementation. Audio-math casts are unavoidable here; allow them
//! at the module level.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::no_effect_underscore_binding
)]

use telaradio_core::WavBuffer;
use telaradio_dsp::{Envelope, apply_am};

const SR: u32 = 44_100;

/// Build a stereo `WavBuffer` where every sample is 1.0. Ideal carrier
/// for verifying the gate envelope shape — `output == gate` directly.
fn unit_stereo_buffer(seconds: f64) -> WavBuffer {
    let frames = (f64::from(SR) * seconds) as usize;
    let samples = vec![1.0_f32; frames * 2];
    WavBuffer {
        sample_rate: SR,
        channels: 2,
        samples,
    }
}

#[test]
fn envelope_variants_construct() {
    // Smoke: each variant exists and is Copy/Clone/Eq.
    let square = Envelope::Square;
    let sine = Envelope::Sine;
    let triangle = Envelope::Triangle;
    assert_ne!(square, sine);
    assert_ne!(sine, triangle);
}

#[test]
fn depth_zero_returns_input_unchanged() {
    let input = unit_stereo_buffer(0.5);
    let output = apply_am(&input, 16.0, 0.0, Envelope::Square);

    assert_eq!(output.sample_rate, input.sample_rate);
    assert_eq!(output.channels, input.channels);
    assert_eq!(output.samples.len(), input.samples.len());
    for (i, (&a, &b)) in input.samples.iter().zip(&output.samples).enumerate() {
        assert!((a - b).abs() < 1e-6, "sample {i}: input {a} != output {b}");
    }
}

#[test]
fn square_full_depth_attenuates_trough_to_floor() {
    // Square at depth=1 sends the gate trough to 0.0. Roughly half
    // the samples should be at (or very near) 0.0; the other half at
    // 1.0. The 1 ms anti-click ramp eats a small fraction of frames
    // around each transition — leave plenty of slack.
    let input = unit_stereo_buffer(1.0);
    let output = apply_am(&input, 16.0, 1.0, Envelope::Square);

    let near_zero = output.samples.iter().filter(|s| s.abs() < 1e-3).count();
    let total = output.samples.len();
    // Allow a wide window: 50% ± 5%
    let frac = near_zero as f64 / total as f64;
    assert!(
        (0.45..=0.55).contains(&frac),
        "expected ~50% near-zero samples, got {frac}"
    );
}

#[test]
fn rate_locked_phase_4_hz_one_second_yields_4_cycles() {
    // 4 Hz over 1 second @ 44.1 kHz = exactly 4 gate cycles.
    // Cycle = peak half + trough half. Count *falling* transitions
    // (peak → trough) as the cycle marker: there is exactly one per
    // cycle and the first frame starts at the peak (not in trough), so
    // we observe all 4 transitions over 1 second.
    let input = unit_stereo_buffer(1.0);
    let output = apply_am(&input, 4.0, 1.0, Envelope::Square);

    let mut transitions = 0_usize;
    let mut prev_high = false;
    for chunk in output.samples.chunks_exact(2) {
        let v = chunk[0];
        let high = v > 0.5;
        if prev_high && !high {
            transitions += 1;
        }
        prev_high = high;
    }
    assert_eq!(
        transitions, 4,
        "expected 4 falling gate transitions over 1s @ 4 Hz, got {transitions}"
    );
}

#[test]
fn sine_envelope_has_no_large_step_discontinuities() {
    // Sine envelope is C^∞: max |Δgate| per sample = depth * π * rate / sr.
    // We allow a generous safety margin (10x).
    let input = unit_stereo_buffer(0.25);
    let rate = 16.0_f64;
    let depth = 1.0_f64;
    let output = apply_am(&input, rate, depth, Envelope::Sine);

    let max_step = depth * std::f64::consts::PI * rate / f64::from(SR) * 10.0;
    let max_step_f32 = max_step as f32;

    for window in output
        .samples
        .chunks_exact(2)
        .collect::<Vec<_>>()
        .windows(2)
    {
        let prev = window[0][0];
        let cur = window[1][0];
        let step = (cur - prev).abs();
        assert!(
            step <= max_step_f32,
            "sine step {step} exceeded bound {max_step_f32}"
        );
    }
}

#[test]
fn triangle_envelope_is_piecewise_linear() {
    // Triangle envelope has constant slope between apexes.
    // We sample 3 consecutive frames in the middle of an up-ramp and
    // assert the second derivative is approximately zero (to f32 noise).
    let input = unit_stereo_buffer(0.25);
    let output = apply_am(&input, 4.0, 1.0, Envelope::Triangle);

    // Skip the first quarter cycle (apex region) and the very last few
    // frames (apex region). Sample a stretch in between.
    let total_frames = output.samples.len() / 2;
    let start = total_frames / 16; // safely inside an up-ramp
    let end = total_frames / 8; // still inside the same up-ramp
    let mut max_second_diff = 0.0_f32;
    for i in (start + 1)..(end - 1) {
        let a = output.samples[(i - 1) * 2];
        let b = output.samples[i * 2];
        let c = output.samples[(i + 1) * 2];
        let second = (c - 2.0 * b + a).abs();
        if second > max_second_diff {
            max_second_diff = second;
        }
    }
    // Generous f32-noise allowance.
    assert!(
        max_second_diff < 1e-4,
        "triangle second-difference {max_second_diff} too large (expected ~0)"
    );
}

#[test]
fn stereo_pair_invariant_holds() {
    // For every frame i, output[2i] == output[2i+1] when the input has
    // identical L and R: AM is applied per-frame, identical on both
    // channels (Woods et al. paper-faithful).
    let input = unit_stereo_buffer(0.1);
    let output = apply_am(&input, 16.0, 0.7, Envelope::Square);

    for chunk in output.samples.chunks_exact(2) {
        assert!(
            (chunk[0] - chunk[1]).abs() < f32::EPSILON,
            "L/R diverged: {} vs {}",
            chunk[0],
            chunk[1],
        );
    }
}

#[test]
fn anti_click_ramp_smooths_square_transition() {
    // At a Square down-transition, the gate goes from 1.0 to (1 - depth).
    // Without a ramp, a single sample-pair shows the entire jump.
    // With a 1 ms linear ramp, the jump is spread over ~44 samples @ 44.1 kHz.
    // We assert that no two adjacent frames differ by more than a fraction
    // of `depth`.
    let input = unit_stereo_buffer(0.5);
    let depth = 1.0_f32;
    let output = apply_am(&input, 16.0, f64::from(depth), Envelope::Square);

    // Without a ramp, the jump is `depth` per frame at the transition.
    // With a 1 ms ramp at 44.1 kHz, the jump is at most ~depth/44 per
    // frame. Allow generous slack: assert max step < 0.1 (would be 1.0
    // without a ramp).
    let mut max_step = 0.0_f32;
    for w in output
        .samples
        .chunks_exact(2)
        .collect::<Vec<_>>()
        .windows(2)
    {
        let step = (w[1][0] - w[0][0]).abs();
        if step > max_step {
            max_step = step;
        }
    }
    assert!(
        max_step < 0.1,
        "expected smooth ramp, got max step {max_step}"
    );
}

#[test]
fn output_preserves_metadata() {
    let input = unit_stereo_buffer(0.05);
    let output = apply_am(&input, 16.0, 0.5, Envelope::Square);
    assert_eq!(output.sample_rate, input.sample_rate);
    assert_eq!(output.channels, input.channels);
    assert_eq!(output.samples.len(), input.samples.len());
}

#[test]
fn mono_input_modulates_correctly() {
    // Mono channels=1: gate applies frame-by-frame too. Use a 1-second
    // buffer so we cover multiple full gate cycles (peaks *and* troughs).
    let frames: usize = SR as usize; // 1s at 44.1 kHz
    let input = WavBuffer {
        sample_rate: SR,
        channels: 1,
        samples: vec![1.0_f32; frames],
    };
    let output = apply_am(&input, 4.0, 1.0, Envelope::Square);
    assert_eq!(output.channels, 1);
    assert_eq!(output.samples.len(), frames);
    // ~50% of samples should be near the trough (depth=1 → 0.0).
    let near_zero = output.samples.iter().filter(|s| s.abs() < 1e-3).count();
    assert!(
        near_zero > frames / 4,
        "mono trough should attenuate, got {near_zero}/{frames}"
    );
}
