# Current state

**Date**: 2026-04-29
**Phase**: Phase 1d2 (real ACE-Step wired into the Swift app + first-launch
model setup sheet) complete on `main` — Rust + Swift build green, audible
verification of the new real-model + use-existing paths owed by user (the
existing mock-path regression should still produce the 440 Hz sine at 16 Hz).
Phase 1e (background buffer queue) not yet started.
**Build status**: `cargo test --workspace` green (71 passed / 2
ignored). `cargo clippy --all-targets -- -D warnings` clean (pedantic).
`cargo fmt --check` clean. `make ffi` regenerates the cbindgen header
and builds `libtelaradio_ffi.a`. `make swift` links the Swift package
against it; `make app-run` launches the Telaradio executable. First
launch (no `UserDefaults` set) shows a `ModelSetupView` sheet asking the
user to choose a model source.

## Last session summary

Phase 1d2 — real ACE-Step wired into the Swift app. Three sequential
Sonnet sub-agents landed three commits on `main`:

1. **`73ee6d7`** — Rust FFI surface: `tr_cancel_token_*`, `tr_ensure_model_download`,
   `tr_ensure_model_use_existing`, `tr_generate_ace_step`, `tr_string_free`.
   Plus `model-adapter::ace_step_artifacts()` exposing the canonical HF
   manifest (sha256s currently placeholders; real-download bootstrap is a
   tracked follow-up). 7 new FFI tests; 71/2 totals.
2. **`c9c0df6`** — Swift wrappers in `Telaradio.swift`: throwing async
   functions `ensureModelDownload(progress:)`, `ensureModelUseExisting`,
   `generateAceStep`. Progress-callback bridging via `Unmanaged.passRetained`
   + a class-based `ProgressContext`; main-actor dispatch inside the C
   bridge.
3. **`1328cdf`** — SwiftUI sheet (`ModelSetupView`) + UserDefaults-backed
   `ModelSettings` + `PlayerViewModel` branching on `settings.backend`
   (`.mock` keeps the 5-second sine; `.aceStep` calls the real model).
   `PlayerView` shows the sheet via `.sheet(isPresented: !isConfigured)`
   with `.interactiveDismissDisabled()`.

Spec at [`phases/phase-1d2-real-ace-step-spec.md`](phases/phase-1d2-real-ace-step-spec.md);
report at [`phases/phase-1d2-real-ace-step-report.md`](phases/phase-1d2-real-ace-step-report.md).

**Manual verification owed** (user, on macOS): `defaults delete com.telaradio.Telaradio`
to clear settings, then `make app-run`. Sheet should appear; "Use mock
for now" should regress cleanly to the 1d MVL behavior. Real-model
"Use existing folder" path requires real sha256s in
`ace_step.rs::ace_step_artifacts()` — that's the next-step bootstrap.

### Earlier in the session: Phase 1d MVL

Phase 1d MVL — macOS Swift player shell that exercises the full Rust
pipeline end-to-end. Three things landed:

**1. `telaradio-ffi` crate** — new workspace member at `ffi/`. Pure C
ABI shim around `telaradio-core`, `telaradio-dsp`, and
`telaradio-model-adapter`. Functions exposed: `tr_recipe_parse` /
`tr_recipe_free` (round-trip recipes); `tr_wavbuffer_new` /
`tr_wavbuffer_free` plus accessors for samples / len / sample_rate /
channels; `tr_generate_mock` (spawns the Python mock subprocess);
`tr_apply_am` (DSP modulation); `tr_last_error` (thread-local error
string). Errors surface as null + `tr_last_error`.

**2. cbindgen header generation.** `ffi/cbindgen.toml` + `ffi/build.rs`
regenerate `apple/Telaradio/Sources/TelaradioFFI/include/telaradio_ffi.h`
on every `cargo build -p telaradio-ffi`. Forward declarations for
`TrRecipe` and `TrWavBuffer` are injected via `after_includes`.

**3. SwiftUI macOS app at `apple/Telaradio/`.** Two targets in the
SwiftPM package: `TelaradioFFI` (system library wrapping the cbindgen
header via `module.modulemap`) and `Telaradio` (executable).
Idiomatic Swift wrappers (`Recipe`, `WavBuffer` ARC-managed handles;
`Telaradio` static API with throwing functions) hide the unsafe
pointer wrangling. SwiftUI `PlayerView` shows status text +
Play/Pause/Stop. `PlayerViewModel` orchestrates the
generate→modulate→play pipeline using AVAudioEngine.

A workspace-root `Makefile` orchestrates the cross-language build:
`make ffi` → `make swift` → `make app-run`. `MACOSX_DEPLOYMENT_TARGET=13.0`
is exported to align Rust's deployment target with SwiftPM's
`platforms: [.macOS(.v13)]`.

**Manual verification complete (2026-04-29)**: `make app-run` launches
the Telaradio window, Play produces the expected 440 Hz sine pulsing
at 16 Hz, Pause/Stop respond. One small fix landed during verification:
SwiftPM executables on macOS launch without an activation policy, so
the window never gained focus. Promoting via `NSApplication.shared.setActivationPolicy(.regular)`
+ `activate(ignoringOtherApps: true)` in `TelaradioApp.init()` fixed
it. Also untracked ~2515 accidentally-committed `apple/Telaradio/.build/`
artifacts and added `**/.build/` to `.gitignore`.

See [`phases/phase-1d-macos-player-report.md`](phases/phase-1d-macos-player-report.md).

## What exists

- Phase 0 scaffold (CLAUDE.md, ARCHITECTURE.md, ROADMAP.md, README.md,
  PHASE_0_REPORT.md, LICENSE, CLA.md, `.github/`, module READMEs,
  `.docs/ai/` handoff)
- Cargo workspace at project root (members: `core`, `dsp`, `ffi`,
  `model-adapter`)
- `telaradio-core` (`core/`):
  - `recipe::*` — schema v1 types + strict parser
  - `audio::WavBuffer` + `DEFAULT_SAMPLE_RATE_HZ` / `DEFAULT_CHANNELS`
  - `generator::Generator` trait + `GeneratorError`
- `telaradio-dsp` (`dsp/`):
  - `dsp::Envelope` + `From<core::recipe::Envelope>`
  - `dsp::apply_am(...) -> WavBuffer`
- `telaradio-model-adapter` (`model-adapter/`):
  - NDJSON IPC (`protocol::Request` / `Response`)
  - `subprocess::SubprocessGenerator` (mock-sine)
  - `ace_step::AceStepGenerator` (real ACE-Step, lives but not yet
    wired into the Swift app)
  - `hf_download::*` + `model_install::*`
  - `python/telaradio_subprocess.py` + `python/telaradio_ace_step.py`
    + `python/pyproject.toml` (uv project)
- `telaradio-ffi` (`ffi/`) — Phase 1d:
  - `tr_recipe_parse/free`
  - `tr_wavbuffer_new/free` + accessors
  - `tr_generate_mock` + `tr_apply_am`
  - `tr_last_error`
  - cbindgen-generated `telaradio_ffi.h`
- `apple/Telaradio/` Swift package — Phases 1d MVL + 1d2:
  - `TelaradioFFI` system library target
  - `Telaradio` SwiftUI macOS app target
  - `Telaradio.swift` (idiomatic wrapper for all FFI surfaces),
    `PlayerView.swift`, `PlayerViewModel.swift`, `TelaradioApp.swift`,
    `ModelSettings.swift` (UserDefaults-backed observable), and
    `ModelSetupView.swift` (first-launch sheet)
- Workspace `Makefile` orchestrating cross-language builds
- 65 Rust integration tests across 10 test files (1 ignored e2e)
- `recipes/example-foggy-lofi.json`
- GitHub repo `TaylorFinklea/telaradio` (public)

## Blockers

None.

## What does NOT exist yet

- Real sha256 checksums in `ace_step::ace_step_artifacts()` — placeholders
  until the one-time HF download bootstrap. Without this, both the
  Download path and the Use-existing path fail validation. See report
  for the procedure.
- File picker for arbitrary recipes (Phase 1g or sooner if felt)
- Background buffer queue (Phase 1e)
- Settings UI: preset / 3-tier / advanced (Phase 1g) — also where a
  "Change model…" affordance and a download-cancel button belong
- Remaining ~19 starter recipes (Phase 1f)
- iOS app (Phase 2)
- CLI smoke binary `telaradio-modulate` (deferred Phase 1c)

## Pointers

- [`next-steps.md`](next-steps.md) — exact next actions
- [`decisions.md`](decisions.md) — index of decision records
- [`phases/`](phases/) — phase specs and reports
- [`../../ROADMAP.md`](../../ROADMAP.md) — phases 1–4
