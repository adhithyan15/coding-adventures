# Changelog

All notable changes to the CommonMark 0.31.2 conformance foundation are
documented here.

## 0.1.0 — 2026-10-24

### Added

- Pinned the exact official 652-example CommonMark 0.31.2 JSON artifact with
  immutable source, license, byte-length, and SHA-256 provenance.
- Added closed corpus, provenance, and renderer schemas.
- Added sixteen independently authored TE00 renderer vectors covering all
  nineteen node kinds, raw-HTML routing, escaping, URL rejection, list modes,
  nesting, and deterministic block order.
- Added dependency-free strict validation for duplicate keys, ambiguous
  numbers, UTF-8 and Unicode scalars, transport and decoded-tree bounds,
  object-graph aliases and cycles, exact official metadata, TE00 containment,
  complete node coverage, and raw plus semantic digests.
- Added adversarial tests and explicit separation from native reader
  migrations and GFM adoption.
- Added always-run hosted CI selection for the foundation validator and its
  dedicated falsification suite.
- Corrected the migration inventory to all sixteen live CommonMark and GFM
  package readers across seven languages.
