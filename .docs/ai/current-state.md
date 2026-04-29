# Current state

**Date**: 2026-04-29
**Phase**: Phase 1d MVL (macOS Swift player shell, mock-only) complete on
`main` and **verified end-to-end through Mac speakers** — clicking Play
produces the expected 440 Hz sine modulated at 16 Hz. Phase 1d2 (real
ACE-Step wiring + first-launch model UX) and 1e (background buffer
queue) not yet started.
**Build status**: `cargo test --workspace` green (65 passed / 1
ignored). `cargo clippy --all-targets -- -D warnings` clean (pedantic).
`cargo fmt --check` clean. `make ffi` regenerates the cbindgen header
and builds `libtelaradio_ffi.a`. `make swift` links the Swift package
against it; `make app-run` launches the Telaradio executable.

## Last session summary

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
- `apple/Telaradio/` Swift package — Phase 1d MVL:
  - `TelaradioFFI` system library target
  - `Telaradio` SwiftUI macOS app target
  - `Telaradio.swift` (idiomatic wrapper), `Recipe.swift`,
    `WavBuffer.swift`, `PlayerView.swift`, `PlayerViewModel.swift`,
    `TelaradioApp.swift`
- Workspace `Makefile` orchestrating cross-language builds
- 65 Rust integration tests across 10 test files (1 ignored e2e)
- `recipes/example-foggy-lofi.json`
- GitHub repo `TaylorFinklea/telaradio` (public)

## Blockers

None.

## What does NOT exist yet

- Real ACE-Step wired into the Swift app (Phase 1d2)
- First-launch model install SwiftUI flow (Phase 1d2)
- File picker for arbitrary recipes (Phase 1g or sooner if felt)
- Background buffer queue (Phase 1e)
- Settings UI: preset / 3-tier / advanced (Phase 1g)
- Remaining ~19 starter recipes (Phase 1f)
- iOS app (Phase 2)
- CLI smoke binary `telaradio-modulate` (deferred Phase 1c)

## Pointers

- [`next-steps.md`](next-steps.md) — exact next actions
- [`decisions.md`](decisions.md) — index of decision records
- [`phases/`](phases/) — phase specs and reports
- [`../../ROADMAP.md`](../../ROADMAP.md) — phases 1–4
