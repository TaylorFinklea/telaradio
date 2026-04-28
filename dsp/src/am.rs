//! Amplitude modulation per Woods et al. 2024 §Methods.
//!
//! `apply_am` is a pure function: given an input `WavBuffer`, a rate,
//! depth, and envelope shape, it returns a new `WavBuffer` with each
//! frame multiplied by a sample-rate-aware gate envelope. Stereo
//! channels are modulated identically (paper-faithful).
//!
//! For Square envelopes a 1 ms linear crossfade is applied around each
//! transition to suppress audible clicks at high depth. Sine and
//! Triangle envelopes are continuous and need no ramp.

use telaradio_core::WavBuffer;

use crate::envelope::Envelope;

/// Anti-click ramp half-width in seconds. The ramp is centered on each
/// Square transition, so the total ramp width is `2 * RAMP_HALF_WIDTH_S`.
/// 1 ms total = 0.5 ms each side. This is short enough to be inaudible
/// as a level change at low rates and long enough to suppress clicks at
/// high depth.
const RAMP_HALF_WIDTH_S: f64 = 0.000_5;

/// Apply amplitude modulation to a `WavBuffer`.
///
/// # Parameters
/// - `buffer`: input PCM, interleaved when `channels > 1`.
/// - `rate_hz`: gate frequency (Woods et al. used beta-band rates;
///   default 16 Hz).
/// - `depth`: gate trough depth in `[0.0, 1.0]`. 0 = no modulation,
///   1 = trough goes to silence.
/// - `envelope`: gate shape.
///
/// # Behaviour
/// - depth=0 returns samples unchanged (within f32 epsilon).
/// - Stereo channels receive identical gate values per frame.
/// - Phase is reset at frame 0; the same `(i, sample_rate, rate_hz)`
///   triple yields the same gate value regardless of channel count.
///
/// # Allocation
/// Allocates exactly one output `Vec<f32>` of `buffer.samples.len()`.
///
/// # Cast precision
/// Frame index → f64 is bounded by `samples.len()/channels`. f64 has a
/// 53-bit mantissa (~9.0e15), so the cast is exact for any audio
/// duration we care about (millennia at 44.1 kHz). The final gate
/// f64 → f32 cast loses precision but never truncates audibly: the gate
/// is in `[0, 1]` and f32 precision dwarfs the dB-level effects we care
/// about.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn apply_am(buffer: &WavBuffer, rate_hz: f64, depth: f64, envelope: Envelope) -> WavBuffer {
    let channels = usize::from(buffer.channels);
    let frames = buffer.samples.len() / channels.max(1);
    let mut samples = Vec::with_capacity(buffer.samples.len());

    let sr = f64::from(buffer.sample_rate);
    // Half-width of the Square anti-click ramp in *cycle fractions*
    // (because we compare against `fraction` in [0, 1)).
    let ramp_half_frac = RAMP_HALF_WIDTH_S * rate_hz;

    for i in 0..frames {
        let phase = (i as f64 / sr) * rate_hz;
        let fraction = phase - phase.floor();
        let gate = compute_gate(fraction, depth, envelope, ramp_half_frac);
        let gate_f32 = gate as f32;
        for c in 0..channels {
            let s = buffer.samples[i * channels + c];
            samples.push(s * gate_f32);
        }
    }

    WavBuffer {
        sample_rate: buffer.sample_rate,
        channels: buffer.channels,
        samples,
    }
}

/// Compute the gate value at a given cycle fraction.
///
/// `fraction` is in `[0.0, 1.0)`. `ramp_half_frac` is the
/// anti-click ramp half-width expressed in cycle fractions
/// (only used by `Square`).
fn compute_gate(fraction: f64, depth: f64, envelope: Envelope, ramp_half_frac: f64) -> f64 {
    match envelope {
        Envelope::Square => square_gate(fraction, depth, ramp_half_frac),
        Envelope::Sine => {
            // 1 - depth * (0.5 - 0.5*cos(2π·fraction))
            //   = 1 - depth * 0.5 * (1 - cos(2π·fraction))
            // At fraction=0: 1.0 (peak). At fraction=0.5: 1-depth (trough).
            let two_pi = std::f64::consts::TAU;
            1.0 - depth * 0.5 * (1.0 - (two_pi * fraction).cos())
        }
        Envelope::Triangle => {
            // Piecewise linear: peak at fraction=0, trough at fraction=0.5.
            // tri ∈ [0, 1]; gate = 1 - depth * tri.
            let tri = if fraction < 0.5 {
                2.0 * fraction
            } else {
                2.0 * (1.0 - fraction)
            };
            1.0 - depth * tri
        }
    }
}

/// Square-wave gate with a 1 ms linear crossfade applied around each
/// transition (at fraction=0.0 and fraction=0.5).
///
/// Transitions:
/// - fraction crossing 0.0 (== crossing 1.0): trough → peak (rising)
/// - fraction crossing 0.5: peak → trough (falling)
fn square_gate(fraction: f64, depth: f64, ramp_half_frac: f64) -> f64 {
    let peak = 1.0_f64;
    let trough = 1.0 - depth;

    // Distance to each of the two transition points, expressed as a
    // signed delta in [-0.5, 0.5]. The rising transition is at 0.0
    // (treat 1.0 as 0.0). The falling transition is at 0.5.
    let to_rising = if fraction <= 0.5 {
        fraction
    } else {
        fraction - 1.0
    };
    let to_falling = fraction - 0.5;

    // If we're inside the ramp window of either transition, interpolate
    // linearly across `2 * ramp_half_frac` cycle fractions. Outside the
    // ramp, hold at peak (first half) or trough (second half).
    if to_rising.abs() < ramp_half_frac {
        // Crossfade trough → peak, centered on fraction=0.
        // t ∈ [0, 1] across the full ramp width.
        let t = (to_rising + ramp_half_frac) / (2.0 * ramp_half_frac);
        trough + (peak - trough) * t
    } else if to_falling.abs() < ramp_half_frac {
        // Crossfade peak → trough, centered on fraction=0.5.
        let t = (to_falling + ramp_half_frac) / (2.0 * ramp_half_frac);
        peak + (trough - peak) * t
    } else if fraction < 0.5 {
        peak
    } else {
        trough
    }
}
