# Changelog

All notable changes to `coding-adventures-wordprocessingml` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-07-03

### Added

- Initial release: milestone **W3** of the OOXML effort (spec `WML01`).
- `open_docx(bytes: &[u8]) -> Result<Document, DocxError>` — reads a `.docx`
  from its bytes on top of the `opc` + `xml-parser` layers.
- Document model: `Document` → `Block` (`Paragraph` | `Table`) →
  `Paragraph { text, runs }` / `Run { text }` and
  `Table { rows }` → `Row` → `Cell { text, paragraphs }`.
- Paragraph text is built by walking `<w:r>` runs explicitly (not
  `text_content()`), concatenating each run's text; `<w:tab/>` → `\t` and
  `<w:br/>` → `\n` are folded in, and `xml:space="preserve"` text survives.
- `Document::text()` for whole-document plain-text extraction, plus
  `Document::paragraphs()` and `Document::tables()` iterators.
- `DocxError` wrapping OPC errors, a missing document part, non-UTF-8 parts, and
  malformed XML.
- Full spec (`code/specs/WML01-wordprocessingml.md`), README, and an
  end-to-end test over a real DEFLATE-compressed `.docx` fixture.
