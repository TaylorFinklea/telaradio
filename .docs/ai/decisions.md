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
- Mono vs. stereo, sample rate convention — Phase 1b
- Recipe PR lint rules — Phase 2 when CI exists

## 2026-04-27 — Phase 1a recipe-core decisions

1. **GitHub repo**: `TaylorFinklea/lockstep` created public on
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
