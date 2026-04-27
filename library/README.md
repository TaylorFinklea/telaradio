# library/

Rust crate. Recipe filesystem I/O + GitHub sync.

Phase 1 responsibilities:
- Read/write recipe JSON files in the local recipe directory
  (`~/Library/Application Support/Telaradio/recipes/` on macOS)
- Index by tag, search by free-text on title

Phase 2 responsibilities:
- Sync (read-only at first) from the canonical Telaradio library repo on
  GitHub
- Apply recipe schema migrations as the schema evolves
