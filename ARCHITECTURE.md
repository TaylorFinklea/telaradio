# Telaradio — Architecture

This document specifies the system's structure. Decisions captured here are
load-bearing; changes should land alongside an entry in `.docs/ai/decisions.md`
and a phase report.

## Recipe format

A recipe is a small JSON document — typically ~200 bytes — that
deterministically describes one track. Recipes are the canonical artifact;
audio is regenerated from them. Schema version is explicit so parsers can
reject unknown versions cleanly.

### Schema (v1)

```json
{
  "schema_version": "1",
  "id": "5b4f2a8c-9e3d-4f17-b2a1-7c0c1f3e8d92",
  "title": "Foggy lofi for deep work",
  "tags": ["lofi", "focus", "morning"],
  "prompt": "warm vinyl lofi, jazzy keys, slow tempo, no vocals",
  "seed": 1893421,
  "model": {
    "id": "ace-step-1.5-xl",
    "version": "1.5.0"
  },
  "duration_seconds": 240,
  "modulation": {
    "rate_hz": 16,
    "depth": 0.5,
    "envelope": "square"
  },
  "created_at": "2026-04-26T15:00:00Z",
  "author": "tfinklea"
}
```

### Field reference

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `schema_version` | string | yes | Currently `"1"`. Parsers reject unknown values. |
| `id` | UUID v4 string | yes | Stable identifier. Filename is `<id>.json`. |
| `title` | string | yes | Human-readable. Not used for generation. |
| `tags` | string[] | yes | Free-form. Used for library browsing/filtering. |
| `prompt` | string | yes | Passed to the model. Determinism depends on prompt + seed + model id+version. |
| `seed` | integer | yes | Generator-input seed. |
| `model.id` | string | yes | e.g. `ace-step-1.5-xl`. Recipes pin a specific implementation. |
| `model.version` | string | yes | Semver. Pin so audio reproduces. |
| `duration_seconds` | integer | yes | Generated track length. |
| `modulation.rate_hz` | number | yes | AM rate. Default 16. Allowed: any positive number; UI exposes 8/12/16/20 in advanced mode. |
| `modulation.depth` | number | yes | 0–1. Default 0.5. |
| `modulation.envelope` | string | yes | `"square"` | `"sine"` | `"triangle"`. v1 ships `"square"` per Woods et al. |
| `created_at` | ISO-8601 string | yes | Authoring time. |
| `author` | string | yes | GitHub username, or `"anonymous"`. |

### Defaults

If a parser encounters a recipe missing optional modulation fields (during
Phase 2+ schema migrations), it applies: `rate_hz=16`, `depth=0.5`,
`envelope="square"`. v1 schema requires all three explicitly.

### Future fields (Phase 2)

```json
{
  "nature_layer": {
    "source_id": "freesound-12345",
    "volume": 0.3
  }
}
```

Optional. Excluded from amplitude modulation (mixed post-AM, pre-output).
Sources resolve to local CC0 assets — see `ROADMAP.md` Phase 2.

## Module boundaries

```
core/            Rust workspace root (recipe types, error types, traits)
dsp/             Rust crate: amplitude modulation, audio graph
model-adapter/   Rust crate: Generator trait + ACE-Step subprocess bridge
library/         Rust crate: recipe filesystem I/O + GitHub sync (Phase 2)
web/             SvelteKit app (Phase 2)
apple/           Swift package: macOS app (Phase 1) + iOS app (Phase 2)
recipes/         Hand-seeded starter library (read-only Phase 1)
```

The Rust crates form one workspace under `core/`. The Swift package is
independent and talks to the Rust backend over HTTP/gRPC. The web app is
also independent and uses the same backend API.

## Platform choice rationale

Hybrid: Rust backend + SvelteKit web + native Swift iOS/macOS clients.

- **Why a backend at all**: ACE-Step is Python-native and 4–6 GB of weights;
  embedding it in a Swift or web client is unrealistic. A backend isolates
  the model runtime and gives both frontends one place to call.
- **Why Rust**: predictable real-time performance for the modulation DSP,
  zero-cost FFI to a Python subprocess, single static binary for ops.
- **Why SwiftUI for the listening surface**: focus music is mobile-first.
  AVFoundation handles background audio, lock-screen controls, and Now
  Playing properly; web audio in iOS Safari does not.
- **Why SvelteKit for web**: the contribution and library-browsing UX is the
  one place a web app actually beats a native app — easy linking, easy
  contributing, easy preview without install. SvelteKit also pairs well with
  the AGPL/open-source contributor audience.

## Model abstraction interface

Every generator implements:

```rust
pub trait Generator {
    fn id(&self) -> &str;            // e.g. "ace-step-1.5-xl"
    fn version(&self) -> &str;       // e.g. "1.5.0"
    fn generate(
        &self,
        prompt: &str,
        seed: u64,
        duration_seconds: u32,
    ) -> Result<WavBuffer, GeneratorError>;
}
```

`WavBuffer` is a single contiguous PCM buffer (float32, 44.1 kHz mono or
stereo — TBD in Phase 1 first commit). Recipes pin `model.id` + `model.version`,
so adding a generator never silently changes existing recipes.

The v1 ACE-Step adapter spawns a long-lived Python subprocess and speaks to
it over stdio (newline-delimited JSON request/response, audio via a temp
file path returned in the response).

## Modulation DSP stages

```
WavBuffer  →  AM(rate_hz, depth, envelope)  →  WavBuffer
```

Pseudocode for the square-wave AM (Woods et al. §Methods):

```
for sample i in input:
    phase = (i / sample_rate) * rate_hz
    gate  = 1.0 if (phase mod 1.0) < 0.5 else (1.0 - depth)
    output[i] = input[i] * gate
```

Sine and triangle envelopes substitute the gate calculation; depth
parameterizes the trough amplitude, not the peak (peak stays at 1.0).

The DSP stage is pure: no side effects, no allocation beyond the output
buffer. This is what makes it cheap enough to apply per-track without
caching modulated outputs.

## Data flow

```
recipe.json
    │
    ▼
┌─────────────┐    ┌──────────────┐    ┌────────────┐    ┌──────────┐
│ recipe-load │ →  │ model-adapt  │ →  │ AM modulate│ →  │ playback │
│ (Rust)      │    │ (Py via IPC) │    │ (Rust)     │    │ (native) │
└─────────────┘    └──────────────┘    └────────────┘    └──────────┘
                          ▲
                     model cache
                ~/.../Telaradio/models/

[Future Phase 2 graph]

recipe.json
    │
    ▼
┌─────────────┐    ┌──────────────┐    ┌────────────┐    ┌────────┐    ┌──────────┐
│ recipe-load │ →  │ model-adapt  │ →  │ AM modulate│ →  │  mix   │ →  │ playback │
└─────────────┘    └──────────────┘    └────────────┘    │  ▲     │    └──────────┘
                                                         │  │     │
                                       nature-layer ─────┘        │
                                       (CC0 asset, no AM)        │
                                                                  │
                                                       volume from recipe
                                                       or session override
```

## Privacy and telemetry

Telaradio does not phone home.

- First-launch model download is the only mandatory outbound call (HF Hub).
  A "use existing model file" path is offered for fully air-gapped installs.
- Phase 2 GitHub library sync is opt-in and uses the public GitHub API.
- No analytics, no crash reporting service, no usage pings. If we ever add
  any of these, they ship as opt-in with a separate decision in
  `.docs/ai/decisions.md` and a release note.
