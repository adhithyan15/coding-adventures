# Changelog

## 0.1.0 — 2026-04-13

### Added

- Initial implementation of CMP06 Brotli-style compression and decompression.
- `compress($data)` — two-pass encoder with LZ matching and Huffman coding.
- `decompress($data)` — CMP06 wire-format decoder.
- Four literal context buckets (bucket 0: space/punct, 1: digit, 2: uppercase, 3: lowercase).
- 64-entry ICC (insert-copy code) table bundling insert + copy length ranges.
- 32-entry distance code table covering offsets 1–65535 (extends CMP05's 24 codes to 32).
- 65535-byte sliding window LZ matcher, minimum match length 4.
- LSB-first bit stream packing/unpacking.
- Single-symbol Huffman tree support (code "0" per spec).
- Empty input special case (header + sentinel ICC code 63 only).
- Dependency on `CodingAdventures::HuffmanTree` (DT27); no LZSS dependency.
- Full test suite covering all 10 CMP06 spec test cases.
