# Changelog

All notable changes to `coding-adventures-huffman-compression` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
