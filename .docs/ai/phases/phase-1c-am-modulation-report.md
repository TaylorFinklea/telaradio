# Phase Report: Phase 1c — AM Modulation DSP

**Roadmap item:** Phase 1, item 4 (Rust AM modulation DSP per Woods et al. §Methods)
**Date:** 2026-04-26
**Status:** complete
**Branch:** `phase-1c` (worktree `/Users/tfinklea/git/telaradio-phase-1c`); not pushed — parent merges.

## What shipped

A new `telaradio-dsp` workspace member crate implementing a pure
amplitude-modulation transform per Woods et al. 2024 *Communications
Biology* §Methods.

### Files added

- `dsp/Cargo.toml` — workspace member; depends on `telaradio-core` only.
- `dsp/src/lib.rs` — module wiring + re-exports of `Envelope` and
  `apply_am`.
- `dsp/src/envelope.rs` — `dsp::Envelope` enum (`Square`, `Sine`,
  `Triangle`) with a `From<core::recipe::Envelope>` impl for pipeline
  glue.
- `dsp/src/am.rs` — `apply_am` and the gate-shape helpers.
- `dsp/tests/am_apply.rs` — 10 integration tests covering the public
  API contract.
- `model-adapter/tests/dsp_pipeline.rs` — 1 end-to-end smoke test
  (mock-sine subprocess → apply_am → sample-distribution assertions).

### Files modified

- `Cargo.toml` (workspace) — `dsp` added to `members`,
  `telaradio-dsp = { path = "dsp" }` added to `[workspace.dependencies]`.
- `model-adapter/Cargo.toml` — `telaradio-dsp` added as `dev-dependency`
  for the smoke test.
- `dsp/README.md` — appended an "Implemented (Phase 1c)" section.
- `core/README.md` — appended a one-paragraph cross-reference to the
  new DSP crate.
- `ROADMAP.md` — Phase 1 item 4 marked `[x]`.
- `.docs/ai/current-state.md`, `next-steps.md`, `decisions.md` —
  updated.

## Acceptance criteria — all met

- [x] New `telaradio-dsp` workspace member crate at `dsp/`.
- [x] `dsp::Envelope` enum, decoupled from `core::recipe::Envelope`,
      with `From` conversion.
- [x] `dsp::apply_am(buffer, rate_hz, depth, envelope) -> WavBuffer` —
      pure function, allocates only the output buffer.
- [x] Identical AM on both stereo channels (paper-faithful).
- [x] 1 ms linear crossfade at envelope transitions (Square only —
      Sine and Triangle are already C^∞ / piecewise C^0).
- [x] Sample-rate-aware phase: `phase = (i / sr) * rate_hz`, frame
      indices map to identical gate values across channels.
- [x] TDD-driven coverage: depth=0 identity, depth=1 trough floor,
      rate-locked phase (4 Hz / 1 s = 4 cycles), Sine smoothness,
      Triangle piecewise-linearity, stereo pair invariant, anti-click
      ramp behavior, mono support, metadata preservation.
- [x] Quality gates: `cargo test --workspace` (40 tests green),
      `cargo clippy --all-targets -- -D warnings` clean (pedantic),
      `cargo fmt --check` clean.
- [x] Pipeline smoke test in `model-adapter/tests/dsp_pipeline.rs` —
      generates a 1s mock buffer, applies AM at 16 Hz / 0.5 / Square,
      asserts modulated RMS sits between half and 95% of raw RMS
      (gate=0.5 trough should attenuate the troughs but not silence
      the peaks).

## Test counts

| Crate | Suite | Tests |
|---|---|---|
| telaradio-core | unit + audio + generator + recipe_parse | 0 + 4 + 2 + 14 |
| telaradio-dsp | am_apply (integration) | 10 |
| telaradio-model-adapter | dsp_pipeline + end_to_end + protocol_serde | 1 + 4 + 5 |
| **Total** | | **40** (29 prior + 11 new) |

## Mid-build judgment calls (logged here)

1. **Anti-click ramp is centered on each Square transition**, not
   leading/trailing it. The ramp covers `±0.5 ms` around `fraction=0.0`
   and `fraction=0.5`. Centering keeps the average gate value
   identical to a hard-square gate over a full cycle, preserving
   loudness statistics across Phase 1c → 1d eyeball comparisons.
2. **`From<core::recipe::Envelope> for dsp::Envelope` is implemented
   now**, even though the spec deferred it to "whoever wires the
   recipe into the player". Cost is six lines, value is one fewer
   tiny commit when Phase 1d/1e wires it up. Recipe schema is
   unchanged.
3. **Cast lints**: `apply_am` carries a localized
   `#[allow(clippy::cast_precision_loss, cast_possible_truncation,
   cast_sign_loss)]` with a documented rationale comment. Frame index
   → f64 is exact for any audio length we care about; gate f64 → f32
   is at most a sub-LSB rounding on a value in `[0, 1]`. Tests get a
   module-level `#![allow(...)]` for the same casts (idiomatic in
   audio-math test code).
4. **`mono_input_modulates_correctly` test was extended to 1 second**
   from the original 0.1 s (which only covered the first 40% of a
   cycle at 4 Hz and never reached a trough). The fix landed during
   the GREEN cycle, not as a hidden test-only relaxation.
5. **`rate_locked_phase` test counts falling transitions**, not rising
   ones. Rising-transition counting requires a "previously low" state
   at frame 0, which the synthesized buffer doesn't have (it starts
   in the peak region). Falling transitions give the cleanest 1:1
   mapping to gate cycles.

## Caveats & deviations

- **No CLI smoke binary** (`telaradio-modulate`) — explicitly deferred
  by the spec; the integration test exercises the same code path
  programmatically.
- **No custom-file modulation** — the spec excluded this; AM is
  applied only to generator output.
- **Configurable ramp time**: hard-coded constant
  `RAMP_HALF_WIDTH_S = 0.000_5` (1 ms total). Recipe-schema
  configurability is a Phase 2 candidate.
- **Phase 1b2 coordination**: this branch and the `phase-1b2` branch
  both touch the workspace `Cargo.toml` `members` list and the same
  handoff docs. The parent session will resolve merges.

## Verification commands

```sh
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

All three pass on the `phase-1c` branch.
