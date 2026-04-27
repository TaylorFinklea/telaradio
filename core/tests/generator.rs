//! Integration tests for `telaradio_core::generator::Generator` trait
//! and its error type. Uses a trivial in-memory implementation to exercise
//! the trait contract without depending on a model.

use telaradio_core::WavBuffer;
use telaradio_core::generator::{Generator, GeneratorError};

struct InMemoryGenerator {
    wav: WavBuffer,
}

// `id` and `version` return string literals; clippy would prefer
// `-> &'static str` on the impl, but matching the trait's `&str` keeps
// real impls (with non-static identity) viable.
#[allow(clippy::unnecessary_literal_bound)]
impl Generator for InMemoryGenerator {
    fn id(&self) -> &str {
        "in-memory"
    }

    fn version(&self) -> &str {
        "0.0.0"
    }

    fn generate(
        &self,
        _prompt: &str,
        _seed: u64,
        _duration_seconds: u32,
    ) -> Result<WavBuffer, GeneratorError> {
        Ok(self.wav.clone())
    }
}

#[test]
fn generator_trait_is_object_safe_and_callable() {
    let wav = WavBuffer {
        sample_rate: 44_100,
        channels: 2,
        samples: vec![0.5_f32; 88_200],
    };
    let g: Box<dyn Generator> = Box::new(InMemoryGenerator { wav: wav.clone() });

    assert_eq!(g.id(), "in-memory");
    assert_eq!(g.version(), "0.0.0");

    let out = g
        .generate("test prompt", 42, 1)
        .expect("in-memory generator should succeed");
    assert_eq!(out.sample_rate, 44_100);
    assert_eq!(out.channels, 2);
    assert_eq!(out.samples.len(), 88_200);
}

#[test]
fn generator_error_variants_format() {
    // Every variant should produce a human-readable Display message.
    let errs: Vec<GeneratorError> = vec![
        GeneratorError::Io(std::io::Error::other("io oops")),
        GeneratorError::Subprocess("subprocess died".into()),
        GeneratorError::Wav("malformed WAV header".into()),
        GeneratorError::ProtocolMismatch("expected ok, got err".into()),
    ];
    for e in errs {
        let s = format!("{e}");
        assert!(!s.is_empty(), "error must format to a non-empty string");
    }
}
