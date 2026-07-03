# Changelog

All notable changes to the `ppt` crate are documented here. Format loosely
follows [Keep a Changelog](https://keepachangelog.com/); this crate uses
semantic versioning.

## [0.1.0] — 2026-07-03

Initial release — milestone **PPT01**: a zero-third-party-dependency reader for
legacy `.ppt` (PowerPoint 97–2003 binary, [MS-PPT]) presentations, layered on
the `cfb` compound-file reader.

### Added

- `open_ppt(&[u8]) -> Result<Presentation, PptError>` — open `.ppt` bytes,
  read the `"PowerPoint Document"` stream via `cfb`, and extract per-slide text.
- `parse_document_stream(&[u8]) -> Result<Presentation, PptError>` — parse a raw
  record stream directly (test/tooling entry point that bypasses the CFB layer).
- `Presentation` (`slides()`, `slide_count()`) and `Slide` (`text()`,
  `text_runs()`) typed model.
- `PptError` (`Cfb`, `NoDocumentStream`, `Truncated`) with `Display`, `Error`
  (with `source()`), and `From<cfb::CfbError>`.
- [MS-PPT] record walker: 8-byte RecordHeader parsing, container-vs-atom via
  `recVer == 0xF`, recursion through Document (`0x03E8`) and arbitrary
  containers, one `Slide` per Slide container (`0x03EE`), and decoding of
  TextBytesAtom (`0x0FA8`, Latin-1) and TextCharsAtom (`0x0FA0`, UTF-16LE)
  atoms, each NUL-stripped.

### Security

- `#![forbid(unsafe_code)]`; no `unwrap`/`expect`/`panic!`; all reads
  bounds-checked; arithmetic via `checked_add`.
- Recursion-depth cap (64) prevents stack-overflow DoS from deeply nested
  containers.
- Slide count and total-text caps prevent unbounded allocation from hostile
  files; no pre-sizing to file-declared lengths.
- Clean stop on the CFB's trailing zero padding and on any malformed / over-long
  `recLen`; the cursor always advances, so no hang is possible.

### Tests

- Required end-to-end test against the real CFB-wrapped fixture (2 slides,
  correct text, ordering guard).
- Isolated TextBytes (Latin-1, incl. `é`) and TextChars (UTF-16, incl. `Ω`,
  unpaired-surrogate → U+FFFD, odd trailing byte) decoding.
- Container recursion (text nested inside a Slide inside a Document container).
- Trailing-zero-padding stop, truncated header, and over-long `recLen` clean
  stops; deep-nesting no-overflow.
- Error paths: non-CFB → `Cfb`; valid CFB lacking the stream → `NoDocumentStream`.
