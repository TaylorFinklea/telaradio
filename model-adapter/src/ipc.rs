//! Shared NDJSON-over-stdio IPC machinery for the Python subprocesses
//! that back our `Generator` impls.
//!
//! Both `subprocess::SubprocessGenerator` (mock-sine) and
//! `ace_step::AceStepGenerator` (real ACE-Step) compose this — the only
//! thing that varies between them is the Python script + how the
//! Python interpreter is located + the generator id/version. The IPC
//! itself is identical: write one JSON `Request` per line, read one
//! JSON `Response` per line, audio crosses by temp WAV path.
//!
//! Held inside a `Mutex` because `Generator::generate` takes `&self`,
//! and a single subprocess can't service two concurrent requests.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

use telaradio_core::audio::{DEFAULT_CHANNELS, DEFAULT_SAMPLE_RATE_HZ, WavBuffer};
use telaradio_core::generator::GeneratorError;

use crate::protocol::{Request, Response};

pub(crate) struct IoState {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

pub(crate) struct IpcChannel {
    state: Mutex<IoState>,
}

impl IpcChannel {
    /// Spawn `python_exe <script>` (with optional extra args) and prepare
    /// the IPC pipes.
    pub(crate) fn spawn(
        python_exe: &Path,
        script: &Path,
        extra_args: &[&str],
    ) -> Result<Self, GeneratorError> {
        let mut command = Command::new(python_exe);
        command
            .arg(script)
            .args(extra_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        Self::spawn_with_command(command)
    }

    /// Spawn from a pre-configured [`Command`] (so callers can set env
    /// vars / cwd before us). The command must already have stdin and
    /// stdout piped.
    pub(crate) fn spawn_with_command(mut command: Command) -> Result<Self, GeneratorError> {
        let mut child = command.spawn()?;

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

    /// Send one request, wait for one response, and decode the WAV.
    pub(crate) fn generate(
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
                let _ = std::fs::remove_file(&wav_path);
                Ok(buffer)
            }
            Response::Err { message } => Err(GeneratorError::Subprocess(message)),
        }
    }
}

impl Drop for IpcChannel {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
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

    // 16-bit signed PCM → f32 in [-1.0, 1.0].
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
