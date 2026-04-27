//! Telaradio core: recipe types, schema v1 parser, and error types.
//!
//! See `ARCHITECTURE.md` at the repo root for the canonical schema.

pub mod audio;
pub mod error;
pub mod generator;
pub mod recipe;

pub use audio::{DEFAULT_CHANNELS, DEFAULT_SAMPLE_RATE_HZ, WavBuffer};
pub use error::RecipeError;
pub use generator::{Generator, GeneratorError};
pub use recipe::{Envelope, ModelRef, Modulation, Recipe};
