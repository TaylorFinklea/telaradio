# Next steps

Phase 1 checklist lives in [`../../ROADMAP.md`](../../ROADMAP.md).
Phase 1a (recipe core), 1b (Generator trait + mock subprocess), and
1b2 (real ACE-Step + HF download) are complete. Phase 1c (AM
modulation DSP) is being built in parallel on a sibling worktree.

## Recommended next: Phase 1d — macOS Swift app shell

Once 1c lands, Phase 1d is the natural next step: a minimal native
macOS app that loads a recipe, calls `AceStepGenerator` (or the mock)
via FFI / a small Rust shim, applies the AM modulation from 1c, and
plays the result.

1. Decide the FFI surface: a thin Rust C API (`cbindgen`) or a Swift
   package wrapping a Rust static lib.
2. Player UI: load recipe button, play / pause / skip controls. No
   library browsing, no settings panel — those are Phase 1g.
3. Wire `model_install::ensure_model` into first launch with
   `prompt_install_mode_cli` replaced by a real SwiftUI prompt.

## Alternative: prime the ACE-Step environment for the user

Before merging 1b2, the user may want to:

1. `cd model-adapter/python && uv sync` — first-time install is heavy
   (multi-GB venv, ~170 deps).
2. Download the ACE-Step weights manually from
   `https://huggingface.co/ACE-Step/ACE-Step-v1-3.5B`, into a chosen
   `$TELARADIO_MODEL_DIR`.
3. Run the e2e test: `TELARADIO_MODEL_DIR=$DIR cargo test -p
   telaradio-model-adapter -- --include-ignored ace_step_e2e`.

This validates the full pipeline against the real model. Skip if
you're happy waiting until Phase 1d when the Swift app exercises it.

## After 1c / 1d

- Phase 1e — Background buffer queue (keep 2–3 tracks ahead generated
  and modulated during idle time)
- Phase 1f — Hand-seed ~20 starter recipes
- Phase 1g — Settings UI (preset / 3-tier intensity / advanced
  rate/depth/bypass panel)

## Open follow-ups from Phase 1b2

- The `ACE_STEP_GENERATOR_ID` is `"ace-step-1.5-xl"` but the actual
  model is `ACE-Step/ACE-Step-v1-3.5B`. Decide whether the id should
  track the parameter count, the marketing name, or stay opaque.
  Recipes pinning the current id are the constraint — once they're
  authored, the id is locked.
- The ACE-Step PyPI sdist is broken (missing `requirements.txt`). We
  pin a GitHub commit instead. Periodically re-check if upstream has
  re-published; if so, swap the `[tool.uv.sources]` entry for a plain
  PyPI version.
- `ensure_model` populates an install dir but nothing yet wires it
  into `AceStepGenerator::spawn`. Phase 1d is the natural place to
  glue `ensure_model` → `spawn(model_dir)`.
