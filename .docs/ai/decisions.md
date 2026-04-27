# Decision log

Append-only record of non-obvious design, tooling, or scope decisions.
Each entry: date, decision, rationale, what it supersedes (if anything).

## 2026-04-26 — Phase 0 foundational decisions

See [`../../PHASE_0_REPORT.md`](../../PHASE_0_REPORT.md) for the full
record. Summary:

1. Codename: **Lockstep**
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
- Mono vs. stereo, sample rate convention — Phase 1 first commit
- Recipe PR lint rules — Phase 2 when CI exists
