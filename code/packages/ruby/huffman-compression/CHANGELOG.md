# Changelog

All notable changes to `coding-adventures-huffman-compression` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.1] — 2026-11-10

### Security

- SEC-003: `decompress` now treats the code-lengths table as hostile input.
  It raises `ArgumentError` for `symbol_count > 256`, duplicate symbols, code
  lengths outside 1..56, tables not sorted by length, over-subscribed
  (Kraft > 1) tables, and a declared `original_length` larger than 8 x payload
  bytes (checked before any sizing). Bits are read in O(1) from the payload
  and decoding fails as soon as the accumulated bits exceed the longest code.
  Valid encoder output still round-trips.

## [0.1.0] — 2026-04-12

### Added

- Initial implementation of CMP04 Huffman compression.
- `HuffmanCompression.compress(data)` — compresses a binary String to CMP04 wire format.
- `HuffmanCompression.decompress(data)` — reconstructs the original bytes from CMP04 wire format.
- CMP04 wire format: big-endian `original_length` + `symbol_count` header, canonical code-lengths
  table sorted by (length, symbol), LSB-first packed bit stream.
- Canonical code reconstruction from lengths alone (no tree structure in wire format).
- LSB-first bit-packing and unpacking helpers (private class methods).
- Edge case handling: empty input, single-byte input, single-distinct-byte input, all 256 values.
- Full minitest suite (28 tests) with round-trip, wire-format structural, and edge-case coverage.
- Delegates tree construction to `coding-adventures-huffman-tree` (DT27) gem.
- StandardRB-compliant code style.
