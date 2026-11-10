# Changelog — coding-adventures-huffman-compression

## [0.1.1] — 2026-11-10

### Security

- SEC-003: `decompress` now treats the code-lengths table as hostile input.
  It rejects `symbol_count > 256`, duplicate symbols, code lengths outside
  1..56, tables not sorted by length, and over-subscribed (Kraft > 1) tables,
  and rejects a declared `original_length` larger than 8 x payload bytes
  before sizing anything from it. Decoding reads bits in O(1) from the payload
  bytes and fails as soon as the accumulated bits exceed the longest code.
  All failures raise `ValueError`. Valid encoder output still round-trips.

## [Unreleased]

### Changed

- Pin the canonical and Windows BUILD environments to Python 3.13 so the
  heap and Huffman Tree prerequisite closure is standalone-clean.

## [0.1.0] — 2026-04-12

### Added

- Initial implementation of CMP04 Huffman compression.
- `compress(data)` — encodes bytes using Huffman entropy coding.
- `decompress(data)` — decodes CMP04 wire-format bytes.
- CMP04 wire format: 8-byte header (original_length + symbol_count) + code-lengths
  table + LSB-first packed bit stream.
- Depends on `coding-adventures-huffman-tree` (DT27) for tree construction and
  canonical code derivation. No embedded tree logic.
- Edge cases: empty input, single distinct byte, bytearray input.
- 37 unit tests with ≥90% coverage.
