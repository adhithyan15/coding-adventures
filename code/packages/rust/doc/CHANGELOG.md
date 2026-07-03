# Changelog

All notable changes to the `doc` crate are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this
project adheres to Semantic Versioning.

## [0.1.0] — 2026-07-03

### Added

- Initial release: a from-scratch, zero-third-party-dependency reader for legacy
  Word 97–2003 `.doc` binaries ([MS-DOC]), extracting the main-document text.
  Layered on the `cfb` compound-file reader. `#![forbid(unsafe_code)]`.
- **FIB parsing** of the four fields needed to locate the text: `wIdent` magic
  (`0xA5EC`), the `fWhichTblStm` flag selecting `0Table`/`1Table`, and
  `fcClx`/`lcbClx` locating the CLX inside the table stream — read at their
  exact [MS-DOC] byte offsets.
- **CLX walking**: skips zero or more `Prc` (property-run) parts and reads the
  `Pcdt` part's `PlcPcd` piece table. The part loop is guaranteed to advance.
- **PlcPcd decoding**: solves `n = (lcb - 4) / 12` for the piece count,
  validates `lcb >= 4` and `(lcb - 4) % 12 == 0`, walks the parallel CP and PCD
  arrays, and concatenates pieces in character-position order.
- **FcCompressed** bit trick: bit 30 selects 8-bit (Latin-1, bytes at `real/2`)
  vs 16-bit (UTF-16LE, bytes at `real`); surrogate pairs handled, lone
  surrogates map to U+FFFD.
- Public API: `open_doc`, `Document::text`, and the `DocError` taxonomy
  (`Cfb`, `NotWordDocument`, `NoTableStream`, `MalformedPieceTable`,
  `Truncated`) with `Display`, `std::error::Error` (`source()`), and
  `From<cfb::CfbError>`.
- **Security hardening** for untrusted input: every file-derived offset/length
  is bounds-checked against the actual (zero-padded) stream bytes with
  `checked_*` arithmetic and `get(..)`; total decoded text capped at 64 MiB and
  piece count capped; no `unwrap`/`expect`/`panic!`/panicking index.
- An internal `extract_text` seam lets the piece-table logic be unit-tested with
  synthetic buffers, independent of `cfb`.
- End-to-end test reading a real CFB-wrapped fixture that decodes to
  `"Hello, DOC!"`, plus extensive unit tests (compressed / uncompressed /
  multi-piece decode, the piece-count math and its rejection, `Prc` skipping,
  and all error paths).
- Spec: `code/specs/DOC01-binary-reader.md` — literate walkthrough of the
  CFB layering, FIB fields, CLX/Prc/Pcdt structure, PlcPcd layout, and the
  FcCompressed decode.
