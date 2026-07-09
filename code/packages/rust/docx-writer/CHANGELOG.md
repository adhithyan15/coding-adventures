# Changelog

All notable changes to `coding-adventures-docx-writer` are documented here. The
format follows [Keep a Changelog](https://keepachangelog.com/), and this project
adheres to [Semantic Versioning](https://semver.org/).

## [0.2.0] — 2026-07-08

### Added

- **Run formatting.** A new `Run { text, bold, italic, mono }` type with chainable
  builders (`Run::plain("x").bold().italic()`). Bold → `<w:b/>`, italic →
  `<w:i/>`, monospace → a Consolas `<w:rFonts>` — all *direct* formatting that
  renders in Word with no styles part. A flag-free run emits no `<w:rPr>`.
- **Paragraph styles.** A `ParagraphStyle` enum (`Normal`, `Heading(1..=6)`,
  `Code`, `Quote`, `List`) and `Document::add_styled_paragraph(style, Vec<Run>)`.
  A non-`Normal` style emits `<w:pPr><w:pStyle w:val="…"/></w:pPr>` and causes
  `write_docx` to add a minimal `word/styles.xml` (wired via
  `word/_rels/document.xml.rels`) defining `Heading1`…`Heading6` (bold, sized,
  with `<w:outlineLvl>` so Word's outline works), `Code`, `Quote`, and
  `ListParagraph`. A document that uses no styles has no `styles.xml` at all.
- These additions back the Markdown → `.docx` pipeline (see
  `code/specs/MD02-markdown-to-docx.md`).

### Unchanged

- The existing text-only API (`new`, `add_paragraph`, `add_paragraph_runs`,
  `add_table`) is byte-for-byte compatible — an all-`Normal`, unformatted
  document serializes exactly as before (no `<w:rPr>`, no `<w:pPr>`, no
  `styles.xml`). All prior round-trip tests pass untouched.

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
