# Changelog

All notable changes to `coding-adventures-opc` are documented here. The format
is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-07-02

Initial release — milestone **M2** of the OOXML effort: a format-agnostic Open
Packaging Conventions (OPC) package reader.

### Added

- `Package::open(bytes) -> Result<Package, OpcError>` — opens an OOXML ZIP
  package, inflating every member into memory and eagerly parsing
  `/[Content_Types].xml` (failing fast if it is missing or malformed).
- `Package::part_names()` — sorted list of logical, `/`-rooted part names.
- `Package::has_part(name)` and `Package::read_part(name)` — existence check and
  raw-bytes access; both accept a name with or without a leading `/`.
- `Package::content_type(part_name)` — resolves the content type with
  **Override before Default**; Default matches by case-insensitive extension.
- `Package::relationships(source_part)` — the relationships declared for a part
  (or the package, via the `"/"` sentinel), parsed lazily and cached. Absent
  `.rels` files yield an empty list, not an error.
- `Package::resolve(source_part, rel_id)` — dereference a relationship id to a
  resolved internal part name.
- `Package::main_document_part()` — the main document part, discovered by
  following the package-level `/officeDocument` relationship.
- `Relationship` struct (`id`, `rel_type`, `target`, `mode`, `resolved_target`)
  and `TargetMode` enum (`Internal` / `External`).
- `OpcError` enum: `NotAZip`, `MissingContentTypes`, `MalformedXml`, `NotUtf8`.
- Relationship `Target` resolution relative to the **source part's directory**,
  with a `.`/`..`-aware URI-join that **clamps directory traversal at the
  package root** (a hostile `../../..` target can never escape the package).
  `External` targets are left as opaque URIs and never resolved to parts.
- `fixture` module exposing `MINIMAL_XLSX`, a real DEFLATE-compressed `.xlsx`
  with 6 OPC parts, reused by the crate doctest and the test suite.

### Notes

- Built on the `zip` (M0) and `coding-adventures-xml-parser` (M1) crates.
- Pure computation — no filesystem, network, process, or environment access
  (see `required_capabilities.json`).
- Deliberately **no document-format semantics**; interpreting parts as
  workbooks/sheets is milestone M3 (SpreadsheetML), which consumes this crate.
