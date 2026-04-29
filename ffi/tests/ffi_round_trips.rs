//! Integration tests exercising the C-ABI surface through `unsafe`.
//! These tests validate ownership semantics, error reporting, and the
//! end-to-end pipeline that the Swift app will use.

#![allow(unsafe_code)]
// Test-data generators cast small loop indices into f32 / f64. The values
// always fit in the mantissa (max ~2k); the cast is intentional.
#![allow(clippy::cast_precision_loss)]

use std::ffi::{CStr, CString};
use std::path::PathBuf;

use telaradio_ffi::{
    tr_apply_am, tr_generate_mock, tr_last_error, tr_recipe_free, tr_recipe_parse,
    tr_wavbuffer_channels, tr_wavbuffer_free, tr_wavbuffer_len, tr_wavbuffer_new,
    tr_wavbuffer_sample_rate, tr_wavbuffer_samples,
};

const VALID_RECIPE_JSON: &str = r#"{
  "schema_version": "1",
  "id": "5b4f2a8c-9e3d-4f17-b2a1-7c0c1f3e8d92",
  "title": "Foggy lofi for deep work",
  "tags": ["lofi", "focus"],
  "prompt": "warm vinyl lofi",
  "seed": 1893421,
  "model": { "id": "ace-step-v1-3.5b", "version": "1.0.0" },
  "duration_seconds": 240,
  "modulation": { "rate_hz": 16.0, "depth": 0.5, "envelope": "square" },
  "created_at": "2026-04-27T15:00:00Z",
  "author": "tfinklea"
}"#;

fn cstring(s: &str) -> CString {
    CString::new(s).expect("test string contains no NUL")
}

fn mock_script_path() -> CString {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("ffi has a parent")
        .join("model-adapter/python/telaradio_subprocess.py");
    cstring(path.to_str().expect("path is utf8"))
}

#[test]
fn parses_valid_recipe_returns_non_null() {
    let json = cstring(VALID_RECIPE_JSON);
    unsafe {
        let recipe = tr_recipe_parse(json.as_ptr());
        assert!(!recipe.is_null(), "valid recipe should parse");
        tr_recipe_free(recipe);
    }
}

#[test]
fn invalid_json_returns_null_and_sets_last_error() {
    let bad = cstring("not valid json {{{");
    unsafe {
        let result = tr_recipe_parse(bad.as_ptr());
        assert!(result.is_null(), "invalid JSON must return null");
        let err = tr_last_error();
        assert!(!err.is_null(), "last_error must be set on failure");
        let msg = CStr::from_ptr(err).to_string_lossy();
        assert!(!msg.is_empty(), "error message must be non-empty");
    }
}

#[test]
fn unknown_schema_version_returns_null() {
    let bad =
        cstring(&VALID_RECIPE_JSON.replace(r#""schema_version": "1""#, r#""schema_version": "9""#));
    unsafe {
        let result = tr_recipe_parse(bad.as_ptr());
        assert!(
            result.is_null(),
            "unsupported schema_version must return null"
        );
        let err = tr_last_error();
        assert!(!err.is_null());
    }
}

#[test]
fn null_input_to_parse_returns_null_safely() {
    unsafe {
        let result = tr_recipe_parse(std::ptr::null());
        assert!(result.is_null());
    }
}

#[test]
fn wavbuffer_new_round_trips_samples_and_metadata() {
    let samples: Vec<f32> = (0..1024).map(|i| (i as f32) / 1024.0).collect();
    unsafe {
        let buf = tr_wavbuffer_new(samples.as_ptr(), samples.len(), 44_100, 2);
        assert!(!buf.is_null());

        assert_eq!(tr_wavbuffer_len(buf), 1024);
        assert_eq!(tr_wavbuffer_sample_rate(buf), 44_100);
        assert_eq!(tr_wavbuffer_channels(buf), 2);

        let samples_ptr = tr_wavbuffer_samples(buf);
        assert!(!samples_ptr.is_null());
        let slice = std::slice::from_raw_parts(samples_ptr, 1024);
        for (i, s) in slice.iter().enumerate() {
            let expected = (i as f32) / 1024.0;
            assert!((s - expected).abs() < f32::EPSILON);
        }

        tr_wavbuffer_free(buf);
    }
}

#[test]
fn apply_am_with_depth_zero_returns_unchanged_buffer() {
    let samples: Vec<f32> = (0..2048).map(|i| ((i % 100) as f32) / 100.0).collect();
    unsafe {
        let input = tr_wavbuffer_new(samples.as_ptr(), samples.len(), 44_100, 2);
        let modulated = tr_apply_am(input, 16.0, 0.0, 0); // 0 = Square
        assert!(!modulated.is_null());

        assert_eq!(tr_wavbuffer_len(modulated), samples.len());
        let out_ptr = tr_wavbuffer_samples(modulated);
        let out = std::slice::from_raw_parts(out_ptr, samples.len());
        for (i, (out_sample, in_sample)) in out.iter().zip(samples.iter()).enumerate() {
            assert!(
                (out_sample - in_sample).abs() < f32::EPSILON * 2.0,
                "depth=0 should be unity at index {i}: out={out_sample}, in={in_sample}"
            );
        }

        tr_wavbuffer_free(modulated);
        tr_wavbuffer_free(input);
    }
}

#[test]
fn apply_am_with_unknown_envelope_returns_null() {
    let samples = vec![0.5_f32; 256];
    unsafe {
        let input = tr_wavbuffer_new(samples.as_ptr(), samples.len(), 44_100, 2);
        let modulated = tr_apply_am(input, 16.0, 0.5, 99); // bogus envelope
        assert!(modulated.is_null());
        let err = tr_last_error();
        assert!(!err.is_null());
        tr_wavbuffer_free(input);
    }
}

#[test]
fn generate_mock_round_trips_through_python_subprocess() {
    let script = mock_script_path();
    let prompt = cstring("test prompt");
    unsafe {
        let buf = tr_generate_mock(script.as_ptr(), prompt.as_ptr(), 42, 1);
        if buf.is_null() {
            let err = tr_last_error();
            let msg = if err.is_null() {
                "(no error message)".to_string()
            } else {
                CStr::from_ptr(err).to_string_lossy().into_owned()
            };
            panic!("generate_mock returned null: {msg}");
        }

        // 1 second of stereo at 44.1 kHz = 88_200 samples.
        assert_eq!(tr_wavbuffer_len(buf), 88_200);
        assert_eq!(tr_wavbuffer_sample_rate(buf), 44_100);
        assert_eq!(tr_wavbuffer_channels(buf), 2);

        let samples = std::slice::from_raw_parts(tr_wavbuffer_samples(buf), 88_200);
        let nonzero = samples.iter().filter(|s| s.abs() > 0.01).count();
        assert!(
            nonzero > 88_200 / 4,
            "mock sine should produce many non-zero samples"
        );

        tr_wavbuffer_free(buf);
    }
}

#[test]
fn end_to_end_generate_then_apply_am() {
    let script = mock_script_path();
    let prompt = cstring("e2e test");
    unsafe {
        let raw = tr_generate_mock(script.as_ptr(), prompt.as_ptr(), 7, 1);
        assert!(!raw.is_null());

        let modulated = tr_apply_am(raw, 16.0, 0.5, 0);
        assert!(!modulated.is_null());

        // Modulation should have changed the sample distribution.
        let raw_samples = std::slice::from_raw_parts(tr_wavbuffer_samples(raw), 88_200);
        let mod_samples = std::slice::from_raw_parts(tr_wavbuffer_samples(modulated), 88_200);
        let differing = raw_samples
            .iter()
            .zip(mod_samples.iter())
            .filter(|(a, b)| (*a - *b).abs() > f32::EPSILON)
            .count();
        assert!(differing > 88_200 / 4, "AM should alter many samples");

        tr_wavbuffer_free(modulated);
        tr_wavbuffer_free(raw);
    }
}
