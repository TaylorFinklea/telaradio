#!/usr/bin/env python3
"""Telaradio Python subprocess (Phase 1b2 — real ACE-Step engine).

Speaks the same NDJSON protocol as `telaradio_subprocess.py` (the mock
engine from Phase 1b). One JSON request per line on stdin, one JSON
response per line on stdout. Audio crosses the boundary by temp-file
path: this script writes a 16-bit PCM WAV and returns the path; the
Rust caller reads + deletes it.

The ACE-Step pipeline is loaded lazily on the first request so the
subprocess can start cheaply (and so a `--probe` invocation does not
need GPU memory). The model checkpoint directory is read from the
`TELARADIO_MODEL_DIR` environment variable; if unset, ACE-Step falls
back to its own default download path.

Usage:
    python telaradio_ace_step.py            # NDJSON IPC loop
    python telaradio_ace_step.py --probe    # print engine version + exit
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING, Any, TypedDict

if TYPE_CHECKING:
    from collections.abc import Callable

ACE_STEP_GENERATOR_VERSION = "1.0.0"


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


def _load_pipeline() -> Callable[[RequestDict, Path], None]:
    """Construct an `ACEStepPipeline` and return a closure that runs it.

    Imports happen lazily so `--probe` does not pay the torch / diffusers
    import cost. The closure writes the generated WAV to the given path.
    """
    # Imports are local on purpose — see docstring.
    from acestep.pipeline_ace_step import ACEStepPipeline  # noqa: PLC0415

    checkpoint_dir = os.environ.get("TELARADIO_MODEL_DIR")
    pipeline = ACEStepPipeline(
        checkpoint_dir=checkpoint_dir,
        dtype="float32",
        torch_compile=False,
    )

    def run(req: RequestDict, out_path: Path) -> None:
        # ACEStepPipeline.__call__ writes the audio file directly; we pass
        # the duration in seconds and let the pipeline pick its sample
        # rate. We then rely on the Rust side to verify the rate matches
        # the request.
        pipeline(
            prompt=req["prompt"],
            audio_duration=float(req["duration_seconds"]),
            manual_seeds=[int(req["seed"])],
            save_path=str(out_path),
        )

    return run


def handle_request(
    req: RequestDict, runner: Callable[[RequestDict, Path], None]
) -> OkResponseDict:
    """Run one generation request and return the response payload."""
    fd, path_str = tempfile.mkstemp(prefix="telaradio_ace_", suffix=".wav")
    os.close(fd)
    out_path = Path(path_str)

    runner(req, out_path)

    return {
        "kind": "ok",
        "wav_path": str(out_path),
        "sample_rate": int(req["sample_rate"]),
        "channels": int(req["channels"]),
    }


def main_loop(runner_factory: Callable[[], Callable[[RequestDict, Path], None]]) -> None:
    """Run the NDJSON request/response loop on stdin/stdout.

    The runner is constructed on the first request (lazy model load) so
    the subprocess can start without paying the model load cost until
    the caller asks for audio.
    """
    runner: Callable[[RequestDict, Path], None] | None = None
    for line in sys.stdin:
        stripped = line.strip()
        if not stripped:
            continue
        res: OkResponseDict | ErrResponseDict
        try:
            req: RequestDict = json.loads(stripped)
            if runner is None:
                runner = runner_factory()
            res = handle_request(req, runner)
        except (json.JSONDecodeError, KeyError, ValueError, OSError) as exc:
            res = ErrResponseDict(
                kind="err",
                message=f"{type(exc).__name__}: {exc}",
            )
        except Exception as exc:  # noqa: BLE001  -- want to surface any model error
            res = ErrResponseDict(
                kind="err",
                message=f"{type(exc).__name__}: {exc}",
            )
        sys.stdout.write(json.dumps(res) + "\n")
        sys.stdout.flush()


def probe() -> None:
    """Print engine version metadata and exit (no model load)."""
    info: dict[str, Any] = {
        "engine": "ace-step",
        "version": ACE_STEP_GENERATOR_VERSION,
    }
    sys.stdout.write(json.dumps(info) + "\n")
    sys.stdout.flush()


def main() -> None:
    """Parse args; either probe or enter the NDJSON loop."""
    parser = argparse.ArgumentParser(description="Telaradio ACE-Step subprocess.")
    parser.add_argument(
        "--probe",
        action="store_true",
        help="Print engine version metadata and exit (no model load).",
    )
    args = parser.parse_args()

    if args.probe:
        probe()
        return

    main_loop(_load_pipeline)


if __name__ == "__main__":
    main()
