# Next steps

Phase 1 checklist lives in [`../../ROADMAP.md`](../../ROADMAP.md). Items
1–2 (Cargo bootstrap, recipe parser + validator) are complete. The next
slice is **Phase 1b — ACE-Step adapter + first-launch model download**.

## Phase 1b — Model adapter (next session)

1. Define the `Generator` trait in `lockstep-core` (signature is in
   `ARCHITECTURE.md` §Model abstraction interface).
2. Bootstrap the `model-adapter/` workspace member crate.
3. Decide IPC format with the Python subprocess (newline-delimited JSON
   over stdio is the leading candidate; confirm before building).
4. Decide audio interchange format (raw PCM in a temp file, base64'd
   inline, or shared-memory buffer). Affects round-trip latency.
5. Implement `AceStepGenerator { id, version, generate(...) }`.
6. First-launch model download: resumable HTTP from Hugging Face into
   `~/Library/Application Support/Lockstep/models/`. Also support
   pointing at a pre-existing weights path.
7. Integration test that downloads (or finds) the model, generates a
   short clip, and writes it to a temp file. Mark `#[ignore]` since it
   needs network + GPU.

## After Phase 1b

- Phase 1c — Rust AM modulation DSP (depends on `WavBuffer` shape in core)
- Phase 1d — macOS Swift app shell
- Phase 1e — Background buffer queue
- Phase 1f — Hand-seed ~20 starter recipes
- Phase 1g — Settings UI (preset / intensity / advanced)

## Decisions to make at the start of Phase 1b

- IPC format with Python (newline-delimited JSON vs. msgpack vs. gRPC)
- Audio interchange (temp file vs. inline base64 vs. shared memory)
- Audio sample rate (44.1 kHz is most likely; 48 kHz is a real
  alternative if AVFoundation prefers it)
- Mono vs. stereo for v1 (mono is simpler; stereo is what users expect)
- `Generator` trait async or sync? Async fits HTTP/gRPC server; sync is
  simpler for the subprocess. Probably both: blocking impl + an async
  wrapper in the backend.
