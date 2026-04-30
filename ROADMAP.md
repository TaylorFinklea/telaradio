# Telaradio — Roadmap

Phases are not date-bound. Each phase ends with a `PHASE_N_REPORT.md` at the
project root capturing what shipped, what slipped, and what was learned.

## Phase 0 — Decision capture and scaffolding (this session)

Done. See `PHASE_0_REPORT.md`.

## Phase 1 — Minimum Viable Loop (solo)

Goal: a single user (Taylor) can play a recipe end-to-end on macOS and have
it sound good enough to use during a real work session.

- [x] Rust workspace bootstrap under `core/` (Cargo.toml, error types,
      recipe types matching `ARCHITECTURE.md` schema v1)
- [x] Recipe parser + schema validator
- [x] `Generator` trait + Python subprocess adapter
      (Phase 1b: mock-sine engine; Phase 1b2: real ACE-Step)
- [x] First-launch model download (resumable HTTP, HF Hub, into
      `~/Library/Application Support/Telaradio/models/`); also support
      pointing at a pre-existing weights file (Phase 1b2)
- [x] Rust AM modulation DSP per Woods et al. §Methods (square envelope,
      configurable rate + depth, default 16 Hz / 0.5)
- [x] Native macOS Swift app: minimal player UI (load recipe, play, pause,
      skip) — Phase 1d MVL (mock-only, hardcoded recipe) + Phase 1d2
      (real ACE-Step wiring + first-launch model setup sheet) shipped.
      Audible verification of the real-model path waits on the one-time
      sha256 bootstrap (see `.docs/ai/next-steps.md`).
- [ ] Background buffer queue: keep 2–3 tracks ahead generated and modulated
      during idle time; regenerate on prompt/seed change
- [ ] Hand-seeded starter library of ~20 recipes spanning lofi, ambient,
      electronic, and (text-prompt) nature-hybrid styles, committed to
      `recipes/`
- [ ] Settings UI: preset selector (Focus / Relax / Sleep), 3-tier intensity
      toggle, advanced panel with rate/depth/bypass
- [ ] `PHASE_1_REPORT.md` covering ear-eval against Brain.fm

Explicitly out of scope: iOS app polish, web UI, contribution flow, voting,
nature soundscape layer, watch HR adaptation, session timers.

## Phase 2 — Open contribution surface + nature layer

Goal: third parties can contribute recipes; iOS reaches parity; the audio
graph gains the nature layer.

- [ ] SvelteKit web app: browse the canonical GitHub library, preview
      recipe metadata, render simple metadata diffs
- [ ] GitHub PR contribution flow: recipe lint + JSON schema CI check on PRs
- [ ] iOS Swift app: parity with the macOS player
- [ ] Auth on the backend if it's shared (TBD: GitHub OAuth most likely)
- [ ] **Nature soundscape layer** added to the audio graph (mixed
      pre-output, excluded from AM):
  - Sources: freesound.org with the CC0 filter + sonniss GameAudioGDC packs.
    myNoise has no public API, so direct integration is not viable —
    use the asset libraries instead.
  - License audit at curation time (CC0 only, attribution recorded for
    courtesy in `recipes/NATURE_SOURCES.md`).
  - Assets shipped with the backend, not the iOS app, to keep the mobile
    binary small.
  - Recipe schema gains optional `nature_layer: { source_id, volume }`.
  - Settings allow per-session override volume.
- [ ] `PHASE_2_REPORT.md`

## Phase 3 — Adaptive listening

Goal: Telaradio responds to the listener's state, not just the recipe.

- [ ] Apple Watch heart-rate adaptation (Endel-style: gently nudge
      generation parameters as HR changes)
- [ ] Apple Focus Mode integration + session timers
- [ ] User feedback signal capture (skip rate, replay rate, dwell time) →
      simple recipe ranking
- [ ] `PHASE_3_REPORT.md`

## Phase 4 — Community + scale

Goal: the project becomes load-bearing for people who aren't Taylor.

- [ ] Voting layer on recipes — design TBD. Two candidates: lean on PR
      reactions + a derived ranking, or stand up a small voting service
      (still AGPL, still self-hostable).
- [ ] GPU sharing / remote generation as a fallback for low-spec hardware
- [ ] Model variant marketplace (MusicGen, YuE, future Apache-licensed
      entrants); recipes choose the model at authoring time
- [ ] `PHASE_4_REPORT.md`

## Backlog

<!-- tier3_owner: claude -->

Tiered tech-debt and small-improvement items land here as the project grows.
Empty at end of Phase 0 — populate during Phase 1 audits.

### Haiku tier

_(none yet)_

### Sonnet tier

_(none yet)_

### Opus tier

_(none yet)_
