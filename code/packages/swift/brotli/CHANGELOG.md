# Changelog

## 0.1.0 — 2026-04-13

### Added

- Initial implementation of CMP06 Brotli-inspired compression and decompression.
- `Brotli.compress(_:)` — two-pass encoder with LZ matching, context-dependent
  Huffman trees, and insert-and-copy command bundling.
- `Brotli.decompress(_:)` — CMP06 wire-format decoder.
- 64-entry ICC table covering insert bases 0, 1, 2, 3, 5, 9, 17 with copy
  lengths up to 769 bytes (code 55), plus end-of-data sentinel (code 63).
- 32-entry distance table covering distances 1–65535 (codes 0–31).
- 4 literal context buckets derived from the preceding byte's character class
  (space/punct, digit, uppercase, lowercase).
- LSB-first bit stream packing (`BitBuilder`) and unpacking.
- 10-byte wire format header with entry counts for ICC, distance, and 4 literal trees.
- Canonical Huffman tree reconstruction for decompression.
- Single-symbol Huffman tree edge case: code = "0".
- Empty input special case: 13-byte minimal wire format.
- `BrotliError` enum for compressed-data errors.
- Dependency on `CodingAdventuresHuffmanTree` (DT27).
- `BUILD` and `BUILD_windows` files using `swift test`.
- XCTest suite (`BrotliTests`) covering all 8 spec test cases plus 20+
  additional edge-case and stress tests.
