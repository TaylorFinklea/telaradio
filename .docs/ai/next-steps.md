# Next steps

Phase 1 checklist lives in [`../../ROADMAP.md`](../../ROADMAP.md).
Phases 1a (recipe core), 1b (Generator trait + mock subprocess), 1b2
(real ACE-Step + HF download), and 1c (AM modulation DSP) are
complete and merged into `main`.

## Recommended next: Phase 1d — macOS Swift player shell

With the DSP done, ACE-Step wired up, and the mock available for fast
tests, the natural next phase is a minimal native macOS app that:

1. Loads a `Recipe` via `Recipe::parse`.
2. Resolves the ACE-Step model dir via `model_install::ensure_model`
   (replacing `prompt_install_mode_cli` with a real SwiftUI prompt).
3. Calls a generator (`AceStepGenerator` or the mock for tests) via
   FFI or a small Rust shim.
4. Applies AM modulation (`dsp::apply_am`) per `recipe.modulation`.
5. Plays the resulting `WavBuffer` via AVFoundation with basic
   transport controls (load, play, pause, skip).

Decisions to make at the start of Phase 1d:

- **FFI surface**: thin Rust C API via `cbindgen` (Swift consumes it
  through a bridging header) vs. spawn the Rust binary as a subprocess
  and stream WAV over stdout. The C-ABI option is more idiomatic for a
  native app; the subprocess option is faster to ship.
- **Sample-rate mismatch handling**: ACE-Step output is 44.1 kHz, but
  AVFoundation's preferred internal rate on Apple Silicon may be 48
  kHz. Decide whether to resample at playback or accept the conversion.
- **First-launch UX**: the model install prompt becomes a SwiftUI
  flow. Sketch the screens before building.

## Optional: prime the ACE-Step environment

Before Phase 1d (or anytime), to validate the full real-model pipeline:

1. `cd model-adapter/python && uv sync` — first-time install is heavy
   (multi-GB venv, ~170 deps).
2. Download ACE-Step weights from
   `https://huggingface.co/ACE-Step/ACE-Step-v1-3.5B` into a chosen
   `$TELARADIO_MODEL_DIR`.
3. Run the e2e test: `TELARADIO_MODEL_DIR=$DIR cargo test -p
   telaradio-model-adapter -- --include-ignored ace_step_e2e`.

Skip if you'd rather wait until Phase 1d's Swift app exercises it.

## After Phase 1d

- Phase 1e — Background buffer queue (keep 2–3 tracks ahead generated
  and modulated during idle time)
- Phase 1f — Hand-seed ~20 starter recipes (lofi, ambient, electronic,
  nature-hybrid prompts)
- Phase 1g — Settings UI (preset selector / 3-tier intensity slider /
  advanced rate-depth-bypass panel)
- Phase 1 wrap: `PHASE_1_REPORT.md` covering ear-eval against Brain.fm

## Open follow-ups from Phase 1b2

- **Model id mismatch.** The constant `ACE_STEP_GENERATOR_ID` is
  `"ace-step-1.5-xl"` (matches the recipe schema's existing pin) but
  the actual model on Hugging Face is
  `ACE-Step/ACE-Step-v1-3.5B`. Decide whether the id should track the
  parameter count, the marketing name, or stay opaque as an internal
  alias. Recipes pinning the current id are the constraint — once
  recipes are authored against an id, it is effectively locked.
  `recipes/example-foggy-lofi.json` also pins `model.id =
  "ace-step-1.5-xl"`.
- **ACE-Step PyPI sdist is broken.** `setup.py` reads a
  `requirements.txt` not in the tarball. We pin a GitHub commit
  (`1bee4c9f`) instead via `[tool.uv.sources]`. Periodically check if
  upstream has re-published; swap to a plain PyPI version if so.
- **`ensure_model` is not yet wired into `AceStepGenerator::spawn`.**
  The pieces exist; the glue lands in Phase 1d when first-launch UX
  becomes real.

## Open follow-ups from Phase 1c

- `telaradio-modulate` CLI smoke binary for ear-validation (deferred
  by spec; defer until felt need).
- Recipe-schema field for configurable anti-click ramp time (Phase 2
  candidate; current 1 ms hard-coded constant is fine for v1).
