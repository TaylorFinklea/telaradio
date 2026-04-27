# Next steps

Phase 1 checklist lives in [`../../ROADMAP.md`](../../ROADMAP.md). Phase
1a (recipe core) and Phase 1b (Generator trait + mock subprocess) are
complete. The next decision point: **Phase 1b2 (real ACE-Step) or Phase 1c
(AM modulation DSP) first?** Both can proceed independently because the
mock generator unblocks downstream work.

## Recommended: Phase 1c — AM modulation DSP (next session)

The DSP can be built and ear-validated against the mock's 440 Hz sine
without needing the real model. Faster iteration loop than Phase 1b2.

1. Bootstrap the `dsp/` workspace member crate.
2. (TDD) Define `AmEnvelope` (Square / Sine / Triangle) and the AM
   transform `apply_am(buffer, rate_hz, depth, envelope) -> WavBuffer`
   per Woods et al. 2024 §Methods. Pure function; no allocation beyond
   the output buffer.
3. (TDD) Tests: at depth=0.0 the buffer is unchanged; at depth=1.0 +
   square envelope, 50% of samples are zero; rate-locked phase test.
4. CLI smoke binary `apply-modulation` that takes a recipe + WAV path
   and writes a modulated WAV. Useful for ear-validation.
5. Wire DSP into the model-adapter pipeline so a generated buffer can
   be modulated end-to-end.

## Alternative: Phase 1b2 — real ACE-Step (when ready)

1. Decide Python venv strategy: a `uv`-managed project under
   `model-adapter/python/` (current ad-hoc `uv run --with` becomes a
   real `pyproject.toml` with ACE-Step deps).
2. First-launch HF model download (resumable HTTP) into
   `~/Library/Application Support/Telaradio/models/`. Add an "use
   existing weights file" path for air-gapped installs.
3. New `AceStepGenerator` impl alongside `SubprocessGenerator` (or
   parameterize the engine inside the existing subprocess via a
   request field).
4. Mark e2e tests `#[ignore]` for the ACE-Step integration since they
   need ~10s and ~5 GB on disk.

## After 1c / 1b2

- Phase 1d — macOS Swift app shell
- Phase 1e — Background buffer queue
- Phase 1f — Hand-seed ~20 starter recipes
- Phase 1g — Settings UI (preset / 3-tier intensity / advanced)

## Decisions to make at the start of Phase 1c

- AM math precision: f32 (matches `WavBuffer.samples`) or f64 internally?
- Should the DSP own a separate `ModulationEnvelope` enum, or reuse
  `Recipe.modulation.envelope` directly? (Currently the recipe envelope
  is in `core::recipe::Envelope` and the DSP needs its own anyway since
  it'll grow rate-modulation patterns over time.)
