# web/

SvelteKit web client. Phase 2.

Responsibilities:
- Browse the canonical GitHub library
- Preview recipe metadata
- Render simple metadata diffs for PRs
- Drive the contribution UX

The web client is read-mostly: it reads from the GitHub library repo and
the local backend, and writes by opening GitHub PRs (no direct write to
the library).
