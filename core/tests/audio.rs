//! Integration tests for `telaradio_core::audio::WavBuffer`.

use telaradio_core::audio::{DEFAULT_CHANNELS, DEFAULT_SAMPLE_RATE_HZ, WavBuffer};

#[test]
fn wav_buffer_holds_metadata_and_samples() {
    let samples = vec![0.5_f32; 88_200]; // 1 second of stereo @ 44.1 kHz
    let buf = WavBuffer {
        sample_rate: 44_100,
        channels: 2,
        samples,
    };

    assert_eq!(buf.sample_rate, 44_100);
    assert_eq!(buf.channels, 2);
    assert_eq!(buf.samples.len(), 88_200);
}

#[test]
fn wav_buffer_duration_seconds_round_trips() {
    let buf = WavBuffer {
        sample_rate: 44_100,
        channels: 2,
        samples: vec![0.0_f32; 88_200],
    };

    assert!((buf.duration_seconds() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn wav_buffer_duration_seconds_handles_mono() {
    let buf = WavBuffer {
        sample_rate: 44_100,
        channels: 1,
        samples: vec![0.0_f32; 22_050],
    };
    assert!((buf.duration_seconds() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn defaults_match_spec() {
    assert_eq!(DEFAULT_SAMPLE_RATE_HZ, 44_100);
    assert_eq!(DEFAULT_CHANNELS, 2);
}
