# Changelog

## 0.1.0 — 2026-04-12

### Added

- Initial implementation of CMP05 DEFLATE compression and decompression.
- `Deflate.compress(_:)` — two-pass DEFLATE encoder (LZSS + dual Huffman).
- `Deflate.decompress(_:)` — CMP05 wire-format decoder.
- Length and distance code tables with extra bits.
- LSB-first bit stream packing/unpacking.
- Dependencies on `LZSS` (CMP02) and `HuffmanTree` (DT27).
