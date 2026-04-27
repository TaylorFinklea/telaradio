use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecipeError {
    #[error("recipe JSON is malformed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("unsupported schema_version `{found}`; this build of lockstep-core only supports `1`")]
    UnsupportedSchemaVersion { found: String },

    #[error("modulation.depth must be in [0.0, 1.0], got {0}")]
    DepthOutOfRange(f64),

    #[error("modulation.rate_hz must be > 0, got {0}")]
    RateNonPositive(f64),

    #[error("duration_seconds must be > 0")]
    DurationZero,
}
