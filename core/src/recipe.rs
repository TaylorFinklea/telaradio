use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::RecipeError;

/// Modulation envelope shape. Schema v1 ships `square` per Woods et al. 2024;
/// `sine` and `triangle` are accepted for future audio-graph experiments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Envelope {
    Square,
    Sine,
    Triangle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Modulation {
    pub rate_hz: f64,
    pub depth: f64,
    pub envelope: Envelope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRef {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub schema_version: String,
    pub id: Uuid,
    pub title: String,
    pub tags: Vec<String>,
    pub prompt: String,
    pub seed: u64,
    pub model: ModelRef,
    pub duration_seconds: u32,
    pub modulation: Modulation,
    pub created_at: DateTime<Utc>,
    pub author: String,
}

impl Recipe {
    /// Parse a recipe from JSON. Validates schema version and modulation
    /// invariants beyond what serde catches structurally.
    pub fn parse(json: &str) -> Result<Self, RecipeError> {
        let recipe: Self = serde_json::from_str(json)?;
        recipe.validate()?;
        Ok(recipe)
    }

    /// Serialize the recipe as pretty JSON.
    pub fn serialize(&self) -> Result<String, RecipeError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    fn validate(&self) -> Result<(), RecipeError> {
        if self.schema_version != "1" {
            return Err(RecipeError::UnsupportedSchemaVersion {
                found: self.schema_version.clone(),
            });
        }
        if !(0.0..=1.0).contains(&self.modulation.depth) {
            return Err(RecipeError::DepthOutOfRange(self.modulation.depth));
        }
        if self.modulation.rate_hz <= 0.0 {
            return Err(RecipeError::RateNonPositive(self.modulation.rate_hz));
        }
        if self.duration_seconds == 0 {
            return Err(RecipeError::DurationZero);
        }
        Ok(())
    }
}
