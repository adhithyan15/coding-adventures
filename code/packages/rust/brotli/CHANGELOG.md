# Changelog

## 0.1.0 — 2026-04-13

### Added

- `compress(data: &[u8]) -> Vec<u8>` — CMP06 Brotli encoder (two-pass: LZ matching then Huffman).
- `decompress(data: &[u8]) -> Vec<u8>` — CMP06 wire-format decoder.
- 64-entry Insert-Copy Code (ICC) table bundling insert and copy length ranges.
- 32-entry distance code table covering offsets 1–65535.
- 4 literal context buckets based on the preceding byte's character class.
- 65535-byte sliding window LZ matching with minimum match length 4.
- LSB-first bit packing via `BitBuilder`.
- Wire format: 10-byte header + ICC table + dist table + 4 literal trees + bit stream.
- End-of-data sentinel using ICC code 63 (insert=0, copy=0).
- Dependency on `huffman-tree` (DT27). No LZSS dependency.
