# Changelog

All notable changes to `ppt-writer` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-07-03

### Added

- Initial release (**PPTW01**): a from-scratch, zero-third-party-dependency
  **writer** for the legacy **PowerPoint 97-2003 binary** format (`.ppt`,
  [MS-PPT]). Milestone **C4**.
- Public API:
  - `Presentation::new()`, `Presentation::add_slide() -> &mut Slide`.
  - `Slide::new()`, `Slide::add_text(&str)` — one paragraph → one text atom.
  - `write_ppt(&Presentation) -> Vec<u8>`.
- **Record emission** per [MS-PPT]:
  - 8-byte **RecordHeader** with bit-packed `recVerAndInstance`
    (`(recVer & 0xF) | (recInstance << 4)`), `recType`, and a `u32` body
    `recLen`; `recVer == 0xF` marks a container.
  - **Slide container** (`recType 0x03EE`, `recVer 0xF`) per slide.
  - **TextBytesAtom** (`0x0FA8`) for all-Latin-1 paragraphs (one byte per char)
    and **TextCharsAtom** (`0x0FA0`, UTF-16LE) for anything with a char > 0xFF —
    chosen per paragraph.
- **CFB wrapping** via the sibling `cfb-writer` crate: the concatenated Slide
  containers form the **"PowerPoint Document"** stream, plus a 4-byte stub
  **"Current User"** stream for authenticity.
- **Robustness**: `#![forbid(unsafe_code)]`; no `unwrap`/`expect`/`panic!` on
  the public path; oversize bodies skipped via `u32::try_from` rather than
  wrapped into a corrupting length; deterministic output.
- **Round-trip proof**: writes a two-slide deck, reopens it with the `cfb`
  reader, extracts the "PowerPoint Document" stream, and walks the record tree
  (recursing into containers, decoding text atoms, stopping on the sector
  zero-padding sentinel). Asserts two Slide containers and their exact
  paragraph text. 13 unit tests + 1 doctest, all passing; clippy clean under
  `-D warnings`.

### Known limitations

- Minimal record profile: no `DocumentContainer`, persist-object directory,
  master slides, or formatting — just Slide containers with text atoms,
  sufficient to place and round-trip slide text for milestone C4.
- Only Slide-level text is modelled; shapes, tables, images, and speaker notes
  are out of scope for 0.1.0.

[MS-PPT]: https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-ppt/
