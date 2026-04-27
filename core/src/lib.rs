//! Lockstep core: recipe types, schema v1 parser, and error types.
//!
//! See `ARCHITECTURE.md` at the repo root for the canonical schema.

pub mod error;
pub mod recipe;

pub use error::RecipeError;
pub use recipe::{Envelope, ModelRef, Modulation, Recipe};
