#!/usr/bin/env python3
"""Telaradio Python subprocess (Phase 1b mock).

Reads NDJSON requests on stdin, writes NDJSON responses on stdout. The
mock engine generates a 440 Hz sine wave at the requested sample rate,
channel count, and duration; the response includes the path to a temp
WAV file that the caller is responsible for deleting.

Phase 1b2 swaps this mock for real ACE-Step inference behind the same
IPC contract. The protocol does not change.
"""

from __future__ import annotations

import array
import json
import math
import os
import sys
import tempfile
import wave
from pathlib import Path
from typing import TypedDict

MOCK_FREQ_HZ = 440.0
SAMPLE_AMPLITUDE = 0.5
PCM16_MAX = 32_767
SAMPLE_WIDTH_BYTES = 2  # 16-bit signed PCM


class RequestDict(TypedDict):
    """Wire format for a generation request from the Rust adapter."""

    prompt: str
    seed: int
    duration_seconds: int
    sample_rate: int
    channels: int


class OkResponseDict(TypedDict):
    """Wire format for a successful generation response."""

    kind: str
    wav_path: str
    sample_rate: int
    channels: int


class ErrResponseDict(TypedDict):
    """Wire format for a failure response."""

    kind: str
    message: str


def generate_sine_pcm(sample_rate: int, channels: int, duration_seconds: int) -> bytes:
    """Generate a 16-bit PCM sine wave as interleaved channel bytes."""
    n_frames = sample_rate * duration_seconds
    omega = 2.0 * math.pi * MOCK_FREQ_HZ / sample_rate
    amp = int(SAMPLE_AMPLITUDE * PCM16_MAX)

    samples = array.array("h")
    for i in range(n_frames):
        v = int(amp * math.sin(omega * i))
        for _ in range(channels):
            samples.append(v)
    return samples.tobytes()


def write_wav(path: Path, sample_rate: int, channels: int, pcm: bytes) -> None:
    """Write 16-bit PCM bytes as a WAV file."""
    with wave.open(str(path), "wb") as wav:
        wav.setnchannels(channels)
        wav.setsampwidth(SAMPLE_WIDTH_BYTES)
        wav.setframerate(sample_rate)
        wav.writeframes(pcm)


def handle_request(req: RequestDict) -> OkResponseDict:
    """Mock-generate audio for one request and return the response payload."""
    sample_rate = int(req["sample_rate"])
    channels = int(req["channels"])
    duration_seconds = int(req["duration_seconds"])

    pcm = generate_sine_pcm(sample_rate, channels, duration_seconds)

    fd, path_str = tempfile.mkstemp(prefix="telaradio_", suffix=".wav")
    os.close(fd)
    path = Path(path_str)
    write_wav(path, sample_rate, channels, pcm)

    return {
        "kind": "ok",
        "wav_path": str(path),
        "sample_rate": sample_rate,
        "channels": channels,
    }


def main() -> None:
    """Run the NDJSON request/response loop on stdin/stdout."""
    for line in sys.stdin:
        stripped = line.strip()
        if not stripped:
            continue
        res: OkResponseDict | ErrResponseDict
        try:
            req: RequestDict = json.loads(stripped)
            res = handle_request(req)
        except (json.JSONDecodeError, KeyError, ValueError) as exc:
            res = ErrResponseDict(
                kind="err",
                message=f"{type(exc).__name__}: {exc}",
            )
        sys.stdout.write(json.dumps(res) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
