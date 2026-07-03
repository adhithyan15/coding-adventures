# Changelog

All notable changes to `coding-adventures-presentationml` are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/); the project
adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-07-03

### Added

Initial release — milestone **PML01** (`code/specs/PML01-presentationml.md`).

- `open_pptx(&[u8]) -> Result<Presentation, PptxError>`: open a `.pptx` from its
  raw bytes and read it into an ordered `Presentation → Slide → Shape` model.
- `Presentation` with `slides()` and `slide_count()`; slides are returned in
  `<p:sldIdLst>` (show) order.
- `Slide` with `shapes()`, `shape_count()`, and `text()` (all shape text joined
  by `\n`, empty shapes skipped).
- `Shape { text }`: one shape's DrawingML text runs concatenated.
- `PptxError` covering `Opc`, `MissingPresentation`, `NotUtf8`, `MalformedXml`,
  and `MissingSlidePart`, with `Display`, `std::error::Error`, and
  `From<OpcError>`.

### Notes

- Built on `coding-adventures-opc` (OPC01) and `coding-adventures-xml-parser`
  (XML01); the same architecture as `coding-adventures-spreadsheetml`.
- Resolves the two `.pptx` indirections: `r:id → slide part` (via OPC
  relationships) and the **DrawingML** (`a:`) namespace where slide text lives,
  distinct from the PresentationML (`p:`) namespace of the slide structure.
- Speaker notes, tables, grouped shapes, and layout/master-inherited
  placeholders are out of scope for this milestone.

### Tests

- End-to-end test over a real DEFLATE-compressed two-slide fixture asserting
  slide count, order, and per-slide text.
- Unit tests for every error variant, the DrawingML namespace switch, run/
  paragraph joining, and empty/decorative shapes. Coverage well exceeds 80%.
