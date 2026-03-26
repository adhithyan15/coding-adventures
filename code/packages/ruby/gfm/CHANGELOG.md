# Changelog

All notable changes to `coding_adventures_gfm` are documented here.

## [0.1.0] - 2026-03-24

### Added

- Initial release.
- Thin pipeline package wiring `coding_adventures_gfm_parser` and
  `coding_adventures_document_ast_to_html` together.
- `CodingAdventures::Commonmark.parse(markdown)` — delegates to
  `CommonmarkParser.parse`.
- `CodingAdventures::Commonmark.to_html(doc, sanitize: false)` — delegates to
  `DocumentAstToHtml.to_html`.
- `CodingAdventures::Commonmark.parse_to_html(markdown, sanitize: false)` —
  one-call convenience: parse then render.
- 15 unit tests; 100% line and branch coverage.
- Spec: combines TE01 — GFM Parser and TE02 — Document AST to HTML.
