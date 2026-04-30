# Phase Report: Phase 1d2 — Real ACE-Step in the Swift App

**Date:** 2026-04-29
**Outcome:** pass (Rust + Swift build verification); audible-output check
on the real-model + use-existing paths pending user verification
**Spec:** [`phase-1d2-real-ace-step-spec.md`](phase-1d2-real-ace-step-spec.md)

## Changes

Three commits on `main`, dispatched as sequential Sonnet sub-agents:

- **`73ee6d7` — Step 1 (Rust FFI)**
  - `model-adapter/src/ace_step.rs` — added
    `pub fn ace_step_artifacts() -> &'static [ModelArtifact]` (canonical
    HF artifact list for ACE-Step v1 3.5B; sha256s are **placeholders**
    pending real-download bootstrap).
  - `model-adapter/src/lib.rs` — re-exported `ace_step_artifacts`,
    `ModelArtifact`, `InstallMode`, `CancellationToken`, `ensure_model`.
  - `model-adapter/src/model_install.rs` — extended `InstallMode::Download`
    to `(Option<ProgressCallback>, Option<CancellationToken>)` so the FFI
    can thread a caller-owned cancel token. Internal-only API break,
    confined to the model-adapter crate.
  - `ffi/src/lib.rs` — new C ABI surface: `tr_cancel_token_new`/`_cancel`/`_free`,
    `tr_ensure_model_download`, `tr_ensure_model_use_existing`,
    `tr_generate_ace_step`, `tr_string_free`. Used a `CtxPtr(*mut c_void)`
    `Send`-marked newtype + standalone `call_progress_cb` helper to satisfy
    the `Send` bound on the boxed closure.
  - `ffi/tests/ffi_round_trips.rs` — +7 tests (cancel-token lifecycle,
    use-existing happy + error paths, string-free safety, plus an `#[ignore]`d
    download e2e).
  - `ffi/Cargo.toml` — `tempfile` as dev-dep.

- **`c9c0df6` — Step 2 (Swift wrappers)**
  - `apple/Telaradio/Sources/Telaradio/Telaradio.swift` — extended the
    `Telaradio` enum with `ensureModelDownload(installDir:progress:)`,
    `ensureModelUseExisting(installDir:sourceDir:)`, and
    `generateAceStep(modelDir:prompt:seed:durationSeconds:)`. Added a
    private `ProgressContext` class for `Unmanaged.passRetained` /
    `release` bridging of the C progress callback. Hardcoded
    `aceStepTotalBytes: UInt64 = 5_000_000_000` since Step 1 didn't
    expose a `tr_ace_step_total_bytes()` helper; progress fraction
    clamps to `[0.0, 1.0]`. Progress callback dispatches to `MainActor`
    inside the C bridge so SwiftUI bindings get updates on the main
    thread without per-callsite ceremony.

- **`1328cdf` — Step 3 (SwiftUI sheet + view model branching)**
  - `apple/Telaradio/Sources/Telaradio/ModelSettings.swift` (new, 44 LOC) —
    `GenerationBackend` enum (`.mock` / `.aceStep`); `@MainActor`
    `ObservableObject` with `@Published var modelDir: URL?` and
    `@Published var backend`, both with `didSet` writing to `UserDefaults`.
    `defaultInstallDir` static let pointing at
    `~/Library/Application Support/Telaradio/models/ace-step-v1-3.5b`.
  - `apple/Telaradio/Sources/Telaradio/ModelSetupView.swift` (new, 116 LOC) —
    sheet with phase state machine (`.idle` / `.downloading` / `.installing`
    / `.picking`), three buttons (Download / Use existing folder /
    Use mock for now), error surface in red caption text.
  - `apple/Telaradio/Sources/Telaradio/PlayerView.swift` — replaced the
    self-instantiating `@StateObject var viewModel` with a custom `init()`
    that creates `ModelSettings` first and passes it to `PlayerViewModel`.
    Added `.sheet(isPresented:)` on `!modelSettings.isConfigured` with
    `.interactiveDismissDisabled()`.
  - `apple/Telaradio/Sources/Telaradio/PlayerViewModel.swift` — accepts
    `settings: ModelSettings` via init; `playExample()` now branches on
    `settings.backend` (mock keeps the existing 5-second sine path; aceStep
    calls `Telaradio.generateAceStep(...)` with a 30-second default
    duration when the recipe doesn't specify one).

## Decisions made

See `decisions.md` 2026-04-29 entry "Phase 1d2: real ACE-Step in the Swift
app" for the full list.

Mid-build:
- `InstallMode::Download` enum-variant arity bump (1 → 2 fields) to thread
  the FFI cancel token. Confined to `model-adapter`; only one external
  callsite touched (the CLI prompt parser).
- `tr_ace_step_total_bytes` helper deliberately not added — Swift hardcodes
  `5_000_000_000` and clamps to `[0, 1]`. Once real sha256s land we can
  swap to a sum-of-artifacts helper without an FFI break.
- `ModelSettings` owned by `PlayerView` (not the App) — simpler, and
  there's no second window to share state with yet.
- "Use mock" is sticky in `UserDefaults`; resetting requires
  `defaults delete com.telaradio.Telaradio` until Phase 1g adds a Settings
  panel.

## Verification results

```
$ cargo test --workspace
...
Total: 71 passed; 0 failed; 2 ignored
$ cargo clippy --all-targets -- -D warnings
clean (pedantic)
$ cargo fmt --check
clean
$ make ffi
cargo build -p telaradio-ffi → wrote header
$ cd apple/Telaradio && swift build
Build complete!  (zero Swift-compiler warnings; pre-existing macOS-target-
mismatch ld warnings unchanged from Phase 1d)
```

### Manual verification checklist

- [x] `cargo test --workspace` — 71/71 passing (2 ignored)
- [x] `cargo clippy --all-targets -- -D warnings` — clean (pedantic)
- [x] `cargo fmt --check` — clean
- [x] `make ffi` regenerates the C header with the new functions
- [x] `swift build` — zero warnings
- [ ] **(USER)** `make app-run` on a Mac with `defaults delete com.telaradio.Telaradio`
      first → sheet appears with three buttons.
- [ ] **(USER)** "Use mock for now" → sheet dismisses, Play still
      produces the 440 Hz sine modulated at 16 Hz (regression check).
- [ ] **(USER, optional)** "Use existing folder" against a real ACE-Step
      checkpoint → sheet dismisses, Play generates real audio modulated
      at 16 Hz. Now unblocked — see "Bootstrap follow-up landed" below.

## Bootstrap follow-up landed (2026-04-30)

The placeholder-sha256 caveat is now resolved. Original Step 1 sha256s
were placeholder strings; this session replaced them with real values
**without** doing a 7.7 GB download by reading them from HF's
`?blobs=true` API (the `lfs.sha256` field). The non-LFS JSON configs
(~80 KB total) were downloaded and hashed locally.

While verifying the manifest, three additional bugs surfaced:

1. The safetensors filenames in `ace_step_artifacts()` were wrong —
   they're `diffusion_pytorch_model.safetensors`, not `model.safetensors`.
   The placeholder URLs would have 404'd even if the sha256s had been
   real.
2. The umt5-base text encoder was missing its `model.safetensors`
   (1.13 GB) plus `special_tokens_map.json` and `tokenizer_config.json`.
   The `transformers` library would have failed to load the encoder
   without these.
3. The Swift `aceStepTotalBytes` constant was hardcoded to `5_000_000_000`
   — well below the real ~7.7 GB total. Replaced with a call to a new
   FFI export `tr_ace_step_total_bytes()` backed by the new
   `ACE_STEP_TOTAL_BYTES` constant in `ace_step.rs` (8_275_790_207 bytes).

Net diff: 11 artifacts in the manifest (was 8), real sha256s for all,
correct URLs, and a single source of truth for the total byte count.
All quality gates green afterward (cargo test/clippy/fmt + make swift).

## Follow-up items

- [ ] **Reconcile the macOS deployment target mismatch between Rust and
      Swift.** Pre-existing `ld: warning: object file ... was built for
      newer 'macOS' version (26.0) than being linked (13.0)` — Rust
      static lib targets the host (26.0) while SwiftPM links 13.0. The
      `Makefile` already exports `MACOSX_DEPLOYMENT_TARGET=13.0`, but
      it's not being picked up by `cargo build` for the Rust deps. Fix:
      either pin `[build] rustflags = ["-C", "link-arg=-mmacosx-version-min=13.0"]`
      in `.cargo/config.toml`, or set the env var at the workspace
      level. Doesn't block functionality.
- [ ] **`settings.modelDir!` force-unwrap in `PlayerViewModel`** is safe
      today (sheet blocks Play until configured), but fragile if Phase 1g
      adds a Settings panel that lets the user clear the path mid-session.
      Replace with a `guard let dir = settings.modelDir else { ... }` and
      surface a clear error.
- [ ] **No "Change model…" affordance.** Phase 1g Settings panel should
      include this so users don't need to `defaults delete` to re-run setup.
- [ ] **No download cancel UI.** Cancel token exists at the FFI; the
      Swift wrapper creates and frees its own internally. Phase 1g should
      surface a Cancel button on the Download progress view.
- [ ] **`ProgressContext` retain leak edge case.** If Phase 1g introduces
      Swift-side `Task` cancellation mid-download, the `Unmanaged.release`
      after the FFI call may be skipped. Either move the release into a
      `defer` (already done) and ensure the Task cancellation doesn't
      bypass the FFI return, or migrate to `Unmanaged.passUnretained`
      with a separate retain held by the caller.

## Context for next phase

- The full pipeline `recipe → ACE-Step → AM → playback` is now wired
  end-to-end in code. The remaining gap is data (real sha256s) and UX
  polish (Settings panel, cancel UI), not architecture.
- Phase 1e (background buffer queue) is the natural next hop. Real
  ACE-Step has ~10 s cold-start latency on consumer hardware; without a
  buffer of 2–3 pre-generated tracks the user feels every Play click.
- The Swift wrappers are designed so 1e can plug in without touching
  the FFI: `Telaradio.generateAceStep(...)` is the seam.
- One landmine: the 30-second default duration in `PlayerViewModel` is
  hardcoded. Once recipes routinely specify durations, move the default
  into recipe parsing (or a `Recipe.durationSeconds(default:)` helper).
