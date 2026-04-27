//! NDJSON IPC types between Rust and the Python subprocess.
//!
//! Wire format is one JSON document per line on stdin/stdout. Audio data
//! crosses the boundary by file path (the subprocess writes a temp WAV
//! and returns the path; the adapter reads + deletes it).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub prompt: String,
    pub seed: u64,
    pub duration_seconds: u32,
    pub sample_rate: u32,
    pub channels: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Response {
    Ok {
        wav_path: PathBuf,
        sample_rate: u32,
        channels: u8,
    },
    Err {
        message: String,
    },
}
