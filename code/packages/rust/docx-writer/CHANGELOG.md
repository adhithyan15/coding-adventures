# Changelog

All notable changes to `coding-adventures-docx-writer` are documented here. The
format follows [Keep a Changelog](https://keepachangelog.com/), and this project
adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-07-03

### Added

- Initial release — milestone **C2** of the OOXML effort.
- `Document` model with `new`, `add_paragraph` (single-run), `add_paragraph_runs`
  (multi-run), and `add_table` (rows of cell text).
- `write_docx(&Document) -> Vec<u8>`: serializes the model to WordprocessingML
  `word/document.xml`, then packages it — `[Content_Types].xml`, `_rels/.rels`
  with the `/officeDocument` relationship, and the document part — into a valid
  `.docx` via the generic `coding-adventures-opc-writer` layer.
- `xml:space="preserve"` on every `<w:t>` so significant whitespace survives; all
  user text routed through `opc-writer`'s `xml_escape` (five specials escaped,
  XML-illegal control characters dropped).
- `#![forbid(unsafe_code)]`; no `unwrap`/`expect`/`panic!` on model-driven paths.
- Round-trip test proving the output reopens through
  `coding-adventures-wordprocessingml` with paragraphs, multi-run concatenation,
  and table cells intact; unit tests for XML escaping, Unicode, empty document,
  empty table, whitespace preservation, and paragraph ordering.
- Spec: `code/specs/DOCXW01-docx-writer.md`.
