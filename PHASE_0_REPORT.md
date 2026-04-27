# Phase 0 Report — Decision Capture and Scaffolding

**Date**: 2026-04-26
**Outcome**: Seven foundational decisions captured; documentation and
directory scaffold written; no implementation code.

## Decisions made

### 1. Codename: Lockstep

Direct reference to neural phase-locking — the mechanism in plain English.
Distinctive (compare "Cadence", which collides heavily in fintech / dev-tool
naming), single word, trademark-clean.

### 2. Platform strategy v1: Hybrid

Rust backend (HTTP/gRPC) wrapping a Python ACE-Step subprocess and a Rust
modulation DSP. Two frontends: SvelteKit web app (curation, library
browsing, contribution UX) and native Swift iOS+macOS apps (listening
surface).

**Rationale**: ACE-Step is Python-native and ~5 GB of weights; isolating
that in a backend lets each frontend stay native to its surface. Web is
where contribution lives; Swift native is where listening lives. Largest
v1 scope of the four options, but matches the "techy users want a native
phone app pointed at their own backend" intuition.

**Tradeoff accepted**: two UI codebases. Justified by listening-UX quality
mattering more than implementation cost for this product.

### 3. Recipe storage and distribution v1: Local + GitHub sync

Recipes live at `~/Library/Application Support/Lockstep/recipes/` (and
platform equivalents). Canonical community library is a public GitHub repo;
contributions are PRs; git history is the audit trail.

**Rationale**: $0 hosting, $0 ops, AGPL-aligned. Recipes are tiny JSON, so
git is genuinely sufficient as the database. Phase 2 voting UX will need to
be designed against PR-comment / reaction primitives or layered on a
separate service — accepted as a Phase 2 design problem rather than a v1
infrastructure burden.

### 4. Modulation UX exposure: All three modes, switchable

- Default (casual): preset selector — Focus / Relax / Sleep
- Setting "Show neural intensity": three-tier Low/Med/High depth slider
- Setting "Advanced (developer)": rate selector (8/12/16/20 Hz), continuous
  depth slider, bypass toggle (also a built-in placebo control)

**Rationale**: Brain.fm hides parameters because it's a closed product;
hiding them in an open-source project where the source is right there is
theatrical. Layering casual → intensity → developer respects the audience
spread without being dishonest.

### 5. Pre-generation strategy: Background buffer

Maintain 2–3 tracks ahead, generated during idle time. First play has ~10s
cold-start latency; subsequent skips are instant. Buffer regenerates on
prompt/seed change. Cache size bounded ~3 × ~6 MB per active queue.

**Rationale**: Streaming services already do this for gapless / skip-ahead;
the same buffer absorbs ACE-Step's generation latency without surfacing a
config knob users won't touch.

### 6. Nature soundscape integration: Out of scope for v1, in for Phase 2

v1 pipeline is `Recipe → ACE-Step → modulation → output`. Architecture
leaves a clean insertion point for the nature layer (mixed pre-output,
excluded from AM).

Phase 2 source plan: freesound.org with the CC0 filter + sonniss
GameAudioGDC packs. myNoise has no public API, so direct integration is
not viable — bundled assets instead.

**Rationale**: keeping v1 scope tight matters more than feature breadth at
the start. The architecture is ready for the layer.

### 7. ACE-Step model distribution: First-launch download

Backend installer ~50 MB; first run prompts user to download ACE-Step 1.5
XL (~5 GB) from Hugging Face into `~/Library/Application Support/Lockstep/
models/`. Resumable HTTP. After first run, fully offline. Architecture
supports model variant selection even though v1 ships XL only.

**Rationale**: bundling 5 GB is hostile to App Store distribution and
balloons the install. HF Hub at runtime breaks the offline-focus promise.
First-launch download is the standard pattern for ML-bundled apps.

## Decisions deferred

- Voting design (PR reactions vs. dedicated service) — Phase 4
- Auth scheme for the shared backend — Phase 2 when the web app starts
  hitting it
- Mono vs. stereo output, sample rate convention — Phase 1 first commit
- Specific lint rules for recipe PRs — Phase 2 when CI exists

## What we know about ACE-Step 1.5 XL

- Apache 2.0 license — compatible with AGPL distribution.
- Apple Silicon support; <4 GB VRAM (M-series unified memory equivalent).
- ~10s generation time per 4-minute track on consumer hardware.
- 4B DiT decoder variant for higher audio quality vs. base.
- Repo: https://github.com/ace-step/ACE-Step-1.5
- Weights distributed via Hugging Face.

Risk: ACE-Step is a young project; upstream stability isn't guaranteed.
The model abstraction layer specifically exists to make migration to a
successor (MusicGen, YuE, ...) cheap.

## Mechanism reference

Woods et al. 2024, "Rapid modulation in music supports attention in
listeners with attentional difficulties," *Communications Biology* 7:1376.
[doi:10.1038/s42003-024-07026-3](https://www.nature.com/articles/s42003-024-07026-3).

Read the §Methods section. Fig. 1 shows the modulation spectrum
manipulation. Default 16 Hz beta-range comes directly from this paper. The
amplitude-modulation math in `ARCHITECTURE.md` §Modulation DSP stages
matches the paper's manipulation.

## Open risks

- **ACE-Step upstream stability**: small project, single-vendor dependency.
  Mitigated by the model-abstraction layer.
- **Apple Silicon ML drift**: PyTorch / MPS support changes between Sonoma
  / Sequoia / Tahoe; we may need to track upstream patches.
- **Hugging Face availability**: first-launch download depends on HF being
  reachable. Air-gapped install path mitigates but adds support surface.
- **Schema migration cost**: pinning model id+version in recipes means
  every model upgrade is a recipe re-author. Acceptable in Phase 1; in
  Phase 2 we'll want a migration tool.
- **Brain.fm overlap**: not a competitive risk (different distribution
  posture entirely), but Taylor's continued Brain.fm subscription is the
  ear-calibration baseline — losing access would weaken Phase 1 verification.

## Deliverables produced this session

- `CLAUDE.md`, `ARCHITECTURE.md`, `ROADMAP.md`, `README.md`, this report
- `LICENSE` (verbatim AGPL-3.0)
- `CLA.md` stub
- `.github/` (CONTRIBUTING.md, PR template, three issue templates)
- `.docs/ai/` seeded (current-state, next-steps, decisions, roadmap pointer)
- Module directory scaffold (`core/`, `dsp/`, `model-adapter/`, `library/`,
  `recipes/`, `web/`, `apple/`) each with a README placeholder

No source files (.rs / .py / .ts / .swift) were written.
