# Changelog — coding-adventures-asciidoc

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-04

### Added
- Initial release.
- `to_html(text)` — parses AsciiDoc and returns an HTML string.
- `parse(text)`   — parses AsciiDoc and returns the Document AST node.
- `render(text)`  — alias for `to_html`.
- Thin wrapper over `coding_adventures.asciidoc_parser` and
  `coding_adventures.document_ast_to_html`.
