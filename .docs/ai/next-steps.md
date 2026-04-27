# Next steps

The Phase 1 checklist lives in [`../../ROADMAP.md`](../../ROADMAP.md). The
items below are sequenced for a fresh Phase 1 session.

## Immediate (next session)

1. Initialize git in `/Users/tfinklea/git/musicapp/` and make the first
   commit (the Phase 0 scaffold).
2. Decide whether to host on GitHub yet (likely yes — the canonical library
   repo design depends on it).
3. Bootstrap the Cargo workspace under `core/` with a minimal recipe types
   crate matching `ARCHITECTURE.md` §Recipe format schema v1.
4. Write the recipe parser + schema validator (TDD: write test recipes
   first, then the parser).

## After that

5. ACE-Step Python subprocess wrapper (`model-adapter/`).
6. Resumable HF model download.
7. Rust AM modulation DSP (`dsp/`).
8. Native macOS Swift app shell (`apple/`).
9. Background buffer queue.
10. Hand-seed ~20 starter recipes.

## Decisions to make in Phase 1 first commit

- Audio sample rate convention (44.1 kHz seems likely)
- Mono vs. stereo for v1 (mono is simpler; stereo is what users expect)
- IPC format with the Python subprocess (newline-delimited JSON over stdio
  is the leading candidate)
- Cargo workspace layout (single workspace vs. multiple)
