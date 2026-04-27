# Lockstep

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)

Open-source focus music. Recipes, not outputs.

## What it is

Lockstep generates music locally and applies amplitude modulation at
beta-range frequencies (~16 Hz by default) to support sustained attention
via neural phase-locking. The mechanism is described in Woods et al. 2024,
*Communications Biology* 7:1376
([doi:10.1038/s42003-024-07026-3](https://www.nature.com/articles/s42003-024-07026-3)).

A track in Lockstep is a small JSON file — a *recipe* — describing the
prompt, seed, model version, and modulation parameters. Clients regenerate
audio from recipes on demand. This means the entire community library is a
git repo of ~200-byte files, and contributions are PRs.

**Lockstep makes no productivity or medical claims.** It implements a
published mechanism and exposes a bypass toggle so you can A/B test the
effect for yourself.

## Status

Pre-Phase-1. Documentation and architecture are written; no runnable code
exists yet. See [`ROADMAP.md`](ROADMAP.md) for the plan.

## Getting involved

The project is solo-curated through Phase 1. Contribution opens in Phase 2,
when the web app, GitHub library repo, and PR-based recipe contribution
flow are ready.

If you want to follow along: star this repo and read
[`ARCHITECTURE.md`](ARCHITECTURE.md). Issues are open for bug reports and
proposals.

## License posture

AGPL-3.0, with a CLA. The CLA preserves dual-licensing optionality if the
project ever needs a different distribution arrangement; the default
posture is and will remain open. See [`LICENSE`](LICENSE) and
[`CLA.md`](CLA.md).

## Pointers

- [`CLAUDE.md`](CLAUDE.md) — project continuity doc (read first if you're
  picking the project up)
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — recipe schema, modules, DSP
- [`ROADMAP.md`](ROADMAP.md) — phases 1–4
- [`PHASE_0_REPORT.md`](PHASE_0_REPORT.md) — initial decisions and rationale

## References

- Woods et al. 2024, *Communications Biology* 7:1376 — mechanism
- [ACE-Step 1.5](https://github.com/ace-step/ACE-Step-1.5) — generation
  model (Apache 2.0)
- [SINE Isochronic Entrainer](https://isochronic.io/) — GPLv3 prior art
  for related FOSS brainwave entrainment, useful as DSP-design reference
