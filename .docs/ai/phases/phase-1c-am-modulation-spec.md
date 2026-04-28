# Phase Spec: Phase 1c — AM Modulation DSP

**Roadmap item:** Phase 1, item 4 (Rust AM modulation DSP per Woods et al. §Methods)
**Date:** 2026-04-28
**Status:** ready to build

## Product

**Goal:** A pure-function amplitude-modulation transform that takes a
`WavBuffer` and returns a modulated `WavBuffer`, implementing the
mechanism described in Woods et al. 2024 *Communications Biology*
([doi:10.1038/s42003-024-07026-3](https://www.nature.com/articles/s42003-024-07026-3)).
After this slice, any `WavBuffer` (mock-generated or real ACE-Step
output once Phase 1b2 lands) can be modulated end-to-end via a single
function call.

### Acceptance criteria

- [ ] New `telaradio-dsp` workspace member crate at `dsp/`
- [ ] `dsp::Envelope` enum (`Square`, `Sine`, `Triangle`) — owned by the
      DSP crate, *not* re-using `core::recipe::Envelope`. The two enums
      may be converted with a small `From` impl.
- [ ] `dsp::apply_am(buffer: &WavBuffer, rate_hz: f64, depth: f64,
      envelope: Envelope) -> WavBuffer` — pure function, no side effects,
      allocation only for the output buffer
- [ ] Identical AM on both stereo channels (in-phase, paper-faithful per
      Woods et al.)
- [ ] 1 ms linear crossfade at envelope transitions to suppress audible
      clicks at high depth values. Internal constant for v1; not
      configurable on the recipe.
- [ ] Sample-rate-aware: phase advances by `1/sample_rate` per sample,
      gate transitions land on the same sample regardless of channels.
- [ ] TDD-driven coverage:
  - depth=0 → output samples == input samples (within f32 epsilon)
  - depth=1 + Square envelope → ~50% of samples are at the trough floor
    (allowing for the 1 ms crossfade region)
  - rate-locked phase: at rate=4 Hz over 1 second, exactly 4 gate
    cycles
  - Sine envelope: smooth (no two adjacent samples differ by more than
    a `max_step` derived from rate × 2π / sample_rate)
  - Triangle envelope: piecewise-linear (second derivative ~zero except
    at apexes)
  - Stereo sample-pair invariant: for every i, `out[2i] == out[2i+1]`
    when input has identical L and R
- [ ] Quality gates from project root:
  - `cargo test` green workspace-wide
  - `cargo clippy --all-targets -- -D warnings` clean (pedantic)
  - `cargo fmt --check` clean
- [ ] Pipeline smoke test (in `model-adapter/tests/` since it requires
      both crates): generate a 1s mock WAV → apply_am at 16 Hz / 0.5 /
      Square → assert the modulation actually changed the sample
      distribution (e.g., gate region samples are clearly attenuated)
- [ ] Phase report at `.docs/ai/phases/phase-1c-am-modulation-report.md`

### Assumptions

- Internal math is `f32` (matches `WavBuffer.samples`). f64 gives no
  audible benefit at audio sample rates and adds conversion noise.
- `WavBuffer.samples` is interleaved when `channels > 1` (L0, R0, L1,
  R1, ...). The DSP processes frames (one sample per channel) in lockstep.
- 1 ms crossfade is hard-coded. Configurable ramp-time on the recipe is
  a Phase 2 candidate, not Phase 1c scope.
- Recipe schema is unchanged. The DSP reads its parameters from
  function arguments; pipeline code is responsible for translating
  `Recipe.modulation` into those args.

### Out of scope (deferred)

- CLI smoke binary (`telaradio-modulate`) — useful for ear-validation
  but not strictly needed; defer until a felt need
- Custom-file modulation (Telaradio modulating arbitrary audio you
  bring) — explicitly *not* a goal per 2026-04-28 product decision
- Live-tweaking depth/rate during playback — Phase 1d (player) territory
- `From<core::recipe::Envelope> for dsp::Envelope` impl — write it if
  the smoke test needs it; otherwise defer to whoever wires the recipe
  into the player
- Configurable ramp-time field on `recipe.modulation`
- Anti-DC-offset filtering, look-ahead lookups, or anything else not
  in Woods et al. §Methods

### Open questions (resolved 2026-04-28)

1. **Envelope enum**: separate `dsp::Envelope` (decoupled from
   `core::recipe::Envelope`). DSP can grow new envelope shapes without
   forcing a recipe schema bump.
2. **Math precision**: f32 internally.
3. **Stereo handling**: identical AM on both channels (paper-faithful).
4. **Anti-click ramps**: 1 ms linear crossfade, hard-coded.
5. **CLI smoke binary**: deferred.

---

## Technical approach

### Scope

Create:
- `dsp/Cargo.toml` — workspace member; deps `telaradio-core`, that's it
  (DSP is pure Rust)
- `dsp/src/lib.rs` — module wiring + re-exports
- `dsp/src/envelope.rs` — `Envelope` enum + `gate_at_phase()` helper
- `dsp/src/am.rs` — `apply_am` function and supporting math
- `dsp/tests/am_apply.rs` — integration tests for the public API
- `model-adapter/tests/dsp_pipeline.rs` — end-to-end smoke (depends on
  both `telaradio-dsp` and `model-adapter`)
- `.docs/ai/phases/phase-1c-am-modulation-report.md`

Modify:
- `Cargo.toml` (workspace): add `dsp` to `members`; add `telaradio-dsp = { path = "dsp" }` to `[workspace.dependencies]`
- `model-adapter/Cargo.toml`: add `telaradio-dsp` as dev-dep for the smoke test
- `dsp/README.md`: append "Implemented (Phase 1c)" section
- `ROADMAP.md`: mark item 4 `[~]` while in progress, then `[x]` on report
- `.docs/ai/current-state.md`, `next-steps.md`, `decisions.md`

### Math sketch (Woods et al. §Methods + 1 ms crossfade)

For each frame index `i` (frame = one sample per channel), at sample
rate `sr`:

```text
phase   = (i / sr) * rate_hz             // cycles since start
fraction = phase - floor(phase)          // in [0.0, 1.0)
gate    = match envelope:
    Square:   if fraction < 0.5 then 1.0 else trough(depth)
    Sine:     1.0 - depth * (0.5 - 0.5 * cos(2π * fraction))
    Triangle: 1.0 - depth * tri_envelope(fraction)

// 1 ms crossfade applied around the Square gate flips at fraction=0.0
// and fraction=0.5. Sine/Triangle are already smooth.
gate    = apply_ramp_if_near_transition(gate, fraction, sr, rate_hz)

for c in 0..channels:
    output[i*channels + c] = input[i*channels + c] * gate
```

Where:
- `trough(depth) = 1.0 - depth` (depth=0 → no modulation; depth=1 → silence between gates)
- The ramp is applied only when within `1ms × rate_hz` cycles of a Square transition

### Steps (TDD discipline mandatory)

1. Add `dsp` to workspace members; create empty `dsp/src/lib.rs`.
2. Write failing test for `Envelope` enum (just constructs each variant).
3. Implement `Envelope` enum.
4. Write failing test for `apply_am` at depth=0 (output == input).
5. Implement `apply_am` minimally; verify GREEN.
6. Add depth=1 + Square test; iterate impl until GREEN.
7. Add rate-locked phase test (4 Hz over 1s = 4 cycles).
8. Add Sine + Triangle envelope tests.
9. Add stereo-pair-invariant test.
10. Add 1 ms crossfade test (samples within ramp region transition smoothly).
11. Run `cargo clippy --all-targets -- -D warnings` and `cargo fmt`. Fix.
12. Add the end-to-end smoke test in `model-adapter/tests/dsp_pipeline.rs`.
13. Write the phase report.
14. Update handoff docs.
15. Commit but **DO NOT PUSH** — parent session merges.

### Verification

- `cargo test --workspace` — all green
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt --check` — clean
- Manual: read the apply_am source, confirm it reads as a plain
  translation of the Woods et al. pseudocode + a small ramp helper

### Commit conventions

- Commit message convention: `feat(dsp): <short>` for new code,
  `chore(dsp): <short>` for tooling
- Trailing footer: `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`
- One commit per phase is fine; finer granularity is OK if natural

### Coordination with Phase 1b2

Phase 1b2 also modifies the workspace `Cargo.toml` and the same handoff
docs. Phase 1c is the smaller change and merges first; Phase 1b2 will
need to rebase or merge cleanly on top.
