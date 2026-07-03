# Changelog

All notable changes to `coding-adventures-pptx-writer` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-07-03

### Added

- Initial release — milestone **C3** of the OOXML effort.
- `Presentation` / `Slide` model with `Presentation::new`,
  `Presentation::add_slide`, and `Slide::add_text`.
- `write_pptx(&Presentation) -> Vec<u8>` serializes a deck to valid `.pptx`
  bytes, packaged via `opc-writer`.
- Emits the full PresentationML scaffold required by strict consumers
  (PowerPoint, python-pptx): `presentation.xml`, one `slideN.xml` per slide, a
  shared `slideMaster1.xml`, `slideLayout1.xml`, and `theme1.xml`, all wired by
  the correct relationship graph, plus `[Content_Types].xml` and every `.rels`.
- Slide text is written in the DrawingML (`a:`) namespace inside a shape's
  `p:txBody`; all user text is escaped via `opc_writer::xml_escape`.
- Handles edge cases without panicking: empty deck (empty `<p:sldIdLst/>` + full
  scaffold), empty slide, arbitrary Unicode, and XML-special / illegal-control
  characters.
- `#![forbid(unsafe_code)]`.
- Structural test suite: unzip via `zip::ZipReader`, parse slide parts via
  `coding_adventures_xml_parser::parse_xml`, verify member presence, per-slide
  text placement and ordering, escaping, id/rels alignment, and well-formedness
  of every part.
- Spec: `code/specs/PPTXW01-pptx-writer.md`.
