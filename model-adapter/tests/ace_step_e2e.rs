//! End-to-end integration test for `AceStepGenerator` against the real
//! ACE-Step model.
//!
//! `#[ignore]`d by default because:
//! 1. The model checkpoint is ~5 GB and lives outside the repo.
//! 2. Inference takes several seconds even on Apple Silicon MPS.
//! 3. The Python venv at `model-adapter/python/.venv` must be primed
//!    via `uv sync` and the model must be downloaded into
//!    `$TELARADIO_MODEL_DIR`.
//!
//! Opt in:
//!
//! ```bash
//! TELARADIO_MODEL_DIR=/path/to/ace-step-weights \
//!     cargo test -p telaradio-model-adapter -- --include-ignored ace_step_e2e
//! ```

use telaradio_core::audio::{DEFAULT_CHANNELS, DEFAULT_SAMPLE_RATE_HZ};
use telaradio_core::generator::Generator;
use telaradio_model_adapter::AceStepGenerator;

#[test]
#[ignore = "needs ACE-Step weights at $TELARADIO_MODEL_DIR; opt in with --include-ignored"]
fn ace_step_generates_a_one_second_buffer() {
    let model_dir = std::env::var("TELARADIO_MODEL_DIR")
        .map(std::path::PathBuf::from)
        .expect("set TELARADIO_MODEL_DIR to the directory containing ACE-Step weights");

    let generator = AceStepGenerator::spawn(&model_dir).expect("spawn ace-step subprocess");
    let buf = generator
        .generate("a calm focus track, soft piano", 42, 1)
        .expect("generate audio");

    assert_eq!(buf.sample_rate, DEFAULT_SAMPLE_RATE_HZ);
    assert_eq!(buf.channels, DEFAULT_CHANNELS);
    let expected = DEFAULT_SAMPLE_RATE_HZ as usize * DEFAULT_CHANNELS as usize;
    assert_eq!(buf.samples.len(), expected, "1 second stereo @ 44.1 kHz");

    let nonzero = buf.samples.iter().filter(|s| s.abs() > 0.001).count();
    assert!(
        nonzero > expected / 10,
        "expected non-trivial signal energy, got {nonzero}/{expected}",
    );
}
