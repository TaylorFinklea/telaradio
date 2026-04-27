//! Audio buffer types and platform-default sample-rate/channel constants.

/// Telaradio's default audio sample rate (44.1 kHz, music-industry standard
/// and ACE-Step's documented output rate).
pub const DEFAULT_SAMPLE_RATE_HZ: u32 = 44_100;

/// Telaradio's default channel count (stereo, ACE-Step's default).
pub const DEFAULT_CHANNELS: u8 = 2;

/// A PCM audio buffer. `samples` is interleaved when `channels > 1`
/// (e.g. for stereo: L0, R0, L1, R1, ...).
#[derive(Debug, Clone, PartialEq)]
pub struct WavBuffer {
    pub sample_rate: u32,
    pub channels: u8,
    pub samples: Vec<f32>,
}

impl WavBuffer {
    /// Wall-clock duration in seconds, derived from `samples.len()`,
    /// `sample_rate`, and `channels`.
    ///
    /// Cast precision: `samples.len()` is a usize. f64 has a 53-bit
    /// mantissa (2^53 samples = ~3 hours of stereo audio at 44.1 kHz),
    /// well past any recipe duration we care about.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn duration_seconds(&self) -> f64 {
        let frames = self.samples.len() / usize::from(self.channels);
        frames as f64 / f64::from(self.sample_rate)
    }
}
