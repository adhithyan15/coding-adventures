# Changelog

## 0.1.0 — 2026-04-12

### Added

- Initial implementation of CMP05 DEFLATE compression and decompression.
- `Compress(data []byte) ([]byte, error)` — two-pass DEFLATE encoder.
- `Decompress(data []byte) ([]byte, error)` — CMP05 wire-format decoder.
- Length code table (symbols 257–284) with extra bits.
- Distance code table (codes 0–23) with extra bits.
- LSB-first bit stream packing via `bitBuilder`.
- Canonical Huffman code reconstruction for the decoder.
- Dependency on `lzss` (CMP02) and `huffman-tree` (DT27).
