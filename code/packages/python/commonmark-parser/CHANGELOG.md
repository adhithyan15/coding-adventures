# Changelog — coding-adventures-commonmark-parser

## [0.1.0] — 2026-03-24

### Added

- Initial Python port of the TypeScript `@coding-adventures/commonmark-parser` package.
- Two-phase CommonMark 0.31.2 parsing:
  - Phase 1 (block_parser.py): Document structure — headings, lists, blockquotes, code blocks, HTML blocks, thematic breaks, link reference definitions.
  - Phase 2 (inline_parser.py): Inline content — emphasis, strong, links, images, code spans, autolinks, raw HTML, backslash escapes, entity decoding.
- Full HTML entity decoding using Python's `html.entities.html5` table (~2200 named entities).
- Scanner utility (scanner.py) with character classification functions for Unicode whitespace and punctuation.
- Entities module (entities.py) for HTML entity decoding and `escape_html()`.
- All 652 CommonMark 0.31.2 spec examples pass (100% compliance).
- 94% overall test coverage.
