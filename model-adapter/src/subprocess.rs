//! `SubprocessGenerator` — spawns a Python child process speaking the
//! NDJSON IPC protocol from `protocol.rs`. One subprocess per generator
//! instance, held open across multiple `generate` calls.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

use telaradio_core::audio::{DEFAULT_CHANNELS, DEFAULT_SAMPLE_RATE_HZ, WavBuffer};
use telaradio_core::generator::{Generator, GeneratorError};

use crate::protocol::{Request, Response};

/// Stable id under which Phase 1b's mock surfaces in `recipe.model.id`.
/// Phase 1b2 will introduce `ace-step-1.5-xl` as a separate generator
/// implementation that ships alongside this one.
pub const MOCK_GENERATOR_ID: &str = "mock-sine";
pub const MOCK_GENERATOR_VERSION: &str = "0.1.0";

/// Held inside a Mutex because `Generator::generate` takes `&self`, and a
/// single subprocess can't service two concurrent requests.
struct IoState {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

pub struct SubprocessGenerator {
    state: Mutex<IoState>,
}

impl SubprocessGenerator {
    /// Spawn `python3 <script>` and prepare the IPC pipes. The script must
    /// implement the NDJSON protocol described in `protocol.rs`.
    pub fn spawn(script: &Path) -> Result<Self, GeneratorError> {
        let mut child = Command::new("python3")
            .arg(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| GeneratorError::Subprocess("subprocess stdin missing".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| GeneratorError::Subprocess("subprocess stdout missing".into()))?;

        Ok(Self {
            state: Mutex::new(IoState {
                child,
                stdin,
                stdout: BufReader::new(stdout),
            }),
        })
    }
}

impl Generator for SubprocessGenerator {
    fn id(&self) -> &str {
        MOCK_GENERATOR_ID
    }

    fn version(&self) -> &str {
        MOCK_GENERATOR_VERSION
    }

    fn generate(
        &self,
        prompt: &str,
        seed: u64,
        duration_seconds: u32,
    ) -> Result<WavBuffer, GeneratorError> {
        let req = Request {
            prompt: prompt.to_owned(),
            seed,
            duration_seconds,
            sample_rate: DEFAULT_SAMPLE_RATE_HZ,
            channels: DEFAULT_CHANNELS,
        };
        let req_json = serde_json::to_string(&req)
            .map_err(|e| GeneratorError::ProtocolMismatch(format!("serialize request: {e}")))?;

        let mut state = self
            .state
            .lock()
            .map_err(|_| GeneratorError::Subprocess("ipc mutex poisoned".into()))?;

        writeln!(state.stdin, "{req_json}")?;
        state.stdin.flush()?;

        let mut response_line = String::new();
        let n = state.stdout.read_line(&mut response_line)?;
        if n == 0 {
            return Err(GeneratorError::Subprocess(
                "subprocess closed stdout before responding".into(),
            ));
        }

        let response: Response = serde_json::from_str(response_line.trim()).map_err(|e| {
            GeneratorError::ProtocolMismatch(format!(
                "deserialize response: {e}; line was: {}",
                response_line.trim()
            ))
        })?;

        match response {
            Response::Ok {
                wav_path,
                sample_rate,
                channels,
            } => {
                let buffer = read_wav(&wav_path, sample_rate, channels)?;
                // Best-effort cleanup; ignore errors (e.g. if the subprocess
                // already removed it on a future hardening pass).
                let _ = std::fs::remove_file(&wav_path);
                Ok(buffer)
            }
            Response::Err { message } => Err(GeneratorError::Subprocess(message)),
        }
    }
}

impl Drop for SubprocessGenerator {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            // Best-effort: signal the child to terminate, then reap it.
            let _ = state.child.kill();
            let _ = state.child.wait();
        }
    }
}

fn read_wav(
    path: &PathBuf,
    expected_rate: u32,
    expected_channels: u8,
) -> Result<WavBuffer, GeneratorError> {
    let reader = hound::WavReader::open(path)
        .map_err(|e| GeneratorError::Wav(format!("open {}: {e}", path.display())))?;
    let spec = reader.spec();

    if spec.sample_rate != expected_rate {
        return Err(GeneratorError::Wav(format!(
            "expected {expected_rate} Hz, got {} Hz",
            spec.sample_rate
        )));
    }
    if spec.channels != u16::from(expected_channels) {
        return Err(GeneratorError::Wav(format!(
            "expected {expected_channels} channels, got {}",
            spec.channels
        )));
    }

    // Mock writes 16-bit signed PCM; convert to f32 in [-1.0, 1.0].
    let samples: Result<Vec<f32>, _> = reader
        .into_samples::<i16>()
        .map(|r| r.map(|s| f32::from(s) / f32::from(i16::MAX)))
        .collect();
    let samples = samples.map_err(|e| GeneratorError::Wav(format!("read samples: {e}")))?;

    Ok(WavBuffer {
        sample_rate: spec.sample_rate,
        channels: expected_channels,
        samples,
    })
}
