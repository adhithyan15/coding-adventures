# Changelog

## 0.1.0 — 2026-04-12

### Added

- Initial implementation of CMP05 DEFLATE compression and decompression.
- `compress(data: Uint8Array): Uint8Array` — two-pass DEFLATE encoder.
- `decompress(data: Uint8Array): Uint8Array` — CMP05 wire-format decoder.
- Length and distance code tables with extra bits.
- LSB-first bit stream packing via `BitBuilder`.
- Dependencies on `@coding-adventures/lzss` and `@coding-adventures/huffman-tree`.
