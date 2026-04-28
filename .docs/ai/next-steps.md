# Next steps

Phase 1 checklist lives in [`../../ROADMAP.md`](../../ROADMAP.md). Phase
1a (recipe core), Phase 1b (Generator trait + mock subprocess), and
Phase 1c (AM modulation DSP) are complete. Phase 1b2 (real ACE-Step) is
building in parallel.

## Recommended: Phase 1d — macOS Swift player shell

With the DSP done and the mock generator producing real WAV buffers,
the macOS player can be wired up end-to-end against the mock and
reuse all the Rust pieces unchanged when ACE-Step lands.

1. Bootstrap `apple/` Swift package with a minimal AppKit/SwiftUI
   shell: load recipe, play, pause, skip.
2. FFI / IPC layer to the Rust workspace. Two candidates:
   (a) build the Rust workspace as a static library and call it from
   Swift via a C-ABI shim, or (b) spawn the Rust binary as a
   subprocess and stream WAV over stdout. Decision deferred to Phase
   1d kickoff.
3. Wire `Recipe::parse` → `SubprocessGenerator::generate` →
   `dsp::apply_am` end-to-end. The Swift app is a thin player on top.

## Alternative: Phase 1e — background buffer queue

Once the player exists, keep 2–3 tracks generated and modulated ahead
in a background queue so playback feels instant. Probably better as
Phase 1e (after 1d) than blended into 1d.

## Phase 1b2 is in flight

`phase-1b2` branch is being built in a parallel worktree. Both
branches modify the workspace `Cargo.toml` `members` list and the
same handoff docs; the parent session merges. No action needed from
this branch.

## After 1d / 1b2

- Phase 1e — Background buffer queue
- Phase 1f — Hand-seed ~20 starter recipes
- Phase 1g — Settings UI (preset / 3-tier intensity / advanced)
- Phase 1 wrap: `PHASE_1_REPORT.md` covering ear-eval against Brain.fm

## Optional follow-ups deferred from Phase 1c

- `telaradio-modulate` CLI smoke binary for ear-validation (deferred
  by spec; defer until felt need).
- Recipe-schema field for configurable anti-click ramp time (Phase 2
  candidate; current 1 ms hard-coded constant is fine for v1).
