# Next steps

Phase 1 checklist lives in [`../../ROADMAP.md`](../../ROADMAP.md).
Phases 1a (recipe core), 1b (mock subprocess), 1b2 (real ACE-Step + HF
download), 1c (AM modulation DSP), 1d MVL (macOS Swift player), and
**1d2 (real ACE-Step in the Swift app)** are complete. Mock pipeline
is verified end-to-end through Mac speakers (2026-04-29). Real-model
audible verification still owed once sha256s are bootstrapped.

## First action: bootstrap real ACE-Step sha256s

Phase 1d2 shipped with **placeholder sha256s** in
`model-adapter/src/ace_step.rs::ace_step_artifacts()`. Until real values
land, both the Download and Use-existing paths in the SwiftUI sheet
fail validation. The fix is a one-time bootstrap:

1. Manually download ACE-Step v1 3.5B once (e.g.
   `huggingface-cli download ACE-Step/ACE-Step-v1-3.5B --local-dir /tmp/ace`).
2. `sha256sum` over each artifact listed in `ace_step_artifacts()`.
3. Paste the hex strings into the function.
4. Commit. After this, the "Use existing folder" sheet button works
   end-to-end against the locally-downloaded folder, and the "Download"
   button works against the HF CDN.

Sized for a Sonnet sub-agent: `bootstrap-ace-step-checksums` task.

## Then: confirm Phase 1d2 audibly

`defaults delete com.telaradio.Telaradio` (clears UserDefaults), then
`make app-run`. Verify in order:

1. The first-launch sheet appears with three buttons.
2. "Use mock for now" dismisses the sheet; Play produces the 1d MVL
   sine (regression check).
3. Re-clear UserDefaults; pick "Use existing folder" against the
   downloaded model dir; Play generates real audio modulated at 16 Hz.

## Recommended next phase: Phase 1e — background buffer queue

Real ACE-Step has ~10 s cold-start latency on consumer hardware. The
fix from the original Phase 0 plan: keep 2–3 tracks pre-generated and
modulated ahead of the user during idle. Hooks in at the
`Telaradio.generateAceStep(...)` seam — no FFI changes needed.

## After 1d2 / 1e

- Phase 1f — Hand-seed ~20 starter recipes (lofi, ambient, electronic,
  nature-hybrid prompts). Now that the full pipeline runs, recipes
  can be ear-validated as they're authored.
- Phase 1g — Settings UI (preset selector / 3-tier intensity slider /
  advanced rate-depth-bypass panel)
- Phase 1 wrap: `PHASE_1_REPORT.md` covering ear-eval against Brain.fm

## Open follow-ups

### From Phase 1d2 (deferred / explicit non-goals)

- **Real sha256s for `ace_step_artifacts()`** — see "First action" above.
  Blocks the real-model audio path; not blocking ship of 1d2 code.
- **macOS deployment target mismatch** — Rust static lib targets host
  (26.0) while SwiftPM links 13.0. `ld: warning` is noisy but harmless.
  Fix: pin `-mmacosx-version-min=13.0` in `.cargo/config.toml` rustflags.
- **"Change model…" affordance** — `defaults delete com.telaradio.Telaradio`
  is the workaround until Phase 1g.
- **Download cancel UI** — cancel token exists at the FFI; the Swift
  wrapper creates and frees its own internally. Phase 1g surfaces a
  Cancel button in the Download progress view.
- **`settings.modelDir!` force-unwrap** — safe today (sheet blocks Play
  until configured) but fragile once Phase 1g lets users clear the path.
  Replace with a `guard let` + clear error.

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
- `ensure_model` and `AceStepGenerator::spawn` are now both reachable
  from Swift; the Swift wrapper calls them in sequence (ensure first,
  then spawn-use-drop). The original "wire ensure_model into spawn"
  proposal was rejected — keeping them separate makes the FFI
  surface compose better.

### From Phase 1c (still open, low priority)

- `telaradio-modulate` CLI smoke binary for ear-validation (deferred
  by spec; defer until felt need)
- Recipe-schema field for configurable anti-click ramp time (Phase 2
  candidate)
