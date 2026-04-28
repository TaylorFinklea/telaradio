---
name: Recipe proposal
about: Propose a new recipe for the community library
labels: recipe-proposal
---

> Recipe contributions move to a PR-based flow in Phase 2. For now, please
> propose recipes via this issue and the maintainer will hand-merge promising
> ones into the starter library.

## Recipe metadata

- **Title**:
- **Tags** (comma-separated):
- **Author** (GitHub handle or "anonymous"):

## Recipe JSON

<!-- Paste the recipe JSON here. Schema is in ARCHITECTURE.md §Recipe format. -->

```json
{
  "schema_version": "1",
  "id": "...",
  "title": "...",
  "tags": [],
  "prompt": "...",
  "seed": 0,
  "model": { "id": "ace-step-v1-3.5b", "version": "1.0.0" },
  "duration_seconds": 240,
  "modulation": { "rate_hz": 16, "depth": 0.5, "envelope": "square" },
  "created_at": "...",
  "author": "..."
}
```

## Why this recipe

<!-- What kind of work / mental state is this for? What did you ear-test it against? -->
