# Decision log

Append-only record of non-obvious design, tooling, or scope decisions.
Each entry: date, decision, rationale, what it supersedes (if anything).

## 2026-04-29 — Phase 1d2: real ACE-Step in the Swift app

1. **Sequential sub-agent dispatch over parallel.** Phase 1d2 has three
   layers (Rust FFI, Swift wrappers, SwiftUI sheet) where each depends
   on the previous layer's API surface. Dispatched as three sequential
   Sonnet sub-agents, each reading the same Phase 1d2 spec, each
   committing on `main` when green. Saved Opus context for the
   spec-writing + verification + report-writing portions.

2. **`InstallMode::Download` arity bump** (1 → 2 fields, adding
   `Option<CancellationToken>`). The internal API previously created
   its own token; the FFI needed to thread a caller-owned token
   through. Confined to `model-adapter`; one external callsite (CLI
   prompt parser) updated. Cleaner than introducing a new variant or
   a parallel function.

3. **Real sha256s sourced from HF API, not from a local download.**
   Phase 1d2 originally shipped with placeholder strings; the bootstrap
   follow-up (2026-04-30) replaced them via Hugging Face's
   `?blobs=true` API, which exposes `lfs.sha256` for every LFS file.
   Only the small JSON configs (~80 KB total) had to be downloaded and
   hashed locally. Total avoided download: ~7.7 GB. While verifying the
   manifest, three pre-existing bugs surfaced and were fixed: the
   safetensors filenames were `model.safetensors` (should be
   `diffusion_pytorch_model.safetensors`); the umt5-base text encoder
   was missing `model.safetensors`, `special_tokens_map.json`, and
   `tokenizer_config.json`; manifest count went 8 → 11.

4. **`tr_ace_step_total_bytes()` exposed via FFI.** Phase 1d2 originally
   hardcoded `aceStepTotalBytes: UInt64 = 5_000_000_000` in Swift (well
   below the real ~7.7 GB). Bootstrap follow-up added
   `pub const ACE_STEP_TOTAL_BYTES: u64 = 8_275_790_207` in
   `model-adapter/src/ace_step.rs` and exposed it through the FFI as
   `tr_ace_step_total_bytes() -> u64`. Single source of truth for the
   download progress UI denominator; no risk of drift between Rust and
   Swift sides.

5. **Progress callback dispatches to `MainActor` inside the C bridge.**
   The download fires the C callback dozens of times per second on the
   download thread; SwiftUI's `@Published` properties on a `@MainActor`
   `ObservableObject` MUST receive updates on the main thread. Doing
   the dispatch once at the C-bridge layer (via `Task { @MainActor in … }`)
   avoids per-callsite ceremony and is fire-and-forget so the
   download isn't slowed by UI updates.

6. **`ensure_model` and `AceStepGenerator::spawn` stay separate, not
   wired together.** The original Phase 1b2 follow-up proposed wiring
   `ensure_model` inside `spawn`. Rejected: the Swift wrapper calls
   them in sequence already (ensure first, then spawn-use-drop), and
   keeping them separate makes the FFI surface compose better — e.g.
   a future Phase 1g "Re-validate model" button can call ensure
   without re-spawning a generator.

7. **`ModelSettings` owned by `PlayerView` via `@StateObject`, not at
   App level.** Simpler today (no second window to share state with).
   Easy to lift to App level later if Phase 2's iOS port shares the
   same SwiftPM target.

8. **"Use mock for now" is sticky in `UserDefaults`.** No "Change
   model…" affordance until Phase 1g Settings panel; workaround is
   `defaults delete com.telaradio.Telaradio`. Trade-off: keeps the
   first-launch flow simple at the cost of dev-loop friction during
   Phase 1f recipe authoring.

9. **No download cancel UI in 1d2.** Cancel token exists at the FFI;
   the Swift wrapper creates + frees its own internally. Surfacing a
   Cancel button is Phase 1g concern. Acceptable because the model is
   only ~5 GB and most users will use the existing-folder path or a
   one-time download.

## 2026-04-26 — Phase 0 foundational decisions

See [`../../PHASE_0_REPORT.md`](../../PHASE_0_REPORT.md) for the full
record. Summary:

1. Codename: **Telaradio**
2. Platform: **Hybrid** (Rust backend + SvelteKit web + Swift native)
3. Recipe storage: **Local + GitHub sync**
4. Modulation UX: **All three modes, switchable in settings**
5. Pre-generation: **Background buffer of 2–3 tracks**
6. Nature soundscapes: **Out of v1; Phase 2 with CC0 sources** (freesound
   + sonniss; myNoise has no public API)
7. Model distribution: **First-launch download from Hugging Face**

Decisions deferred (to be revisited when relevant):

- Voting design (PR reactions vs. dedicated service) — Phase 4
- Auth scheme for shared backend — Phase 2
- Mono vs. stereo, sample rate convention — Phase 1b
- Recipe PR lint rules — Phase 2 when CI exists

## 2026-04-27 — Phase 1a recipe-core decisions

1. **GitHub repo**: `TaylorFinklea/telaradio` created public on
   2026-04-27. Phase 0 scaffold + Phase 1a build pushed as initial
   history.
2. **Recipe `id` typing**: strict `uuid::Uuid` via the `uuid` crate
   (feature flag `serde`, `v4`). Malformed IDs fail at parse time.
3. **Cargo workspace**: single root workspace with `core` as the first
   member crate. `[workspace.dependencies]` pins shared deps; member
   crates declare `dep.workspace = true`.
4. **Recipe parser strictness**: `#[serde(deny_unknown_fields)]` on
   every recipe struct. Unknown JSON fields are a parse error.
   Forward-compat requires explicit `schema_version` bumps.

Mid-build judgment calls (logged here rather than re-asking):

- **Inline JSON in tests** instead of file-per-fixture under
  `core/tests/fixtures/`. Spec mentioned fixture files; switched to
  inline literals for readability. The one exception is the realistic
  example recipe at `recipes/example-foggy-lofi.json`, which doubles as
  a deliverable and as a file-loading round-trip test.
- **Validation order in `Recipe::parse`**: serde structural validation
  first (catches unknown fields, missing fields, wrong types, malformed
  UUID, unknown envelope), then a single `validate()` pass for semantic
  invariants (schema version, depth range, rate, duration).
- **Edition 2024**: workspace pins `edition = "2024"` and
  `rust-version = "1.85"`, since the toolchain installed locally is
  rustc 1.95.

## 2026-04-27 — Phase 1b: Generator trait + mock subprocess

1. **Phase 1b split from 1b2.** Phase 1b lands the `Generator` trait,
   IPC contract, and a mock-engine subprocess that returns a 440 Hz
   sine. Phase 1b2 swaps the mock for real ACE-Step inference + HF
   model download. Reason: with the mock, downstream phases (DSP,
   Swift app, buffer queue) can be built without depending on a 5 GB
   model or GPU. Faster feedback loops, cleaner test isolation.
2. **`Generator` trait is synchronous.** Backend HTTP/gRPC layer in
   Phase 2 will wrap it async. Keeping it sync means `core` stays
   runtime-free.
3. **IPC: NDJSON over stdio + temp WAV file**. Debuggable, no extra
   deps; ACE-Step's ~10s latency dwarfs IPC cost.
4. **One subprocess per `SubprocessGenerator` instance**. Held open
   across `generate` calls; killed + reaped on `Drop`. The Python
   spawn cost (~200 ms) is paid once.
5. **Audio defaults: 44.1 kHz, stereo, 16-bit signed PCM**. Constants
   live at `core::audio::DEFAULT_SAMPLE_RATE_HZ` (44_100) and
   `core::audio::DEFAULT_CHANNELS` (2). Generators target these unless
   they document otherwise.
6. **Mock generator id/version: `"mock-sine"` / `"0.1.0"`**. Stable
   identifiers so a recipe authored against the mock points at it
   forever; ACE-Step takes a different `id`.
7. **`#[serde(tag = "kind", rename_all = "lowercase")]` on Response**.
   Wire format: `{"kind":"ok",...}` and `{"kind":"err","message":...}`.
   Tested explicitly in `protocol_serde.rs`.

Mid-build judgment calls (logged here, not re-asked):

- **Single `Mutex<IoState>` over the subprocess's child + stdin +
  stdout**. `Generator::generate` takes `&self`, so concurrent calls
  serialize naturally on the lock — which matches the subprocess's
  request-response semantics anyway.
- **Best-effort temp WAV cleanup**. Adapter calls `fs::remove_file`
  but ignores errors. If cleanup matters more later, switch to an
  RAII wrapper; for Phase 1b it's noise.
- **Edition 2024 reserved keyword `gen`**. Test variables renamed to
  `generator` to avoid the reserved keyword.
- **Clippy `unnecessary_literal_bound` allowed in tests only**. Real
  `Generator` impls return `&'static str` and clippy is happy; the
  in-memory test impl is annotated locally rather than changing the
  trait signature to `-> &'static str`, which would constrain future
  impls that might want non-static identity.

## 2026-04-28 — Phase 1b2: real ACE-Step + HF download

1. **Two trait impls, not one parameterized subprocess.**
   `AceStepGenerator` lives next to `SubprocessGenerator`, with the
   only varying axes being the script path / Python interpreter /
   id / version. Shared NDJSON plumbing lives in a private
   `ipc::IpcChannel`. Cleaner than a runtime engine flag and keeps
   the distinct ids honest in the recipe schema.
2. **`ACE_STEP_GENERATOR_ID = "ace-step-1.5-xl"`** despite the actual
   HF checkpoint being `ACE-Step/ACE-Step-v1-3.5B`. The id is meant to
   be stable for recipes; we kept the spec's name to avoid a churn on
   day one. Resolved at merge time by a rename — see the 2026-04-28
   "model id rename" entry below.
3. **Resumable HTTP is pure Rust (`reqwest` + `rustls-tls`),
   blocking.** Async would have forced a tokio runtime onto the
   sync `Generator` trait. Rules out openssl on the dep surface.
4. **Manifest is a single `manifest.json` of `[ModelArtifact]`.** Each
   artifact carries `url`, `relative_path`, `sha256`. `ensure_model`
   re-validates every artifact on every call; idempotency comes from
   the validation passing.
5. **`UseExisting` copies, doesn't symlink.** Avoids surprises if the
   source disappears later; ~5 GB extra disk is acceptable.
6. **`prompt_install_mode_cli` is reader/writer-generic.** Tested
   against `&[u8]`/`Vec<u8>` rather than `stdin`/`stderr`. Real
   callers pass the std streams.
7. **`#[ignore]` on the real-model e2e.** Requires ~5 GB checkpoint
   + a primed venv; opt in with `TELARADIO_MODEL_DIR=...
   --include-ignored`. Mocked smoke tests cover the Rust IPC contract
   so CI stays meaningful.
8. **Lazy pipeline load in `telaradio_ace_step.py`.** Subprocess
   starts cheap (no torch import on `--probe`); first request triggers
   the model load. Lets the Rust caller spawn the subprocess
   speculatively.
9. **`ace-step` installed from a GitHub commit pin.** Upstream's PyPI
   sdist is broken (`setup.py` reads `requirements.txt` but the file
   isn't in the tarball — `FileNotFoundError`). Workaround:
   `[tool.uv.sources] ace-step = { git = "...", rev = "1bee4c9f..." }`.
   Switch back to a plain PyPI version when upstream re-publishes.
10. **Stop gitignoring `**/uv.lock`.** Reproducible installs require
    the lockfile in the repo. Phase 1b's gitignore was conservative;
    Phase 1b2 reverses it now that we have a real uv project.

## 2026-04-27 — Project rename: Lockstep → Telaradio

The project was codenamed **Lockstep** during the 2026-04-26 session
(a reference to neural phase-locking, the mechanism this project
implements per Woods et al. 2024). On 2026-04-27 it was renamed to
**Telaradio**, with domain `telaradio.com`. All documentation, code,
the Cargo crate name (`telaradio-core`), and the GitHub repo
(`TaylorFinklea/telaradio`) were updated in a single commit. Earlier
entries above have been edited to use the new name; this entry is the
single record of the rename event.

**Why:** Decision belongs to the user; not re-litigated here. Telaradio
is distinctive, trademark-clean, single-word, and the `.com` domain is
secured.

**Implication for future sessions:** if you find a stray reference to
"Lockstep" anywhere in the repo, it's a leftover from the rename and
should be updated.

## 2026-04-26 — Phase 1c: AM modulation DSP

1. **`dsp::Envelope` is decoupled from `core::recipe::Envelope`.** A
   small `From<core::recipe::Envelope>` impl bridges the two for
   pipeline glue. Reason: DSP can grow new envelope shapes (rate
   modulation, stochastic gating, etc.) without forcing a recipe
   schema bump. Cost is six lines.
2. **Internal AM math is f32 for the per-sample multiplication and
   f64 for phase / gate computation.** Phase accumulates over many
   samples (drift matters); gate is in `[0, 1]` and gets multiplied
   into f32 audio (precision is dominated by the f32 sample
   eventually). The f64 → f32 cast at multiplication time is a
   sub-LSB rounding on a bounded value.
3. **1 ms anti-click ramp, hard-coded.** Centered on each Square
   transition (±0.5 ms each side). Centering preserves the average
   gate value over a full cycle, keeping loudness statistics stable
   across `apply_am` calls. Configurability is a Phase 2 candidate.
4. **Stereo channels modulated identically.** Paper-faithful per
   Woods et al. 2024. The same gate value applies to both channels of
   each frame; no per-channel decorrelation. Future "stereo widening"
   transforms would live as separate DSP stages downstream.
5. **Sine and Triangle envelopes ship now**, even though only Square
   is in the recipe v1 default. Cost is small (a `match` arm each)
   and the recipe schema already accepts `"sine"` / `"triangle"`.
   This avoids a "sine/triangle do nothing" footgun if a recipe
   author flips the field.
6. **`apply_am` is a free function, not a method on `WavBuffer`.**
   Keeps `core::audio` runtime-free and DSP-agnostic. Pipeline code
   reaches `apply_am(&buf, ...)` directly.
7. **Pipeline smoke test lives in `model-adapter/tests/`**, not
   `dsp/tests/`. It exercises both the mock subprocess and the DSP
   crate; placing it next to the mock keeps `telaradio-dsp` free of
   process-spawning test deps.

Mid-build judgment calls (logged here, not re-asked):

- **`From<core::recipe::Envelope> for dsp::Envelope` shipped now**
  even though the spec deferred it. Pipeline code (Phase 1d/1e)
  needs it; six lines is not worth a future commit.
- **Cast lints** (`cast_precision_loss`, `cast_possible_truncation`,
  `cast_sign_loss`) are allowed locally on `apply_am` and at module
  level in the integration tests. Inline rationale comment in
  `am.rs` explains why each cast is safe.
- **Test-module-level `#![allow]` for clippy casts**, not a
  per-function allow. Audio-math tests have casts every few lines;
  per-test allows would be noisier than the lints they suppress.

## 2026-04-28 — Model id rename: `ace-step-1.5-xl` → `ace-step-v1-3.5b`

The Phase 1b2 agent flagged that the recipe-pinning constant
`ACE_STEP_GENERATOR_ID = "ace-step-1.5-xl"` and recipe `model.version =
"1.5.0"` did not match the actual Hugging Face checkpoint name
`ACE-Step/ACE-Step-v1-3.5B`. The "1.5 XL" label came from the original
session 0 prompt and was either outdated or never accurate.

**Resolved at merge time** (this session, 2026-04-28) by renaming
across the codebase to match HF reality:

- `ACE_STEP_GENERATOR_ID = "ace-step-v1-3.5b"` (lowercase, hyphenated,
  tracks the HF model name).
- `ACE_STEP_GENERATOR_VERSION = "1.0.0"` (Telaradio's first integration
  of ACE-Step v1; bump on future behavior changes).
- `recipes/example-foggy-lofi.json`, `core/tests/recipe_parse.rs`
  fixture, `.github/ISSUE_TEMPLATE/recipe_proposal.md`, ARCHITECTURE.md
  examples, CLAUDE.md prose, PHASE_0_REPORT.md section header all
  updated to match.

**Why this was safe to do at merge:** no recipes had been authored in
the wild yet pinning the old id (the only example recipe is the one we
also renamed). After the first community recipes ship, the id is
effectively locked and a future rename would break their reproducibility.

## 2026-04-28 — Phase 1d MVL: macOS player shell

1. **FFI surface: `staticlib` + cbindgen-generated header + Swift
   `module.modulemap`.** Picked over a CLI-helper subprocess because
   (a) a real macOS app spawning a CLI tool is unidiomatic, (b) static
   lib avoids per-generate spawn cost, (c) iOS / App Store
   distribution later essentially requires this approach.
2. **Forward declarations in `cbindgen.toml`'s `after_includes`.** The
   FFI references `Recipe` / `WavBuffer` types from sibling crates
   that cbindgen can't see (parse_deps = false). Manually injecting
   `typedef struct TrRecipe TrRecipe;` is cleaner than configuring
   cbindgen to traverse external crates.
3. **`MACOSX_DEPLOYMENT_TARGET=13.0` exported by the Makefile.**
   Aligns the Rust static lib's macOS minimum with the SwiftPM
   package's `platforms: [.macOS(.v13)]`. Without this, the linker
   warns about objects built for newer macOS than being linked.
4. **Phase 1d MVL is mock-only.** The Swift app calls
   `tr_generate_mock`, not `tr_generate_ace_step`. Real-model wiring
   is Phase 1d2. Reason: every architectural piece (FFI, build, Swift,
   AVAudioEngine) is new in this phase; wiring real ACE-Step on top
   would multiply the things that could go wrong. Land them
   sequentially.
5. **Hardcoded recipe path resolved by walking up to find
   `Cargo.toml`.** Avoids embedding paths into the binary. Works for
   `swift run` from the workspace and for any in-tree binary location.
   Replaced when the file picker ships (Phase 1g or earlier if felt).
6. **AVAudioEngine plays a single `AVAudioPCMBuffer` scheduled with
   `.interrupts`.** No streaming, no scrubbing. Matches the MVL
   "press Play, hear modulated sine" goal.
7. **Swift wrappers own the FFI pointers via `final class` + `deinit`.**
   ARC handles the lifetime; no manual `tr_*_free` calls in app code.
   `Recipe` and `WavBuffer` Swift classes hold `OpaquePointer`s that
   were `Box::into_raw`-ed in Rust.
8. **`linkerSettings: [.unsafeFlags(["-L../../target/debug"])]`.**
   Hardcoded relative path from `apple/Telaradio/` to the Rust target
   dir. Brittle but explicit; the Makefile orchestrates rebuilds in
   the right order. Phase 4-ish App Store distribution will replace
   this with a vendored / embedded library.

Mid-build judgment calls (not re-asked):

- **cbindgen 0.27 → 0.29.** 0.27 didn't parse the Rust 2024
  `#[unsafe(no_mangle)]` syntax and produced an empty header. 0.29
  works.
- **`#[allow(clippy::cast_precision_loss)]` at the FFI test module
  level.** Test data generators cast small loop indices to f32; the
  values fit comfortably in f32's mantissa. Per-line allows would be
  noisier than the suppressed lint.
- **Used `OpaquePointer` directly in Swift wrappers**, not
  `UnsafePointer<TrRecipe>`. Forward-declared C types import as
  `OpaquePointer?` in Swift; matching that simplifies the bridging.
