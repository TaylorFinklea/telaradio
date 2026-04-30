//! Integration tests for `model_install`. Uses an `httpmock` server in
//! place of Hugging Face for the `Download` path; uses a fake model
//! directory on disk for the `UseExisting` path. No real network.

use std::fs;
use std::io::Write as _;

use httpmock::Method::GET;
use httpmock::MockServer;
use telaradio_model_adapter::model_install::{
    InstallMode, ModelArtifact, ModelInstallError, ensure_model, prompt_install_mode_cli,
};

const BODY: &[u8] = b"the quick brown fox jumps over the lazy dog";
// sha256("the quick brown fox jumps over the lazy dog")
const BODY_SHA256: &str = "05c6e08f1d9fdafa03147fcb8f82f124c76d2f70e3d989dc8aadb5e7d7450bec";

fn artifact_for(server: &MockServer) -> ModelArtifact {
    ModelArtifact {
        url: server.url("/model.safetensors"),
        relative_path: "model.safetensors".into(),
        sha256: BODY_SHA256.into(),
    }
}

#[test]
fn download_mode_writes_artifact_and_manifest() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/model.safetensors");
        then.status(200)
            .header("Content-Length", BODY.len().to_string())
            .body(BODY);
    });

    let dir = tempfile::tempdir().expect("tempdir");
    let install_dir = dir.path().join("ace-step");

    let artifacts = vec![artifact_for(&server)];
    let result =
        ensure_model(&install_dir, &artifacts, InstallMode::Download(None, None)).expect("install");

    mock.assert();
    assert_eq!(result, install_dir);
    assert_eq!(
        fs::read(install_dir.join("model.safetensors")).expect("read model"),
        BODY
    );
    assert!(install_dir.join("manifest.json").exists());
}

#[test]
fn second_call_is_idempotent_when_manifest_validates() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/model.safetensors");
        then.status(200)
            .header("Content-Length", BODY.len().to_string())
            .body(BODY);
    });

    let dir = tempfile::tempdir().expect("tempdir");
    let install_dir = dir.path().join("ace-step");
    let artifacts = vec![artifact_for(&server)];

    ensure_model(&install_dir, &artifacts, InstallMode::Download(None, None))
        .expect("first install");
    ensure_model(&install_dir, &artifacts, InstallMode::Download(None, None))
        .expect("second install");

    // First call hits the mock; second should not (manifest is valid).
    assert_eq!(mock.hits(), 1);
}

#[test]
fn use_existing_copies_files_into_install_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let install_dir = dir.path().join("ace-step");
    let source_dir = dir.path().join("user-supplied");
    fs::create_dir_all(&source_dir).expect("source dir");
    {
        let mut f = fs::File::create(source_dir.join("model.safetensors")).expect("create");
        f.write_all(BODY).expect("write");
    }

    let artifacts = vec![ModelArtifact {
        // url is unused in UseExisting mode but we keep the same shape so
        // downstream code can flip modes without rewriting the artifact list.
        url: "https://example.invalid/unused".into(),
        relative_path: "model.safetensors".into(),
        sha256: BODY_SHA256.into(),
    }];

    let result = ensure_model(
        &install_dir,
        &artifacts,
        InstallMode::UseExisting(source_dir),
    )
    .expect("install");

    assert_eq!(result, install_dir);
    assert_eq!(
        fs::read(install_dir.join("model.safetensors")).expect("read model"),
        BODY
    );
    assert!(install_dir.join("manifest.json").exists());
}

#[test]
fn corrupt_existing_manifest_triggers_redownload() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/model.safetensors");
        then.status(200)
            .header("Content-Length", BODY.len().to_string())
            .body(BODY);
    });

    let dir = tempfile::tempdir().expect("tempdir");
    let install_dir = dir.path().join("ace-step");
    let artifacts = vec![artifact_for(&server)];

    ensure_model(&install_dir, &artifacts, InstallMode::Download(None, None))
        .expect("first install");

    // Corrupt the artifact on disk and remove the model file partially.
    {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(install_dir.join("model.safetensors"))
            .expect("open model");
        f.write_all(b"junk").expect("write junk");
    }

    ensure_model(&install_dir, &artifacts, InstallMode::Download(None, None)).expect("recover");

    assert_eq!(
        fs::read(install_dir.join("model.safetensors")).expect("read model"),
        BODY,
        "corrupt model should have been re-downloaded",
    );
    assert!(mock.hits() >= 2, "second download should have hit the mock");
}

#[test]
fn prompt_parses_download_answer() {
    let mut input = b"download\n".as_slice();
    let mut output: Vec<u8> = Vec::new();
    let mode = prompt_install_mode_cli(&mut input, &mut output).expect("prompt");
    assert!(matches!(mode, InstallMode::Download(None, None)));
    let prompt = String::from_utf8(output).expect("utf8");
    assert!(prompt.contains("download"));
}

#[test]
fn prompt_parses_use_existing_answer() {
    let mut input = b"use existing /tmp/model\n".as_slice();
    let mut output: Vec<u8> = Vec::new();
    let mode = prompt_install_mode_cli(&mut input, &mut output).expect("prompt");
    match mode {
        InstallMode::UseExisting(path) => {
            assert_eq!(path, std::path::PathBuf::from("/tmp/model"));
        }
        InstallMode::Download(..) => panic!("expected UseExisting"),
    }
}

#[test]
fn prompt_rejects_garbage_input() {
    let mut input = b"banana\n".as_slice();
    let mut output: Vec<u8> = Vec::new();
    let result = prompt_install_mode_cli(&mut input, &mut output);
    assert!(result.is_err());
}

#[test]
fn use_existing_missing_file_returns_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let install_dir = dir.path().join("ace-step");
    let source_dir = dir.path().join("does-not-exist");

    let artifacts = vec![ModelArtifact {
        url: "https://example.invalid/unused".into(),
        relative_path: "model.safetensors".into(),
        sha256: BODY_SHA256.into(),
    }];

    let result = ensure_model(
        &install_dir,
        &artifacts,
        InstallMode::UseExisting(source_dir),
    );

    assert!(
        matches!(result, Err(ModelInstallError::Io(_))),
        "expected Io error for missing source, got {result:?}",
    );
}
