# Phase Spec: Phase 1d — macOS Swift Player Shell (MVL)

**Roadmap item:** Phase 1, item 6 (Native macOS Swift app: minimal player UI)
**Date:** 2026-04-28
**Status:** ready to build

## Product

**Goal:** A native macOS app that plays a modulated audio buffer through
the speakers, end-to-end through the existing Rust pipeline (Recipe →
mock generator → AM modulation → AVAudioEngine playback). After this
slice, the user can launch the app, click "Play", and hear a 440 Hz
sine wave amplitude-modulated at 16 Hz coming out their Mac speakers.

This is the **minimum viable loop** — it proves the platform boundary
works. Adding a file picker, first-launch model install UX, and
real ACE-Step integration are later phases.

### Acceptance criteria

- [ ] New `ffi/` workspace member crate (`telaradio-ffi`) with
      `crate-type = ["staticlib"]` exposing a C-ABI surface
- [ ] cbindgen-generated `telaradio_ffi.h` header
- [ ] FFI functions (minimum):
  - `tr_recipe_parse(json: *const c_char) -> *mut Recipe` — returns
    opaque pointer or null on error
  - `tr_recipe_free(recipe: *mut Recipe)` — drops the recipe
  - `tr_generate_mock(prompt, seed, duration_seconds, ...) ->
    *mut WavBuffer` — runs the mock generator (spawns Python
    subprocess), blocks until complete
  - `tr_apply_am(buffer, rate_hz, depth, envelope) -> *mut WavBuffer`
    — applies AM, returns new owned buffer
  - `tr_wavbuffer_free(buffer: *mut WavBuffer)` — drops buffer
  - `tr_wavbuffer_samples(buffer) -> *const f32` + `tr_wavbuffer_len`
    + `tr_wavbuffer_sample_rate` + `tr_wavbuffer_channels` — accessors
- [ ] FFI returns errors via thread-local last-error string
  - `tr_last_error() -> *const c_char` (UTF-8, null on no error)
- [ ] Rust unit tests for the FFI surface (round-trip pointer ownership,
      error reporting, edge cases like null input) — TDD
- [ ] New `apple/Telaradio` Swift package with a macOS executable target
- [ ] Swift module wraps the FFI cleanly: `Recipe`, `WavBuffer` Swift
      classes that own the C pointers and free on `deinit`
- [ ] Swift app UI (SwiftUI):
  - Single window, "Telaradio" title
  - Status text ("Ready", "Generating...", "Playing", "Paused")
  - Play / Pause / Stop buttons
  - When user clicks Play: app loads `recipes/example-foggy-lofi.json`
    (path resolved relative to the app bundle; for now hardcoded
    relative to repo root in dev), generates via mock, applies AM at
    the recipe's rate/depth/envelope, then plays through AVAudioEngine
- [ ] Build integration: the Swift package builds the Rust static lib
      via a build script (`build.rs` invocation in a build phase, or
      a Make target invoked before `swift build`)
- [ ] `swift build` produces a runnable macOS executable
- [ ] Manual verification: launch app, click Play, confirm a 440 Hz
      sine modulated at 16 Hz is audible

### Assumptions

- **Mock generator only.** The Swift app calls `SubprocessGenerator`
  (mock 440 Hz sine), not `AceStepGenerator`. Real-model integration
  is Phase 1d2 (or 1e — TBD when this phase ends).
- **Hardcoded example recipe.** Path is `recipes/example-foggy-lofi.json`
  resolved by walking up from the executable's directory to find the
  workspace root in dev. No file picker.
- **44.1 kHz stereo PCM in `[-1.0, 1.0]`** is the format we hand to
  AVAudioEngine. AVFoundation may resample internally to its preferred
  rate; we accept the conversion.
- **Synchronous generate in this phase.** UI may freeze for ~10s while
  the mock runs (it's actually fast; the real ACE-Step would freeze
  longer). Async / background queue is Phase 1e.
- **No first-launch model install UX in this phase.** Mock doesn't need
  a model. The first-launch flow is Phase 1d2.
- **macOS 13+ target** (no specific reason to push lower; SwiftUI is
  modern enough).
- **SwiftPM, not Xcode project.** A `Package.swift` is the build
  manifest. An Xcode project can be generated later via `swift package
  generate-xcodeproj` if we want IDE integration.

### Out of scope (deferred)

- File picker for arbitrary recipes (Phase 1e or 1g, depending on UX shape)
- Real ACE-Step model wiring + first-launch install (Phase 1d2)
- Multi-track playlist / queue / skip-ahead (Phase 1e — background buffer)
- Settings UI (preset selector, intensity slider, advanced panel) — Phase 1g
- iOS app (Phase 2)
- Lock-screen / Now Playing integration (worth doing eventually; not v1)
- Visualizers, animations, anything decorative
- Apple Watch HR adaptation (Phase 3)
- App Store distribution / code signing (Phase 4-ish)

### Open questions (resolved 2026-04-28 — judgment-called)

1. **FFI surface**: Rust `staticlib` + cbindgen-generated C header +
   Swift bridging via `module.modulemap`. Decided rather than
   subprocess-helper because (a) a real macOS app spawning a CLI
   helper feels unidiomatic, (b) static lib avoids per-generate
   process spawn cost, (c) future App Store / iOS distribution
   essentially require this approach anyway.
2. **FFI shim location**: new `ffi/` workspace member crate. Keeps
   `core/`, `dsp/`, `model-adapter/` runtime-free and pure-Rust;
   the C-ABI surface is contained.
3. **Sample-rate handling**: hand 44.1 kHz buffers to
   `AVAudioEngine`'s scheduled buffer API; let AVFoundation resample
   to the output device's rate if needed. One conversion at the very
   end of the chain is acceptable; we don't pre-convert.
4. **UI framework**: SwiftUI, not AppKit. Faster to write; modern
   API; the macOS app is small enough that SwiftUI's limitations
   don't bite.
5. **Build glue**: `Package.swift` declares a system library target
   for the Rust static lib; a `Makefile` at the workspace root
   orchestrates `cargo build -p telaradio-ffi --release` →
   `cbindgen` → `swift build`. CI will run all three.

---

## Technical approach

### Scope

Create:
- `ffi/Cargo.toml` (workspace member, `crate-type = ["staticlib",
  "rlib"]`); deps: `telaradio-core`, `telaradio-dsp`,
  `telaradio-model-adapter`
- `ffi/src/lib.rs` — `extern "C"` functions per the API list above;
  thread-local `LAST_ERROR: RefCell<Option<CString>>`; safe wrappers
  around `Box::into_raw` / `Box::from_raw`
- `ffi/cbindgen.toml` — cbindgen config (output header, namespace
  prefix, exclude lists)
- `ffi/build.rs` — invokes cbindgen during build to produce
  `target/include/telaradio_ffi.h` (or a deterministic path)
- `ffi/tests/ffi_round_trips.rs` — Rust-side integration tests
  exercising the C ABI through `unsafe` calls (validates ownership,
  null-handling, error paths)
- `apple/Telaradio/Package.swift` — SwiftPM manifest with one
  executable target `Telaradio` plus a system library target
  `TelaradioFFI` linking the static lib
- `apple/Telaradio/Sources/TelaradioFFI/module.modulemap` — bridges
  the cbindgen-generated header into Swift
- `apple/Telaradio/Sources/Telaradio/TelaradioApp.swift` — `@main`
  app entry point
- `apple/Telaradio/Sources/Telaradio/PlayerViewModel.swift` —
  `ObservableObject` driving the UI; owns the AVAudioEngine
- `apple/Telaradio/Sources/Telaradio/PlayerView.swift` — SwiftUI
  view with the status + transport buttons
- `apple/Telaradio/Sources/Telaradio/Recipe.swift` —
  thin Swift wrapper around the FFI `Recipe` pointer; ARC-safe
- `apple/Telaradio/Sources/Telaradio/WavBuffer.swift` — same for
  `WavBuffer` pointer; exposes a typed-buffer view
- `apple/Telaradio/Sources/Telaradio/Telaradio.swift` —
  `enum Telaradio` static API: `parseRecipe(_:)`, `generateMock(_:)`,
  `applyAM(_:to:)`, etc. Hides the unsafe pointer wrangling
- `Makefile` at workspace root — `make ffi`, `make swift`, `make app`,
  `make app-run` targets
- `.docs/ai/phases/phase-1d-macos-player-report.md`

Modify:
- `Cargo.toml`: add `ffi` to `members`; add `telaradio-ffi = { path = "ffi" }` to `[workspace.dependencies]`; add `cbindgen` as a build-time dep (under each consumer's `[build-dependencies]`)
- `apple/README.md`: append "Implemented (Phase 1d MVL)" section
- `ROADMAP.md`: mark Phase 1 item 6 `[x]` (or `[~]` if anything's
  meaningfully deferred)
- `.docs/ai/{current-state,next-steps,decisions}.md`

### FFI surface (concrete)

```c
// telaradio_ffi.h (cbindgen output, abridged)

typedef struct TrRecipe TrRecipe;
typedef struct TrWavBuffer TrWavBuffer;

// Returns null and sets last_error on failure.
TrRecipe *tr_recipe_parse(const char *json);
void tr_recipe_free(TrRecipe *recipe);

// Spawns the mock subprocess, generates, returns owned buffer.
// Path argument is the absolute path to model-adapter/python/telaradio_subprocess.py.
TrWavBuffer *tr_generate_mock(
    const char *script_path,
    const char *prompt,
    uint64_t seed,
    uint32_t duration_seconds);

// Applies amplitude modulation; returns NEW owned buffer.
TrWavBuffer *tr_apply_am(
    const TrWavBuffer *input,
    double rate_hz,
    double depth,
    uint32_t envelope_kind);  // 0 = Square, 1 = Sine, 2 = Triangle

void tr_wavbuffer_free(TrWavBuffer *buffer);
const float *tr_wavbuffer_samples(const TrWavBuffer *buffer);
size_t tr_wavbuffer_len(const TrWavBuffer *buffer);
uint32_t tr_wavbuffer_sample_rate(const TrWavBuffer *buffer);
uint8_t tr_wavbuffer_channels(const TrWavBuffer *buffer);

// Last error message; null if no error. UTF-8, NUL-terminated.
// Owned by the FFI; do not free. Cleared on successful call.
const char *tr_last_error(void);
```

### Build flow

```
make ffi      → cargo build -p telaradio-ffi --release
                cbindgen ffi/ -o apple/Telaradio/Sources/TelaradioFFI/include/telaradio_ffi.h

make swift    → cd apple/Telaradio && swift build

make app-run  → make ffi && make swift && \
                .build/.../Telaradio
```

The Swift package's `Package.swift` references the static lib via
`linkerSettings: [.unsafeFlags(["-L../../../target/release", "-ltelaradio_ffi"])]`
(or similar — adjusted at build time).

### Steps (TDD discipline mandatory for the Rust FFI)

1. Add `ffi` to workspace members; create empty `ffi/src/lib.rs`.
2. (TDD) Write failing tests in `ffi/tests/ffi_round_trips.rs`:
   - parse a valid recipe JSON via `tr_recipe_parse`, get non-null
     pointer, free it
   - parse invalid JSON, get null + non-null `tr_last_error`
   - call `tr_apply_am` with depth=0 on a small buffer, verify the
     returned buffer's samples match input (within epsilon)
   - free a buffer twice → must not double-free (impl: pass ownership;
     caller responsibility documented)
3. Implement `extern "C"` functions until tests pass.
4. Add cbindgen config + build.rs.
5. Create `apple/Telaradio/Package.swift`.
6. Create the `TelaradioFFI` system library target with module.modulemap.
7. Create Swift wrappers (`Recipe.swift`, `WavBuffer.swift`,
   `Telaradio.swift`) that hide unsafe pointer wrangling.
8. Create `PlayerViewModel.swift` with AVAudioEngine wiring.
9. Create `PlayerView.swift` with the SwiftUI UI.
10. Create `TelaradioApp.swift` `@main` entry.
11. Write the `Makefile` orchestration.
12. Run quality gates (cargo + swift build).
13. Manual verify: launch app, click play, listen.
14. Update handoff docs and write the phase report.
15. Commit + push.

### Verification

- `cargo test -p telaradio-ffi` — green
- `cargo test --workspace` — still green workspace-wide
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt --check` — clean
- `make ffi` — produces the static lib + header
- `make swift` — `swift build` succeeds with no errors
- **Manual** (only the user can verify): `make app-run` launches the
  app; clicking Play produces audible 440 Hz sine modulated at 16 Hz
  through the Mac speakers

### Risk notes

- **cbindgen + cdecl interaction with Rust 2024 edition**. Recent
  cbindgen versions may need a config option for edition 2024
  parsing. Worst case: pin cbindgen to a known-good version.
- **AVAudioEngine real-time thread**. Audio callbacks run on a
  dedicated thread; we cannot do allocation, locks, or syscalls
  there. Easiest path: schedule a single `AVAudioPCMBuffer` filled
  ahead of time. Audio playback runs from that pre-rendered buffer.
- **Static lib path resolution**. Hardcoded `../../../target/release`
  works in dev; an installed app needs to embed the dylib (Phase
  4-ish). For now, dev-only is fine.
- **macOS 13+ requirement** rules out very old Macs. Acceptable.
- **Swift package must build outside Xcode** (we're using SwiftPM).
  Some SwiftUI features need Xcode; for a single-window app shouldn't
  matter, but flag if encountered.

### Commit conventions

- `feat(ffi): C ABI shim for telaradio-core/dsp/model-adapter (phase 1d)`
- `feat(apple): macOS player MVL (phase 1d)`
- Trailing footer: `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`

### Stopping conditions

- All Rust quality gates green
- `swift build` succeeds (`make swift` exits 0)
- Phase report written and committed
- User notified that manual audio verification is required

If any of these fail, write a partial report and stop with a clear
description of the blocker.
