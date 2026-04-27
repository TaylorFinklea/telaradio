# Lockstep — Project Continuity Doc

> If you just opened this repo and have one minute: skim **What Lockstep is** and **First 30 seconds** below, then read `ARCHITECTURE.md` (recipe format + module boundaries) and `ROADMAP.md` (Phase 1).

## What Lockstep is

An open-source focus music system implementing the amplitude-modulation
mechanism described in Woods et al. 2024 (*Communications Biology* 7:1376,
[doi:10.1038/s42003-024-07026-3](https://www.nature.com/articles/s42003-024-07026-3)).
Default modulation is 16 Hz beta-range, parametrically adjustable.

The architectural unlock is **recipes, not outputs**: a track is a small JSON
file `{prompt, seed, model_version, modulation_params}` that any client
deterministically regenerates locally. This collapses distribution, copyright
exposure, and storage cost simultaneously — the entire community library is a
git repo of ~200-byte files, and every "vote" is a vote on a reproducible
recipe rather than on copyrighted audio.

## Architectural invariants (do not re-litigate)

- **Recipes are canonical, audio is regenerated.** Never bundle or distribute
  generated audio as the artifact. If you find yourself adding an audio
  storage layer, stop and check this doc.
- **Model-agnostic generation.** A `Generator` trait in Rust abstracts the
  model. ACE-Step 1.5 XL is the v1 implementation; MusicGen / YuE / future
  Apache-licensed models must be droppable behind the same interface.
- **AGPL-3.0, with CLA.** The CLA preserves dual-licensing optionality.
- **Solo curator in Phase 1.** No multi-user, voting, or community features
  before Phase 2. Resist scope creep.
- **No medical or productivity claims.** Cite the paper, describe the
  mechanism, let users evaluate efficacy themselves. The bypass toggle in the
  advanced UI is a built-in placebo control for personal A/B testing.
- **No telemetry, no phone-home.** Lockstep does not call out except for
  first-launch model download and (Phase 2+) GitHub library sync.

## Tech stack at a glance

| Layer | Tech |
|-------|------|
| Generation backend | Rust (HTTP/gRPC server) |
| Model runtime | Python subprocess running ACE-Step 1.5 XL |
| DSP | Rust (amplitude modulation, future audio graph) |
| Web client | SvelteKit (Phase 2; library browsing, contribution UX) |
| Native client | Swift / SwiftUI on macOS (Phase 1) and iOS (Phase 2) |
| Recipe storage | Local filesystem + GitHub sync (recipes-as-PR) |
| Model storage | `~/Library/Application Support/Lockstep/models/` |

## Key files & their roles

- `ARCHITECTURE.md` — recipe schema, module boundaries, model abstraction,
  DSP stages, data flow.
- `ROADMAP.md` — phases 1–4. Phase 1 is concrete; phases 2–4 are sketched.
- `PHASE_0_REPORT.md` — historical record of session 0 decisions + rationale.
- `recipes/` — hand-seeded starter library lands here in Phase 1; this
  becomes the canonical local cache once GitHub sync is wired up.
- `.docs/ai/` — cross-session continuity per the global handoff convention
  (`current-state.md`, `next-steps.md`, `decisions.md`, `roadmap.md`).
- `LICENSE` — verbatim AGPL-3.0.
- `CLA.md` — stub; formal CLA process opens in Phase 2.

## Conventions

- **Recipe schema location**: defined in `ARCHITECTURE.md` §Recipe format.
  Schema version is a top-level string field; parsers must reject unknown
  versions explicitly.
- **Modulation defaults**: 16 Hz beta-range, depth 0.5, square envelope.
  Anything in the recipe overrides; missing fields use these defaults.
- **Adding a recipe**: write JSON to `recipes/<id>.json`, validate against
  the schema, add to the library index. In Phase 2 this becomes a PR flow.
- **Extending the model abstraction**: implement the `Generator` trait
  (`generate(prompt, seed, duration) -> WavBuffer`) and register the model
  id+version. Recipes pin a specific model id+version for reproducibility.
- **Audio graph extension point**: future stages (e.g., the Phase 2 nature
  layer) insert *after* modulation and *before* output. Do not modulate the
  nature layer — broadband sources lose information under AM.

## First 30 seconds (for a fresh Claude session)

1. Read this file end-to-end.
2. Open `.docs/ai/current-state.md` to find the live session pointer.
3. Skim `ARCHITECTURE.md` §Recipe format and §Module boundaries.
4. Open `ROADMAP.md`, find the current phase, look at its checklist.
5. Run `git log --oneline -5` to verify state matches the doc claims.
6. Ask the user what they want to work on next, or pick from
   `.docs/ai/next-steps.md`.

If a phase spec exists in `.docs/ai/phases/` without a matching report, the
previous session was mid-protocol — resume there instead of starting fresh.

## Out-of-scope reminders (defer, do not build speculatively)

- Voting / ratings / comments on recipes
- Multi-user accounts and shared backends with auth
- Cloud sync beyond the GitHub library repo
- GPU sharing / remote generation as a primary path
- Apple Watch heart-rate adaptation (worth stealing in Phase 3, not earlier)
- Session timers (piggyback on Apple Focus Mode integration in Phase 3)
- Nature soundscape integration (Phase 2 with concrete CC0 source plan;
  see `ROADMAP.md`)

If a session is pulling on one of these, stop and update the roadmap first.

## Conventions inherited from `~/CLAUDE.md`

- One Bash command per tool call unless genuinely piping.
- Small descriptive commits by default after code changes; do not push.
- Use `TaskCreate` / `TaskUpdate` for non-trivial work.
- AI handoff lives in `.docs/ai/`; update at session end.
