//! Modulation envelope shapes (DSP-side).
//!
//! Owned by the DSP crate so DSP can grow new envelope shapes without
//! forcing a recipe schema bump. See `core::recipe::Envelope` for the
//! recipe-side enum; the two are related by a small `From` impl that
//! callers writing pipeline code can reach for if needed.

use telaradio_core::recipe::Envelope as RecipeEnvelope;

/// Modulation envelope shape used by [`crate::apply_am`].
///
/// - `Square`: instant flip between peak (1.0) and trough (1 - depth)
///   with a 1 ms linear crossfade applied around each transition.
/// - `Sine`: smooth `1 - depth * (0.5 - 0.5*cos(2π·phase))`. Already
///   continuous; no anti-click ramp needed.
/// - `Triangle`: piecewise-linear; rises from trough to peak across the
///   first half-cycle, descends across the second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Envelope {
    Square,
    Sine,
    Triangle,
}

impl From<RecipeEnvelope> for Envelope {
    fn from(value: RecipeEnvelope) -> Self {
        match value {
            RecipeEnvelope::Square => Self::Square,
            RecipeEnvelope::Sine => Self::Sine,
            RecipeEnvelope::Triangle => Self::Triangle,
        }
    }
}
