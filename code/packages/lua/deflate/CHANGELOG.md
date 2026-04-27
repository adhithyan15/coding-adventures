# Changelog

## 0.1.0 — 2026-04-12

### Added

- Initial implementation of CMP05 DEFLATE compression and decompression.
- `compress(data)` — two-pass DEFLATE encoder.
- `decompress(data)` — CMP05 wire-format decoder.
- Length and distance code tables with extra bits.
- LSB-first bit stream packing/unpacking.
- Dependencies on `coding-adventures-lzss` and `coding-adventures-huffman-tree`.
