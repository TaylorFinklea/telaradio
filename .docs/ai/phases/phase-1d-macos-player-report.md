# Phase Report: Phase 1d — macOS Swift Player Shell (MVL)

**Date:** 2026-04-28
**Outcome:** pass (Rust + Swift build verification); audible-output
check pending user verification
**Spec:** [`phase-1d-macos-player-spec.md`](phase-1d-macos-player-spec.md)

## Changes

- `Cargo.toml` — added `ffi` workspace member; added `cbindgen`,
  `telaradio-ffi`, `telaradio-model-adapter` to `[workspace.dependencies]`
- `ffi/Cargo.toml` (new) — `telaradio-ffi` crate, `crate-type =
  ["staticlib", "rlib"]`, depends on core/dsp/model-adapter
- `ffi/cbindgen.toml` (new) — header config; `Recipe`/`WavBuffer`
  renamed to `TrRecipe`/`TrWavBuffer`; forward decls in
  `after_includes`
- `ffi/build.rs` (new) — invokes cbindgen on every build; writes
  `apple/Telaradio/Sources/TelaradioFFI/include/telaradio_ffi.h`
- `ffi/src/lib.rs` (new) — C ABI surface (~250 lines): `tr_recipe_*`,
  `tr_wavbuffer_*`, `tr_generate_mock`, `tr_apply_am`, `tr_last_error`
- `ffi/tests/ffi_round_trips.rs` (new) — 9 integration tests covering
  ownership, error paths, end-to-end pipeline
- `apple/Telaradio/Package.swift` (new) — SwiftPM manifest, two targets
- `apple/Telaradio/Sources/TelaradioFFI/module.modulemap` (new) —
  bridges the cbindgen header into Swift
- `apple/Telaradio/Sources/TelaradioFFI/include/telaradio_ffi.h` (new,
  generated) — committed for visibility but auto-regenerated
- `apple/Telaradio/Sources/Telaradio/Telaradio.swift` (new) —
  idiomatic Swift wrappers (`Recipe`, `WavBuffer` classes; `Telaradio`
  static API; `TelaradioError`)
- `apple/Telaradio/Sources/Telaradio/PlayerViewModel.swift` (new) —
  `@MainActor ObservableObject` driving the player; AVAudioEngine
- `apple/Telaradio/Sources/Telaradio/PlayerView.swift` (new) — SwiftUI
  view, Play/Pause/Stop + status text
- `apple/Telaradio/Sources/Telaradio/TelaradioApp.swift` (new) —
  `@main` `App` entry
- `Makefile` (new) — `make ffi`, `make swift`, `make app-run`, plus
  test/lint/fmt; pins `MACOSX_DEPLOYMENT_TARGET=13.0`
- `apple/README.md` — appended "Implemented (Phase 1d MVL)" section
- `ROADMAP.md` — Phase 1 item 6 marked `[~]` (MVL shipped; full
  version awaits 1d2)
- `.docs/ai/{current-state,next-steps,decisions}.md` — updated for
  Phase 1d MVL

## Decisions made

See `decisions.md` 2026-04-28 entry "Phase 1d MVL: macOS player shell"
for the full list. Summary: static lib + cbindgen + Swift module.modulemap
(picked over CLI helper); MVL is mock-only by design (real ACE-Step
wiring is Phase 1d2); hardcoded recipe path resolved by walking up to
find Cargo.toml; AVAudioEngine plays a single scheduled buffer; Swift
ARC owns the FFI pointers via final class + deinit.

Mid-build:
- cbindgen 0.27 → 0.29 (Rust 2024 syntax support)
- `#[allow(clippy::cast_precision_loss)]` at FFI test module level
- `OpaquePointer` (not `UnsafePointer<TrRecipe>`) in Swift wrappers

## Verification results

```
$ cargo test --workspace
running 4 tests   (audio)             ok. 4 passed
running 2 tests   (generator)         ok. 2 passed
running 14 tests  (recipe_parse)      ok. 14 passed
running 10 tests  (am_apply)          ok. 10 passed
running 9 tests   (ffi_round_trips)   ok. 9 passed
running 3 tests   (ace_step_smoke)    ok. 3 passed
running 1 test    (dsp_pipeline)      ok. 1 passed
running 4 tests   (end_to_end)        ok. 4 passed
running 5 tests   (hf_download)       ok. 5 passed
running 8 tests   (model_install)     ok. 8 passed
running 5 tests   (protocol_serde)    ok. 5 passed
running 1 test    (ace_step_e2e)      0 passed; 1 ignored

Total: 65 passed; 0 failed; 1 ignored

$ cargo clippy --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo]

$ cargo fmt --check
(clean)

$ make ffi
cargo build -p telaradio-ffi → wrote header
Finished

$ make swift
cd apple/Telaradio && swift build → Build complete!
```

### Manual verification checklist

- [x] `cargo test --workspace` — 65/65 passing (1 ignored)
- [x] `cargo clippy --all-targets -- -D warnings` — clean (pedantic)
- [x] `cargo fmt --check` — clean
- [x] `make ffi` regenerates the C header successfully
- [x] `make swift` builds the Swift package without errors
- [ ] **(USER)** `make app-run` launches the Telaradio app
- [ ] **(USER)** Clicking Play produces an audible 440 Hz sine
      pulsing at 16 Hz through the Mac speakers
- [ ] **(USER)** Pause and Stop respond as expected

## Follow-up items

- [ ] Phase 1d2: wire real ACE-Step into the Swift app (FFI shims
      for `ensure_model` + `generate_ace_step`, first-launch SwiftUI
      sheet, model-path persistence in `UserDefaults`)
- [ ] If audio output sounds wrong (silence / distortion / wrong
      modulation), debug `PlayerViewModel.makePCMBuffer`'s
      interleaved → planar demux first
- [ ] Eventually: bundle the Rust static lib + a proper `.app`
      bundle for distribution (Phase 4-ish)
- [ ] Consider whether `apple/Telaradio/Sources/TelaradioFFI/include/
      telaradio_ffi.h` should be committed (currently yes, since
      `swift build` needs it before `cargo build` runs the build script
      on a fresh checkout). Alternative: pre-commit hook runs
      `make ffi` first.

## Context for next phase

- The FFI is purposely small and additive. Phase 1d2 just adds more
  `tr_*` functions (and any required state types) without touching
  existing ones. Avoid `#[repr(C)]` enums and keep "kind" parameters
  as `u32` discriminants for forward-compatibility.
- The `Telaradio` Swift static enum is the right place for new
  throwing wrappers. `lastFFIError` already covers the
  null-with-error convention.
- AVAudioEngine state is owned by `PlayerViewModel`. Adding new
  audio sources (e.g. an ACE-Step-generated track instead of the
  mock) is a one-line swap in `playExample` once the corresponding
  `Telaradio.generateAceStep(...)` exists.
- Build order matters: `cargo build -p telaradio-ffi` MUST run before
  `swift build` on a fresh clone (the header is generated by the
  Rust build script). The Makefile encodes this dependency.
- One landmine: cbindgen output renames `Recipe` → `TrRecipe` and
  `WavBuffer` → `TrWavBuffer` for the C namespace. Inside Rust we
  still see `Recipe` / `WavBuffer`. Don't rename them in `lib.rs` or
  the cross-references between cbindgen and Swift will desync.
