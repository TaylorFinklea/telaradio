# Phase Spec: Phase 1d2 — Real ACE-Step in the Swift App

**Predecessors:** Phase 1d MVL (`phase-1d-macos-player-spec.md`),
Phase 1b2 (`phase-1b2-ace-step-spec.md`).
**Goal:** Replace the hardcoded mock-generation call in the macOS Swift
app with a real ACE-Step path, gated by a first-launch model-setup sheet
that downloads from Hugging Face or accepts an existing folder.

## Why

Phase 1d MVL plays a 440 Hz sine no matter what the recipe says. Phase 1f
(starter recipe library) is blocked on real generation — recipes can't be
ear-tested while every prompt produces a sine. Phase 1d2 is the last
plumbing layer needed before recipe authoring becomes useful.

## Acceptance criteria

- [ ] First launch (no model configured) → SwiftUI sheet appears asking
      the user to choose a model source.
- [ ] Sheet has three buttons:
      - **Download (~5 GB)** — drives `tr_ensure_model_download` via the
        Rust resumable downloader; progress bar updates 0 → 100%.
      - **Use existing folder** — `NSOpenPanel` directory picker; calls
        `tr_ensure_model_use_existing` to validate + symlink/copy.
      - **Use mock for now** — sets backend to mock, dismisses.
- [ ] Resolved model directory persists in `UserDefaults` so subsequent
      launches skip the sheet.
- [ ] `PlayerViewModel.playExample()` branches on the configured backend:
      mock vs ACE-Step.
- [ ] All existing 65 Rust tests still pass (1 ignored unchanged).
- [ ] New FFI integration tests cover cancel-token lifecycle and the
      use-existing path. Download path stays `#[ignore]`d.
- [ ] `cargo test --workspace`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check` all clean.
- [ ] `make ffi && make swift && make app-run` end-to-end.

## Architecture

```
SwiftUI sheet (ModelSetupView)
  ├── "Download"        ──► Telaradio.ensureModelDownload(progress:)
  │                          └── tr_ensure_model_download (FFI)
  │                              └── model_install::ensure_model(InstallMode::Download(cb))
  │                                  └── hf_download::download_with_resume
  ├── "Use existing"    ──► Telaradio.ensureModelUseExisting(sourceDir:)
  │                          └── tr_ensure_model_use_existing (FFI)
  │                              └── model_install::ensure_model(InstallMode::UseExisting(...))
  └── "Use mock"        ──► sets ModelSettings.backend = .mock

ModelSettings (UserDefaults-backed)
  ├── modelDir: URL?
  └── backend: { mock, aceStep }

PlayerViewModel.playExample()
  ├── if backend == .mock     ──► Telaradio.generateMock(...)  ──► applyAM ──► play
  └── if backend == .aceStep  ──► Telaradio.generateAceStep(modelDir:...) ──► applyAM ──► play
```

## Existing Rust API surface (do not rebuild)

Confirmed via Explore agent against the live code:

| Symbol | Path:line | Notes |
|---|---|---|
| `AceStepGenerator::spawn(model_dir: &Path)` | `model-adapter/src/ace_step.rs:48` | Pre-resolved model dir; does NOT call `ensure_model` itself |
| `Generator::generate(prompt, seed, duration_seconds)` | `model-adapter/src/ace_step.rs:78` | Returns `Result<WavBuffer, GeneratorError>` |
| `ensure_model(install_dir, artifacts, mode)` | `model-adapter/src/model_install.rs:66` | Idempotent; sha256-validated |
| `InstallMode::Download(Option<ProgressCallback>)` / `UseExisting(PathBuf)` | `model-adapter/src/model_install.rs:39` | |
| `ModelArtifact { url, relative_path, sha256 }` | `model-adapter/src/model_install.rs:32` | |
| `ProgressCallback = Box<dyn FnMut(u64) + Send>` | `model-adapter/src/hf_download.rs:30` | u64 = cumulative bytes written |
| `download_with_resume(url, dest, sha256, progress, cancel)` | `model-adapter/src/hf_download.rs:81` | |
| `CancellationToken { new, cancel, is_cancelled }` | `model-adapter/src/hf_download.rs:34` | Cheap clone, atomic-bool-backed |

**Open question for Step 1**: Where does the canonical ACE-Step
`&[ModelArtifact]` manifest currently live (likely 1b2 test fixtures)?
Step 1 either lifts it into a public
`pub fn ace_step_artifacts() -> &'static [ModelArtifact]`, or — if the
manifest already exists publicly — re-exports it. Either way,
the FFI must be able to call `ensure_model` with the manifest without
duplicating it.

## Existing Swift patterns (mirror, do not reinvent)

| Pattern | Path:line |
|---|---|
| `TelaradioError` + `lastFFIError()` | `apple/Telaradio/Sources/Telaradio/Telaradio.swift:24` |
| `final class WavBuffer` / `Recipe` (ARC + opaque pointer + deinit free) | `Telaradio.swift:40–87` |
| `withCString` for string args; null-guard return | throughout `Telaradio.swift` |
| `@MainActor` `ObservableObject` view model with `@Published` enum status | `PlayerViewModel.swift:11` |
| Long-running FFI in `Task.detached { … }.value` | `PlayerViewModel.swift:45–83` |

**Not yet present**: `NSOpenPanel`, `UserDefaults`, sheets/modals. Step 3
introduces all three.

## Step 1 — Rust: FFI shims (Sonnet)

### Files

- **modify** `model-adapter/src/lib.rs` — re-export `ace_step_artifacts`
  (new), `ModelArtifact`, `InstallMode`, `CancellationToken`, `ensure_model`.
- **modify** `model-adapter/src/ace_step.rs` — add
  `pub fn ace_step_artifacts() -> &'static [ModelArtifact]` returning the
  canonical artifact list. Lift from existing 1b2 fixture if needed.
- **modify** `ffi/src/lib.rs` — new functions per the table below.
- **modify** `ffi/cbindgen.toml` — add `TrCancelToken` to the
  forward-declared opaque types in `after_includes` (alongside `TrRecipe`,
  `TrWavBuffer`).
- **modify** `ffi/tests/ffi_round_trips.rs` — tests for cancel-token
  lifecycle + `tr_ensure_model_use_existing` happy path. Ignored test for
  download path mirrors 1b2's `#[ignore]` precedent.

### New FFI surface

| Function | Returns | Errors |
|---|---|---|
| `tr_cancel_token_new() -> *mut TrCancelToken` | owned token | never null |
| `tr_cancel_token_cancel(*mut TrCancelToken)` | void | safe on null (no-op) |
| `tr_cancel_token_free(*mut TrCancelToken)` | void | safe on null |
| `tr_ensure_model_download(install_dir: *const c_char, progress_cb: Option<extern "C" fn(*mut c_void, u64)>, ctx: *mut c_void, cancel: *const TrCancelToken) -> *mut c_char` | resolved-path C string (caller frees via `tr_string_free`) | null + `tr_last_error()` |
| `tr_ensure_model_use_existing(install_dir: *const c_char, source_dir: *const c_char) -> *mut c_char` | resolved-path C string | null + `tr_last_error()` |
| `tr_generate_ace_step(model_dir: *const c_char, prompt: *const c_char, seed: u64, duration_seconds: u32) -> *mut TrWavBuffer` | owned buffer | null + `tr_last_error()` |
| `tr_string_free(*mut c_char)` | void | safe on null |

### Implementation notes for Step 1 agent

- The progress callback bridges from `Box<dyn FnMut(u64)>` to
  `extern "C" fn`. The Rust wrapper boxes a closure that captures `ctx`
  and calls the C fn pointer. **Document loudly** in the header doc that
  the callback runs on whatever thread the download is happening on; the
  Swift caller marshals to the main thread.
- `tr_string_free` should drop a `CString` whose ownership originated in
  Rust. Use `CString::from_raw` and let it drop.
- For `tr_generate_ace_step`: spawn the generator, run `generate`, drop
  the generator. Mirrors `tr_generate_mock`'s spawn-use-drop pattern. Yes,
  this means ~few-second subprocess startup per call — Phase 1e (buffer
  queue) is the proper fix; do not over-engineer here.
- Pass the cancellation token through as an `Arc`-cloned reference; the
  caller-owned token outlives the call.
- TDD: write a failing test for cancel-token round-trip first, then the
  use-existing test. Watch each fail, then implement minimum.

### Acceptance gates for Step 1

- All quality gates green (test/clippy pedantic/fmt).
- `make ffi` regenerates the header; new functions appear with doxy comments.
- New FFI tests: at least 3 (token lifecycle, use-existing happy path,
  use-existing error case for missing source dir).
- A single commit on main with conventional-commits style:
  `feat(ffi): expose ensure_model + generate_ace_step + cancel token (phase 1d2 step 1)`.

## Step 2 — Swift wrappers (Sonnet)

Modify only `apple/Telaradio/Sources/Telaradio/Telaradio.swift`. Add to
the `Telaradio` enum:

```swift
static func ensureModelDownload(
    installDir: URL,
    progress: @escaping (Double) -> Void
) async throws -> URL

static func ensureModelUseExisting(
    installDir: URL,
    sourceDir: URL
) async throws -> URL

static func generateAceStep(
    modelDir: URL,
    prompt: String,
    seed: UInt64,
    durationSeconds: UInt32
) async throws -> WavBuffer
```

### Implementation notes for Step 2 agent

- Bridge the C progress callback through a class-based context:
  ```swift
  private final class ProgressContext {
      let onProgress: (Double) -> Void
      let totalBytes: UInt64
      init(onProgress: @escaping (Double) -> Void, totalBytes: UInt64) { … }
  }
  ```
  Pass via `Unmanaged.passRetained(ctx).toOpaque()`. The C callback wraps
  it as `Unmanaged.fromOpaque(...).takeUnretainedValue()` and computes
  `Double(bytes) / Double(totalBytes)`. After the FFI call returns,
  `Unmanaged.fromOpaque(ctx).release()`.
- The progress callback may fire on a non-main thread. Inside the C
  callback Swift wrapper, dispatch to `MainActor` before invoking
  `onProgress`. (`Task { @MainActor in onProgress(...) }` works.)
- Total bytes for ACE-Step: hardcode `static let aceStepTotalBytes:
  UInt64 = ` whatever the manifest sums to, OR add a Rust helper
  `tr_ace_step_total_bytes() -> u64` if Step 1 didn't already. Cleaner
  to do the helper — propose it back to Step 1 if not present after.
- Wrap each call in `withCheckedThrowingContinuation` + `Task.detached`
  so the blocking download doesn't pin the main actor.
- Convert `URL` to a path C-string with `path(percentEncoded: false)`
  (avoid `.path` which is deprecated on newer macOS SDKs).

### Acceptance gates for Step 2

- `make swift` builds clean, zero warnings.
- A single commit on main:
  `feat(apple): Telaradio Swift wrappers for ACE-Step + ensure_model (phase 1d2 step 2)`.

## Step 3 — SwiftUI sheet + view model branching (Sonnet)

### New files

- `apple/Telaradio/Sources/Telaradio/ModelSettings.swift` — UserDefaults-
  backed observable:
  ```swift
  enum GenerationBackend: String { case mock, aceStep }

  @MainActor final class ModelSettings: ObservableObject {
      @Published var modelDir: URL?
      @Published var backend: GenerationBackend
      var isConfigured: Bool { backend == .mock || modelDir != nil }
      // UserDefaults keys: "modelDir" (path string), "backend" (rawValue)
  }
  ```
- `apple/Telaradio/Sources/Telaradio/ModelSetupView.swift` — sheet with
  three buttons; Download branch shows a `ProgressView(value:total:)`.

### Modified files

- `PlayerView.swift` — `@StateObject var modelSettings = ModelSettings()`;
  add `.sheet(isPresented: !$modelSettings.isConfigured) { ModelSetupView(settings: modelSettings) }`.
- `PlayerViewModel.swift` — accept `settings: ModelSettings` in init;
  in `playExample()` branch on `settings.backend`. ACE-Step path:
  `Telaradio.generateAceStep(modelDir: settings.modelDir!, prompt:
  recipe.prompt, seed: recipe.seed, durationSeconds: …)`.
- `TelaradioApp.swift` — instantiate `ModelSettings` at App level (or
  rely on `PlayerView`'s `@StateObject`). Pick whichever survives window
  changes cleanly.

### Acceptance gates for Step 3

- `make app-run` smoke (manual):
  - With `defaults delete <bundleId>` first, sheet appears.
  - "Use mock for now" → sheet dismisses, Play works (regression
    against Phase 1d MVL).
  - "Use existing folder" picks a directory; if it contains a real
    ACE-Step model, Play generates real audio. (User-verified manually.)
- A single commit on main:
  `feat(apple): first-launch model-setup sheet + view model branching (phase 1d2 step 3)`.

## Wrap-up (Opus, after Step 3)

1. Write `.docs/ai/phases/phase-1d2-real-ace-step-report.md`.
2. Update `current-state.md`, `next-steps.md`, `decisions.md`,
   `apple/README.md`.
3. Mark Phase 1 item 6 `[x]` in `ROADMAP.md`.
4. Queue Phase 1e (background buffer queue).

## Out of scope (deliberate)

- Recipe selection UI / file picker (Phase 1g).
- Model upgrade / multi-model coexistence (Phase 4).
- Download cancellation **UI** (cancel token exists at FFI; no UI button
  in 1d2 — add in 1g if felt).
- Re-running setup after first launch (would need a Settings menu;
  workaround is `defaults delete` for now).
- `.app` bundle distribution (Phase 4).
