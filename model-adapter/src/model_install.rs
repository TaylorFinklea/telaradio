//! `ensure_model` — install ACE-Step weights into a canonical directory.
//!
//! Two modes:
//! - [`InstallMode::Download`] streams artifacts from Hugging Face via
//!   `hf_download::download_with_resume` (resumable + checksum-verified).
//! - [`InstallMode::UseExisting`] copies a user-supplied directory of
//!   weights into the install dir (for air-gapped installs / users who
//!   already have the model).
//!
//! After all artifacts are present and validated, a `manifest.json` is
//! written. On subsequent calls, if the manifest matches every
//! artifact's recorded sha256, install is a no-op. If validation fails
//! (file missing, checksum drifted), the artifact is re-fetched.
//!
//! See `tests/model_install_test.rs` for the full behavioral contract.

use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::hf_download::{
    CancellationToken, DownloadError, ProgressCallback, download_with_resume, sha256_file,
};

const MANIFEST_FILE: &str = "manifest.json";

/// One file inside a model. The download URL, where it lives relative to
/// the install dir, and its sha256.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelArtifact {
    pub url: String,
    pub relative_path: PathBuf,
    pub sha256: String,
}

/// How `ensure_model` should populate the install dir.
pub enum InstallMode {
    /// Download from Hugging Face. Optional progress callback fires per
    /// chunk per file; cumulative bytes per file (not aggregated).
    Download(Option<ProgressCallback>),

    /// Copy from an existing local directory. Each artifact's
    /// `relative_path` is read from this source dir.
    UseExisting(PathBuf),
}

#[derive(Debug, thiserror::Error)]
pub enum ModelInstallError {
    #[error("download error: {0}")]
    Download(#[from] DownloadError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("manifest serialize/deserialize error: {0}")]
    Manifest(#[from] serde_json::Error),
}

/// Ensure every artifact is present, validates by sha256, and a
/// `manifest.json` records the result. Returns the install dir.
///
/// Idempotent: if the manifest already lists every artifact and each
/// artifact's on-disk sha256 matches, this returns immediately.
pub fn ensure_model(
    install_dir: &Path,
    artifacts: &[ModelArtifact],
    mode: InstallMode,
) -> Result<PathBuf, ModelInstallError> {
    fs::create_dir_all(install_dir)?;

    if manifest_validates(install_dir, artifacts).unwrap_or(false) {
        return Ok(install_dir.to_owned());
    }

    match mode {
        InstallMode::Download(mut progress) => {
            let cancel = CancellationToken::new();
            for artifact in artifacts {
                let dest = install_dir.join(&artifact.relative_path);
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                // Discard a stale partial / corrupt file before resuming
                // — sha256 validation in `hf_download` will catch the
                // mismatch but starting from a verified clean slate also
                // saves bandwidth on a re-download.
                if dest.exists() && sha256_file(&dest)? != artifact.sha256 {
                    fs::remove_file(&dest)?;
                }
                download_with_resume(
                    &artifact.url,
                    &dest,
                    &artifact.sha256,
                    progress.take(),
                    &cancel,
                )?;
            }
        }
        InstallMode::UseExisting(source_dir) => {
            for artifact in artifacts {
                let src = source_dir.join(&artifact.relative_path);
                let dst = install_dir.join(&artifact.relative_path);
                if let Some(parent) = dst.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&src, &dst)?;
                let actual = sha256_file(&dst)?;
                if actual != artifact.sha256 {
                    return Err(ModelInstallError::Download(DownloadError::ChecksumMismatch {
                        expected: artifact.sha256.clone(),
                        actual,
                    }));
                }
            }
        }
    }

    write_manifest(install_dir, artifacts)?;
    Ok(install_dir.to_owned())
}

/// CLI helper: prompt on stderr, read one line on stdin, return the
/// chosen [`InstallMode`]. Phase 1d will replace this with a UI.
///
/// Lines accepted (case-insensitive, trimmed):
/// - `download`
/// - `use existing <path>`
///
/// # Errors
///
/// Returns an [`std::io::Error`] if reading from `reader` fails or the
/// input doesn't match either of the accepted formats.
pub fn prompt_install_mode_cli<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
) -> std::io::Result<InstallMode> {
    writeln!(writer, "Telaradio needs the ACE-Step model (~5 GB).")?;
    writeln!(writer, "  download                — fetch from Hugging Face")?;
    writeln!(
        writer,
        "  use existing <path>     — copy from an existing directory",
    )?;
    writer.flush()?;

    let mut line = String::new();
    reader.read_line(&mut line)?;
    let trimmed = line.trim();

    if trimmed.eq_ignore_ascii_case("download") {
        return Ok(InstallMode::Download(None));
    }
    if let Some(rest) = trimmed
        .strip_prefix("use existing ")
        .or_else(|| trimmed.strip_prefix("USE EXISTING "))
    {
        return Ok(InstallMode::UseExisting(PathBuf::from(rest.trim())));
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!("unrecognized install-mode answer: {trimmed:?}"),
    ))
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    artifacts: Vec<ModelArtifact>,
}

fn write_manifest(install_dir: &Path, artifacts: &[ModelArtifact]) -> Result<(), ModelInstallError> {
    let manifest = Manifest {
        artifacts: artifacts.to_vec(),
    };
    let json = serde_json::to_string_pretty(&manifest)?;
    let mut file = fs::File::create(install_dir.join(MANIFEST_FILE))?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

fn read_manifest(install_dir: &Path) -> Result<Manifest, ModelInstallError> {
    let bytes = fs::read(install_dir.join(MANIFEST_FILE))?;
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    Ok(manifest)
}

fn manifest_validates(
    install_dir: &Path,
    expected: &[ModelArtifact],
) -> Result<bool, ModelInstallError> {
    let manifest = read_manifest(install_dir)?;
    if manifest.artifacts.len() != expected.len() {
        return Ok(false);
    }
    for (a, b) in manifest.artifacts.iter().zip(expected.iter()) {
        if a.relative_path != b.relative_path || a.sha256 != b.sha256 {
            return Ok(false);
        }
        let path = install_dir.join(&a.relative_path);
        if !path.exists() {
            return Ok(false);
        }
        let actual = sha256_file(&path)?;
        if actual != a.sha256 {
            return Ok(false);
        }
    }
    Ok(true)
}
