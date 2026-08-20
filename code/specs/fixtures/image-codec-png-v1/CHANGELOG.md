# Changelog — IC18 portable PNG fixtures

## 1.0.0 — 2026-08-20

### Added

- Closed Draft 2020-12 schema for the IC18 PNG profile, fixed resource limits,
  five operations, and 29 stable error identifiers.
- Added suggested `PLTE`, semantic `tRNS`, Paeth branch/tie, and normative
  encoder filter-selection evidence.
- Replaced host-zlib compression choices with deterministic stored/fixed
  encoders and a checked dynamic-Huffman vector.
- Eighty-two deterministic cases covering all supported colour forms and
  filters, stored/fixed/dynamic DEFLATE, split IDAT, independent Adler-32,
  encoder interoperability, malformed framing and zlib streams, exact
  consumption, and caller-lowerable resource limits.
- Reproducible standard-library Python generator and an independent repository
  validator that pins schema, cases, all error paths, regeneration, size bounds,
  filter coverage, DEFLATE block coverage, CRCs, zlib framing, unfiltering, and
  RGBA widening.
