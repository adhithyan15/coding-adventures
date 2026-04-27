# Changelog

## 0.1.0 — 2026-04-12

### Added

- Initial implementation of CMP05 DEFLATE compression and decompression.
- `compress/1` — two-pass DEFLATE encoder.
- `decompress/1` — CMP05 wire-format decoder.
- Length and distance code tables with extra bits.
- LSB-first bit stream packing/unpacking.
- Dependencies on `coding_adventures_lzss` (CMP02) and `coding_adventures_huffman_tree` (DT27).
