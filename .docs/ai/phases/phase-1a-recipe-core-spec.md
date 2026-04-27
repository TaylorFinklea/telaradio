# Phase Spec: Phase 1a — Recipe Core

**Roadmap item:** Phase 1, items 1–2 (Rust workspace bootstrap; recipe parser + schema validator)
**Date:** 2026-04-27

## Product

**Goal:** A working Rust crate that parses, validates, and round-trips
recipe JSON files according to `ARCHITECTURE.md` schema v1, with
comprehensive tests. After this slice, anyone with `cargo` can validate a
recipe against the published schema without involving ACE-Step, audio,
or any other moving part.

### Acceptance criteria

- [ ] `cargo test` passes from project root
- [ ] Parser accepts a valid recipe JSON and returns a `Recipe` struct
- [ ] Parser rejects invalid recipes with clear, typed error messages
      (missing required field, wrong type, unknown `schema_version`,
      unknown envelope variant)
- [ ] Round-trip: `parse(serialize(recipe)) == recipe` for any valid recipe
- [ ] Struct fields exactly match the v1 schema in `ARCHITECTURE.md` (no
      drift, no extras)
- [ ] Project is a git repo; first commit is the Phase 0 scaffold; second
      commit is this Phase 1a work
- [ ] Tests cover, at minimum: a known-good recipe, every envelope
      variant, missing required field, wrong type, unknown
      `schema_version`, depth-out-of-range
- [ ] One realistic example recipe JSON committed under `recipes/` so
      future sessions and external readers see what a recipe looks like

### Assumptions

- Schema follows `ARCHITECTURE.md` §Recipe format v1 exactly
- `serde` + `serde_json` for (de)serialization (idiomatic Rust)
- `thiserror` for error types
- `envelope` is a Rust enum (`Square`, `Sine`, `Triangle`) serialized as
  lowercase strings
- All modulation fields are required in v1 (defaults documented in the
  doc but not auto-filled by the parser; the parser is strict)
- `created_at` is parsed as a `chrono::DateTime<Utc>` so ISO-8601
  validity is checked at parse time

### Out of scope (deferred to later Phase 1 sub-slices)

- ACE-Step Python subprocess wrapper — **Phase 1b**
- HF model download — **Phase 1b**
- Amplitude modulation DSP — **Phase 1c**
- macOS Swift app — **Phase 1d**
- Background buffer queue — **Phase 1e**
- Hand-seeded ~20 recipe library — **Phase 1f**
- Settings UI — **Phase 1g**
- GitHub library sync (read-only or otherwise) — **Phase 2**

### Open questions (resolved 2026-04-27)

1. **GitHub hosting** — *Create now (public).* Repo will be
   `tfinklea/telaradio`. First push happens at end of this phase.
2. **UUID handling** — *Strict `Uuid` via the `uuid` crate.* Malformed
   IDs fail at parse time.
3. **Cargo workspace layout** — *Single root workspace.* Each module is
   a workspace member crate; shared dep versions; `cargo test` from root.
4. **Recipe schema strictness** — *Strict: `#[serde(deny_unknown_fields)]`.*
   Unknown JSON fields are a parse error. Forward-compat requires explicit
   `schema_version` bumps. Catches typos loudly.

---

## Technical approach

### Scope

- Create:
  - `Cargo.toml` (workspace root)
  - `core/Cargo.toml` and `core/src/lib.rs` (recipe types crate)
  - `core/src/recipe.rs` — `Recipe` struct + `Modulation` + `ModelRef` + enums
  - `core/src/error.rs` — `RecipeError` thiserror enum
  - `core/tests/recipe_parse.rs` — integration tests
  - `core/tests/fixtures/` — known-good and known-bad recipe JSON
  - `recipes/example-foggy-lofi.json` — one real example recipe
  - `.gitignore` — Rust + macOS + IDE noise
- Modify:
  - `core/README.md` — append "what's implemented now" section
  - `.docs/ai/current-state.md` — session summary at end
  - `.docs/ai/next-steps.md` — remove completed items, add Phase 1b
- Delete: nothing

### Steps

1. `git init` in project root, commit Phase 0 scaffold as the first
   commit (`feat: phase 0 scaffold and decisions`).
2. Write `.gitignore` (Rust target/, .DS_Store, .idea/, etc.).
3. Create root `Cargo.toml` declaring `core` as a workspace member.
4. Create `core/Cargo.toml` with deps: `serde`, `serde_json`, `thiserror`,
   `chrono`, `uuid` (assuming Q2 = strict UUID).
5. Define types in `core/src/recipe.rs`:
   - `Envelope` enum with serde rename_all = "lowercase"
   - `Modulation { rate_hz: f64, depth: f64, envelope: Envelope }`
   - `ModelRef { id: String, version: String }`
   - `Recipe { schema_version: String, id: Uuid, title, tags,
     prompt, seed: u64, model: ModelRef, duration_seconds: u32,
     modulation: Modulation, created_at: DateTime<Utc>, author: String }`
6. Add `Recipe::parse(&str) -> Result<Recipe, RecipeError>` and
   `Recipe::serialize(&self) -> Result<String, RecipeError>`. Inside
   `parse`, explicitly check `schema_version == "1"` and depth in `[0, 1]`.
7. Define `RecipeError` with thiserror covering all rejection cases.
8. Write fixtures: `valid_minimal.json`, `valid_full.json`,
   `bad_schema_version.json`, `bad_envelope.json`, `bad_depth.json`,
   `missing_seed.json`, `wrong_type_seed.json`.
9. Write integration tests asserting accept/reject behavior + round-trip.
10. Generate one realistic example recipe at
    `recipes/example-foggy-lofi.json` (UUID, real prompt, sensible defaults).
11. `cargo test` — fix until green.
12. Append "what's implemented now" to `core/README.md`.
13. Update `.docs/ai/current-state.md` and `.docs/ai/next-steps.md`.
14. Write `.docs/ai/phases/phase-1a-recipe-core-report.md`.
15. Commit Phase 1a as `feat(core): recipe types, parser, schema validator`.

### Verification

- `cargo test` — all tests green
- `cargo clippy --all-targets -- -D warnings` — no clippy warnings
- `cargo fmt --check` — formatted
- Manual: parse `recipes/example-foggy-lofi.json` from a `cargo run`
  smoke-test binary or `cargo test -- --ignored` round-trip test
