//! Telaradio DSP: amplitude modulation per Woods et al. 2024 §Methods.
//!
//! See `ARCHITECTURE.md` §Modulation DSP stages for the audio-graph
//! context. The DSP stage is pure: no side effects, allocation only for
//! the output `WavBuffer`.

pub mod am;
pub mod envelope;

pub use am::apply_am;
pub use envelope::Envelope;
