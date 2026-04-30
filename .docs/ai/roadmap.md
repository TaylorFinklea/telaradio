# Roadmap (pointer)

The canonical roadmap is at [`../../ROADMAP.md`](../../ROADMAP.md). Keep durable phase plans there; this file holds active items in Now/Next/Later.

If you find yourself updating one but not the other, fix it now.

## Now / Next / Later

Active items. Trim as completed.

### Now
- **Bootstrap real ACE-Step sha256s** — Phase 1d2 shipped with placeholder sha256s in `model-adapter/src/ace_step.rs::ace_step_artifacts()`. Until real values land, both Download and Use-existing paths in the SwiftUI sheet fail validation. Fix: download ACE-Step v1 3.5B once (`huggingface-cli download ACE-Step/ACE-Step-v1-3.5B --local-dir /tmp/ace`), `sha256sum` each artifact listed in `ace_step_artifacts()`, paste hex strings, commit. **Tier hint**: Sonnet — bounded mechanical work.
- **Confirm Phase 1d2 audibly** — `defaults delete com.telaradio.Telaradio` (clears UserDefaults), then `make app-run`. Verify (1) first-launch sheet appears with three buttons; (2) "Use mock for now" + Play produces the 1d MVL sine; (3) re-clear UserDefaults, "Use existing folder" + Play generates real audio modulated at 16 Hz.

### Next
- **Phase 1e — background buffer queue.** Real ACE-Step has ~10 s cold-start latency on consumer hardware. Keep 2–3 tracks pre-generated and modulated ahead of the user during idle. Hooks in at the `Telaradio.generateAceStep(...)` seam — no FFI changes needed.
- **Phase 1f** — hand-seed ~20 starter recipes (lofi, ambient, electronic, nature-hybrid prompts).
- **Phase 1g** — Settings UI (preset selector / 3-tier intensity slider / advanced rate-depth-bypass panel).
- **Phase 1 wrap** — `PHASE_1_REPORT.md` covering ear-eval against Brain.fm.

### Later — open follow-ups

**From Phase 1d2**
- macOS deployment target mismatch — Rust static lib targets host (26.0) while SwiftPM links 13.0. Fix: pin `-mmacosx-version-min=13.0` in `.cargo/config.toml` rustflags.
- "Change model…" affordance — `defaults delete com.telaradio.Telaradio` is the workaround until Phase 1g.
- Download cancel UI — Phase 1g surfaces a Cancel button in the Download progress view.
- `settings.modelDir!` force-unwrap — safe today but fragile once Phase 1g lets users clear the path. Replace with a `guard let` + clear error.

**From Phase 1d**
- File picker for arbitrary recipes — defer until felt; current hardcoded path is fine for one-developer usage.
- Multi-track playlist / skip-ahead — Phase 1e (buffer queue) is the natural place.
- Now Playing / lock-screen integration — worth doing eventually, not v1.
- Bundling the Rust static lib into a proper `.app` bundle — Phase 4-ish (App Store distribution).

**From Phase 1b2**
- ACE-Step PyPI sdist is broken upstream; we git-pin commit `1bee4c9f`. Periodically check if upstream re-publishes.
- `ensure_model` and `AceStepGenerator::spawn` are both reachable from Swift; wrapper calls them in sequence. Original "wire ensure_model into spawn" proposal was rejected — keeping them separate makes the FFI surface compose better.

**From Phase 1c (low priority)**
- `telaradio-modulate` CLI smoke binary for ear-validation (deferred by spec).
- Recipe-schema field for configurable anti-click ramp time (Phase 2 candidate).
