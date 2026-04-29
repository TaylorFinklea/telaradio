# Next steps

Phase 1 checklist lives in [`../../ROADMAP.md`](../../ROADMAP.md).
Phases 1a (recipe core), 1b (mock subprocess), 1b2 (real ACE-Step + HF
download), 1c (AM modulation DSP), and 1d MVL (macOS Swift player,
mock-only) are complete. Real ACE-Step is wired in `model-adapter` but
not yet exposed to the Swift app.

## First action: confirm audio works

`make app-run` should launch the Telaradio app. Click "Play":

1. The Mac speakers should produce an audible 440 Hz sine wave that
   pulses at 16 Hz (the modulation rate).
2. Pause / Stop should respond.

If anything sounds wrong — silence, distortion, mismatched sample
rate, the modulation isn't audible — that's the next bug to fix
before moving on.

## Recommended next: Phase 1d2 — wire real ACE-Step into the Swift app

The Rust pieces all exist (`AceStepGenerator`, `ensure_model`,
`hf_download`, `model_install`). They just need to be exposed through
the FFI and surfaced in the SwiftUI player.

1. Add FFI functions: `tr_ensure_model_download`,
   `tr_ensure_model_use_existing`, `tr_generate_ace_step`. Mirror the
   error-via-null + `tr_last_error` convention.
2. Add a "Choose model source" SwiftUI sheet for first launch:
   - "Download (~5 GB)" with a progress bar (drives `download_with_resume`'s
     callback).
   - "Use existing folder" → `NSOpenPanel` directory picker, then
     `model_install::ensure_model(InstallMode::UseExisting)`.
3. Persist the resolved model path in `UserDefaults`.
4. Replace the hardcoded `generateMock` call in `PlayerViewModel` with
   a switch: if a model path is configured, use `generateAceStep`;
   else fall back to mock. Show a setting to toggle.
5. Add a real-model integration smoke test (kept `#[ignore]`d in
   `model-adapter`; verify the Swift wiring against it manually).

## Alternative: Phase 1e — background buffer queue

Once 1d2 lands, the cold-start latency on real ACE-Step (~10s for a
4-min track on consumer hardware) will be felt every time you click
Play. The fix is the buffer queue from the original Phase 0 plan:
keep 2-3 tracks generated and modulated ahead of time during idle.

Worth doing after 1d2 — without real ACE-Step wired up, there's no
latency to mask.

## After 1d2 / 1e

- Phase 1f — Hand-seed ~20 starter recipes (lofi, ambient, electronic,
  nature-hybrid prompts). Now that the full pipeline runs, recipes
  can be ear-validated as they're authored.
- Phase 1g — Settings UI (preset selector / 3-tier intensity slider /
  advanced rate-depth-bypass panel)
- Phase 1 wrap: `PHASE_1_REPORT.md` covering ear-eval against Brain.fm

## Open follow-ups

### From Phase 1d (deferred / explicit non-goals)

- File picker for arbitrary recipes — defer until felt; current
  hardcoded path is fine for one-developer usage
- Multi-track playlist / skip-ahead — Phase 1e (buffer queue) is the
  natural place
- Now Playing / lock-screen integration — worth doing eventually,
  not v1
- Bundling the Rust static lib into a proper `.app` bundle — Phase
  4-ish (App Store distribution)

### From Phase 1b2 (still open)

- ACE-Step PyPI sdist is broken upstream; we git-pin commit
  `1bee4c9f`. Periodically check if upstream re-publishes.
- `ensure_model` is ready but not yet wired into `AceStepGenerator::spawn`;
  Phase 1d2 closes this loop.

### From Phase 1c (still open, low priority)

- `telaradio-modulate` CLI smoke binary for ear-validation (deferred
  by spec; defer until felt need)
- Recipe-schema field for configurable anti-click ramp time (Phase 2
  candidate)
