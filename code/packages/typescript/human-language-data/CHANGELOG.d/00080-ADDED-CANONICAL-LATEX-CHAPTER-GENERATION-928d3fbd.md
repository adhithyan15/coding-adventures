### Added — canonical LaTeX chapter generation

- Added deterministic lesson-AST fingerprints and a pure Markdown-block to
  LaTeX renderer, now covering all 24 Spanish Chapter 1–3 schema-v2 lessons.
- Preserved nested inline emphasis, wrapped long practice lists, and emitted
  text-safe short titles for running headers and PDF bookmarks.
- Added write/check CLI modes, a committed chapter-hash manifest, path-safety
  validation, and a unified-book CI drift gate.
- Exposed each parsed lesson's source hash so book and app consumers can verify
  that they loaded the same canonical content.

