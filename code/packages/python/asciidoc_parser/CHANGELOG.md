# Changelog

## 0.1.0 — 2026-04-04

Initial release.

- Block parser: headings (h1–h6), paragraphs, thematic breaks, fenced code
  blocks (with `[source,lang]` attribute), literal blocks, passthrough blocks,
  quote blocks, unordered lists (nested), ordered lists (nested), line comments.
- Inline parser: strong (`*` / `**`), emphasis (`_` / `__`), code spans,
  links, images, cross-references, URL autolinks, hard breaks, soft breaks.
- Produces a `DocumentNode` conforming to the Document AST spec (TE00).
- 30+ unit tests with ≥ 80% coverage.
