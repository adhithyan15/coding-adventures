# Changelog

All notable changes to `doc-writer` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project adheres to
Semantic Versioning.

## [0.1.0] — 2026-07-03

Initial release. Milestone **C4**: write a legacy `.doc` (Word 97-2003 binary,
[MS-DOC]) from a simple document model.

### Added

- `Document` model with `new()` and `add_paragraph(&str)`; paragraphs are joined
  by the Word paragraph mark `\r` into a single text run.
- `write_doc(&Document) -> Vec<u8>` — emits a valid `.doc` byte buffer:
  - a **`WordDocument`** stream: the FIB (`wIdent`, `nFib`, `fWhichTblStm`,
    `csw`, `cslw`, `ccpText`, `cbRgFcLcb`, `fcClx`, `lcbClx`) with the document
    text stored at a fixed offset (2048).
  - a **`1Table`** stream: a CLX (`Pcdt` → one-piece `PlcPcd`) that maps the
    whole document to the text via a single PCD.
  - both wrapped in an OLE2 / Compound File container via `cfb-writer`.
- **Encoding selection:** 8-bit compressed (Latin-1, one byte/char) when every
  character is `<= U+00FF`, otherwise a 16-bit UTF-16LE fallback that faithfully
  round-trips non-Latin-1 and astral (surrogate-pair) characters. The choice is
  carried by the `FcCompressed` bit-30 (`fCompressed`) flag.
- **Round-trip test:** re-implements [MS-DOC] piece-table retrieval in-test and
  opens the output with the independent `cfb` reader to prove the bytes are a
  genuinely readable `.doc`. Covers `"Hello, DOC!"`, multi-paragraph
  (`"a\rb\rc"`), non-Latin-1 (`"café 你好"`), Latin-1 supplement (`"café"` stays
  8-bit), and an astral char (`"hi 😀"`).
- Unit tests for `FcCompressed` (8-bit and 16-bit) encoding, the exact CLX byte
  layout, `lcbClx` matching the emitted CLX length, FIB field values, encoding
  selection at the `U+00FF` boundary, and the empty document.

### Robustness

- `#![forbid(unsafe_code)]`; no `unwrap`/`expect`/`panic!` on the public path;
  `checked_*` arithmetic on every offset/length a huge document could overflow,
  degrading to a valid empty document on overflow. Deterministic output.

[MS-DOC]: https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-doc/
