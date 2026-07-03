# Changelog

All notable changes to `coding-adventures-opc-writer` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/); this
project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-07-03

Initial release — the write side of OOXML milestone **C1**, mirroring the
read-side `opc` crate.

### Added

- `PackageWriter` — assembles OOXML parts, content types, and relationships into
  a ZIP-based OPC package.
  - `add_default(extension, content_type)` — register a `<Default>` content type
    by file extension (de-duplicated, last registration wins).
  - `add_part(part_name, content_type, data)` — add a part typed by an
    `<Override>` (de-duplicated by part name, last wins).
  - `add_part_defaulted(part_name, data)` — add a part typed by a `<Default>`
    (e.g. `.rels` files), with no per-part override.
  - `finish()` — synthesize `[Content_Types].xml` and emit the ZIP bytes, with
    the content-types part written first per convention.
- `RelationshipsBuilder` — serialize a `.rels` XML part from `(id, type, target)`
  entries, with escaped targets.
- `xml_escape` — a total XML escaper (`& < > " '`) that passes arbitrary Unicode
  through unchanged.
- Round-trip test: the writer's output re-opens under the read-side `opc` crate
  with content types resolving and relationships dereferencing correctly.

### Notes

- `#![forbid(unsafe_code)]`; no `unwrap`/`expect`/`panic!` on input paths.
- Format-agnostic by design — usable by future `.docx` / `.pptx` writers.
