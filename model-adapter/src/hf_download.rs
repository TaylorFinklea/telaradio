//! Resumable HTTP download for Hugging Face artifacts.
//!
//! Synchronous (blocking) on purpose — the wider Rust crate is sync, see
//! `core::generator`. The download:
//!
//! 1. Checks `dest` for an existing partial file. If present, sets a
//!    `Range: bytes=N-` header to resume. (This works against HF's CDN.)
//! 2. Streams the body to `dest`, periodically calling the optional
//!    progress callback with cumulative bytes written.
//! 3. Honors a `CancellationToken`: checked before the request and
//!    between body chunks. A cancelled download leaves the partial file
//!    on disk so the next call can resume.
//! 4. Validates a sha256 over the *full* file once the body is consumed.
//!    Mismatch is an error and the partial is *not* deleted (caller can
//!    re-download by passing a fresh dest path).
//!
//! Tests live in `tests/hf_download_test.rs`. They use `httpmock`; this
//! module never talks to real Hugging Face servers.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};

/// Callback invoked with cumulative bytes written so far. Fires at least
/// once per body chunk; not guaranteed to fire on every byte.
pub type ProgressCallback = Box<dyn FnMut(u64) + Send>;

/// A simple shared cancel signal. Cheap to clone; flipping it once is
/// observed by all clones.
#[derive(Clone, Default)]
pub struct CancellationToken {
    flag: Arc<AtomicBool>,
}

impl CancellationToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("download cancelled by caller")]
    Cancelled,

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("HTTP {status} from server")]
    BadStatus { status: u16 },
}

/// Download `url` to `dest`, resuming if a partial file exists.
///
/// `expected_sha256` is the hex-encoded sha256 of the *full* expected
/// file. Validation runs after the body is fully written.
///
/// `cancel` is checked before issuing the request and between body
/// chunks. A cancelled call returns [`DownloadError::Cancelled`] and
/// leaves the partial file on disk so a subsequent call can resume.
pub fn download_with_resume(
    url: &str,
    dest: &Path,
    expected_sha256: &str,
    progress: Option<ProgressCallback>,
    cancel: &CancellationToken,
) -> Result<(), DownloadError> {
    if cancel.is_cancelled() {
        return Err(DownloadError::Cancelled);
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let resume_from: u64 = std::fs::metadata(dest).map_or(0, |m| m.len());

    let client = reqwest::blocking::Client::builder()
        .build()
        .map_err(DownloadError::Http)?;

    let mut request = client.get(url);
    if resume_from > 0 {
        request = request.header("Range", format!("bytes={resume_from}-"));
    }

    let mut response = request.send()?;

    let status = response.status();
    if !status.is_success() {
        return Err(DownloadError::BadStatus {
            status: status.as_u16(),
        });
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dest)?;

    let mut written: u64 = resume_from;
    let mut progress = progress;
    let mut buf = [0_u8; 64 * 1024];

    loop {
        if cancel.is_cancelled() {
            return Err(DownloadError::Cancelled);
        }
        let n = response.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        written += n as u64;
        if let Some(cb) = progress.as_mut() {
            cb(written);
        }
    }

    file.flush()?;
    drop(file);

    let actual = sha256_file(dest)?;
    if actual != expected_sha256 {
        return Err(DownloadError::ChecksumMismatch {
            expected: expected_sha256.to_owned(),
            actual,
        });
    }

    Ok(())
}

/// Hex-encoded sha256 of a file's contents.
pub fn sha256_file(path: &Path) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex_lower(&hasher.finalize()))
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        // Two hex digits per byte; using write! here avoids pulling
        // another dependency for hex encoding.
        use std::fmt::Write as _;
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}
