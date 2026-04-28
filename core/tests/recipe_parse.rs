//! Integration tests for `telaradio_core::Recipe` parsing and round-tripping.
//!
//! These tests describe the public API contract of the recipe parser
//! before any implementation exists. See `ARCHITECTURE.md` §Recipe format.

use telaradio_core::{Envelope, Recipe, RecipeError};

const VALID_MINIMAL: &str = r#"{
  "schema_version": "1",
  "id": "5b4f2a8c-9e3d-4f17-b2a1-7c0c1f3e8d92",
  "title": "Foggy lofi for deep work",
  "tags": ["lofi", "focus"],
  "prompt": "warm vinyl lofi, jazzy keys, slow tempo, no vocals",
  "seed": 1893421,
  "model": { "id": "ace-step-v1-3.5b", "version": "1.0.0" },
  "duration_seconds": 240,
  "modulation": { "rate_hz": 16.0, "depth": 0.5, "envelope": "square" },
  "created_at": "2026-04-26T15:00:00Z",
  "author": "tfinklea"
}"#;

#[test]
fn parses_a_valid_recipe() {
    let recipe = Recipe::parse(VALID_MINIMAL).expect("valid recipe should parse");

    assert_eq!(recipe.schema_version, "1");
    assert_eq!(recipe.title, "Foggy lofi for deep work");
    assert_eq!(recipe.tags, vec!["lofi", "focus"]);
    assert_eq!(recipe.seed, 1_893_421);
    assert_eq!(recipe.duration_seconds, 240);
    assert!((recipe.modulation.rate_hz - 16.0).abs() < f64::EPSILON);
    assert!((recipe.modulation.depth - 0.5).abs() < f64::EPSILON);
    assert_eq!(recipe.modulation.envelope, Envelope::Square);
    assert_eq!(recipe.model.id, "ace-step-v1-3.5b");
    assert_eq!(recipe.model.version, "1.0.0");
    assert_eq!(recipe.author, "tfinklea");
}

#[test]
fn parses_each_envelope_variant() {
    for envelope_str in &["square", "sine", "triangle"] {
        let json = VALID_MINIMAL.replace(
            r#""envelope": "square""#,
            &format!(r#""envelope": "{envelope_str}""#),
        );
        let recipe = Recipe::parse(&json)
            .unwrap_or_else(|e| panic!("envelope `{envelope_str}` should parse: {e}"));
        let expected = match *envelope_str {
            "square" => Envelope::Square,
            "sine" => Envelope::Sine,
            "triangle" => Envelope::Triangle,
            _ => unreachable!(),
        };
        assert_eq!(recipe.modulation.envelope, expected);
    }
}

#[test]
fn round_trips_through_serialize_and_parse() {
    let recipe = Recipe::parse(VALID_MINIMAL).unwrap();
    let serialized = recipe.serialize().expect("serialize should succeed");
    let reparsed = Recipe::parse(&serialized).expect("reparse should succeed");
    assert_eq!(recipe, reparsed);
}

#[test]
fn rejects_unknown_schema_version() {
    let json = VALID_MINIMAL.replace(r#""schema_version": "1""#, r#""schema_version": "2""#);
    match Recipe::parse(&json) {
        Err(RecipeError::UnsupportedSchemaVersion { found }) => assert_eq!(found, "2"),
        other => panic!("expected UnsupportedSchemaVersion, got {other:?}"),
    }
}

#[test]
fn rejects_unknown_envelope() {
    let json = VALID_MINIMAL.replace(r#""envelope": "square""#, r#""envelope": "sawtooth""#);
    match Recipe::parse(&json) {
        Err(RecipeError::Json(_)) => {} // serde rejects unknown enum variant
        other => panic!("expected Json error for unknown envelope, got {other:?}"),
    }
}

#[test]
fn rejects_depth_out_of_range_high() {
    let json = VALID_MINIMAL.replace(r#""depth": 0.5"#, r#""depth": 1.5"#);
    match Recipe::parse(&json) {
        Err(RecipeError::DepthOutOfRange(v)) => assert!((v - 1.5).abs() < f64::EPSILON),
        other => panic!("expected DepthOutOfRange, got {other:?}"),
    }
}

#[test]
fn rejects_depth_out_of_range_negative() {
    let json = VALID_MINIMAL.replace(r#""depth": 0.5"#, r#""depth": -0.1"#);
    match Recipe::parse(&json) {
        Err(RecipeError::DepthOutOfRange(_)) => {}
        other => panic!("expected DepthOutOfRange, got {other:?}"),
    }
}

#[test]
fn rejects_non_positive_rate() {
    let json = VALID_MINIMAL.replace(r#""rate_hz": 16.0"#, r#""rate_hz": 0.0"#);
    match Recipe::parse(&json) {
        Err(RecipeError::RateNonPositive(_)) => {}
        other => panic!("expected RateNonPositive, got {other:?}"),
    }
}

#[test]
fn rejects_zero_duration() {
    let json = VALID_MINIMAL.replace(r#""duration_seconds": 240"#, r#""duration_seconds": 0"#);
    match Recipe::parse(&json) {
        Err(RecipeError::DurationZero) => {}
        other => panic!("expected DurationZero, got {other:?}"),
    }
}

#[test]
fn rejects_missing_required_field() {
    // Drop the "seed" field entirely.
    let json = VALID_MINIMAL.replace("\n  \"seed\": 1893421,\n", "\n");
    match Recipe::parse(&json) {
        Err(RecipeError::Json(_)) => {}
        other => panic!("expected Json error for missing seed, got {other:?}"),
    }
}

#[test]
fn rejects_wrong_type_for_seed() {
    let json = VALID_MINIMAL.replace(r#""seed": 1893421"#, r#""seed": "not-a-number""#);
    match Recipe::parse(&json) {
        Err(RecipeError::Json(_)) => {}
        other => panic!("expected Json error for wrong seed type, got {other:?}"),
    }
}

#[test]
fn rejects_unknown_top_level_field() {
    let json = VALID_MINIMAL.replace(
        r#""author": "tfinklea""#,
        r#""author": "tfinklea", "extra_field": "rejected""#,
    );
    match Recipe::parse(&json) {
        Err(RecipeError::Json(_)) => {} // deny_unknown_fields rejects extras
        other => panic!("expected Json error for unknown field, got {other:?}"),
    }
}

#[test]
fn rejects_malformed_uuid() {
    let json = VALID_MINIMAL.replace(
        r#""id": "5b4f2a8c-9e3d-4f17-b2a1-7c0c1f3e8d92""#,
        r#""id": "not-a-uuid""#,
    );
    match Recipe::parse(&json) {
        Err(RecipeError::Json(_)) => {} // strict Uuid rejects via serde
        other => panic!("expected Json error for malformed UUID, got {other:?}"),
    }
}

#[test]
fn parses_committed_example_recipe() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../recipes/example-foggy-lofi.json"
    );
    let json = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("could not read example recipe at {path}: {e}"));
    Recipe::parse(&json).expect("committed example recipe should be valid");
}
