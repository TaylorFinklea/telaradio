//! Round-trip tests for the NDJSON IPC types between Rust and the Python
//! subprocess. Wire format must stay stable — these tests pin it.

use std::path::PathBuf;
use telaradio_model_adapter::protocol::{Request, Response};

#[test]
fn request_round_trips_through_json() {
    let req = Request {
        prompt: "warm vinyl lofi".into(),
        seed: 1_893_421,
        duration_seconds: 1,
        sample_rate: 44_100,
        channels: 2,
    };

    let json = serde_json::to_string(&req).expect("serialize");
    let back: Request = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(req, back);
}

#[test]
fn response_ok_uses_kind_tag() {
    let res = Response::Ok {
        wav_path: PathBuf::from("/tmp/telaradio_xyz.wav"),
        sample_rate: 44_100,
        channels: 2,
    };
    let json = serde_json::to_string(&res).expect("serialize");
    assert!(
        json.contains(r#""kind":"ok""#),
        "expected kind=ok tag in {json}"
    );
}

#[test]
fn response_err_uses_kind_tag() {
    let res = Response::Err {
        message: "boom".into(),
    };
    let json = serde_json::to_string(&res).expect("serialize");
    assert!(
        json.contains(r#""kind":"err""#),
        "expected kind=err tag in {json}"
    );
    assert!(json.contains(r#""message":"boom""#));
}

#[test]
fn response_round_trips_both_variants() {
    let ok = Response::Ok {
        wav_path: PathBuf::from("/tmp/telaradio_xyz.wav"),
        sample_rate: 44_100,
        channels: 2,
    };
    let err = Response::Err {
        message: "boom".into(),
    };

    for r in [&ok, &err] {
        let json = serde_json::to_string(r).expect("serialize");
        let back: Response = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*r, back);
    }
}

#[test]
fn response_rejects_unknown_kind() {
    let json = r#"{"kind":"banana","wav_path":"/tmp/x.wav","sample_rate":44100,"channels":2}"#;
    let result: Result<Response, _> = serde_json::from_str(json);
    assert!(result.is_err(), "unknown kind should fail to parse");
}
