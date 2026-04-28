//! Integration tests for `hf_download`. Mocks the Hugging Face HTTP API
//! with `httpmock` — never hits real servers.
//!
//! Coverage:
//! - Plain GET writes a complete file (smoke).
//! - A partial file on disk + a `Range` header to resume — final
//!   content matches.
//! - Checksum validation: corrupt body → error; matching body → ok.
//! - Cancellation token: a pre-cancelled token aborts the download.
//! - Progress callback fires with monotonically growing values.

use std::fs;
use std::io::Write as _;
use std::sync::{Arc, Mutex};

use httpmock::Method::GET;
use httpmock::MockServer;
use telaradio_model_adapter::hf_download::{
    CancellationToken, DownloadError, ProgressCallback, download_with_resume,
};

const BODY: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
// sha256("abcdefghijklmnopqrstuvwxyz0123456789")
const BODY_SHA256: &str = "011fc2994e39d251141540f87a69092b3f22a86767f7283de7eeedb3897bedf6";

#[test]
fn downloads_a_full_file_when_target_does_not_exist() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/model.safetensors");
        then.status(200)
            .header("Content-Length", BODY.len().to_string())
            .body(BODY);
    });

    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("model.safetensors");

    download_with_resume(
        &server.url("/model.safetensors"),
        &dest,
        BODY_SHA256,
        None,
        &CancellationToken::new(),
    )
    .expect("download");

    mock.assert();
    let got = fs::read(&dest).expect("read dest");
    assert_eq!(got, BODY);
}

#[test]
fn resumes_download_from_existing_partial_file() {
    let server = MockServer::start();
    let prefix_len: usize = 10;
    let suffix = &BODY[prefix_len..];

    // Server only serves bytes 10.. when asked with a Range header, mimicking
    // a real HF mirror. If the client incorrectly issued a full GET, this
    // mock won't match and httpmock will surface that.
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/model.safetensors")
            .header("Range", format!("bytes={prefix_len}-"));
        then.status(206)
            .header("Content-Length", suffix.len().to_string())
            .header(
                "Content-Range",
                format!("bytes {prefix_len}-{}/{}", BODY.len() - 1, BODY.len()),
            )
            .body(suffix);
    });

    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("model.safetensors");

    // Pre-write the first 10 bytes to simulate an interrupted download.
    {
        let mut f = fs::File::create(&dest).expect("create partial");
        f.write_all(&BODY[..prefix_len]).expect("write partial");
    }

    download_with_resume(
        &server.url("/model.safetensors"),
        &dest,
        BODY_SHA256,
        None,
        &CancellationToken::new(),
    )
    .expect("download");

    mock.assert();
    let got = fs::read(&dest).expect("read dest");
    assert_eq!(got, BODY);
}

#[test]
fn checksum_mismatch_returns_error() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/model.safetensors");
        then.status(200)
            .header("Content-Length", BODY.len().to_string())
            .body(BODY);
    });

    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("model.safetensors");

    let result = download_with_resume(
        &server.url("/model.safetensors"),
        &dest,
        // wrong checksum
        "0000000000000000000000000000000000000000000000000000000000000000",
        None,
        &CancellationToken::new(),
    );

    match result {
        Err(DownloadError::ChecksumMismatch { .. }) => {}
        other => panic!("expected ChecksumMismatch, got {other:?}"),
    }
}

#[test]
fn pre_cancelled_token_aborts_before_request() {
    let server = MockServer::start();
    // No mock — if we hit the network the request will fail with 404,
    // but we expect the cancel check to short-circuit before that.

    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("model.safetensors");

    let token = CancellationToken::new();
    token.cancel();

    let result = download_with_resume(
        &server.url("/model.safetensors"),
        &dest,
        BODY_SHA256,
        None,
        &token,
    );

    assert!(matches!(result, Err(DownloadError::Cancelled)));
}

#[test]
fn progress_callback_fires_with_monotonically_increasing_values() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/model.safetensors");
        then.status(200)
            .header("Content-Length", BODY.len().to_string())
            .body(BODY);
    });

    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("model.safetensors");

    let calls: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_for_cb = Arc::clone(&calls);
    let cb: ProgressCallback = Box::new(move |bytes| {
        calls_for_cb.lock().expect("mutex").push(bytes);
    });

    download_with_resume(
        &server.url("/model.safetensors"),
        &dest,
        BODY_SHA256,
        Some(cb),
        &CancellationToken::new(),
    )
    .expect("download");

    let calls = calls.lock().expect("mutex").clone();
    assert!(!calls.is_empty(), "progress callback never fired");
    for w in calls.windows(2) {
        assert!(w[0] <= w[1], "progress went backward: {calls:?}");
    }
    assert_eq!(*calls.last().unwrap(), BODY.len() as u64);
}
