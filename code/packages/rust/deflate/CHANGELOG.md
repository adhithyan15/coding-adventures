# Changelog

## 0.1.0 — 2026-04-12

### Changed

- Complete rewrite to implement CMP05 wire format using LZSS (CMP02) + dual canonical Huffman trees (DT27).

### Added

- `compress(data: &[u8]) -> Result<Vec<u8>, String>` — two-pass DEFLATE encoder.
- `decompress(data: &[u8]) -> Result<Vec<u8>, String>` — CMP05 wire-format decoder.
- Length code table (symbols 257–284) with extra bits.
- Distance code table (codes 0–23, for offsets 1–4096) with extra bits.
- LSB-first bit stream packing via `BitBuilder`.
- Dependencies on `lzss` (CMP02) and `huffman-tree` (DT27).
