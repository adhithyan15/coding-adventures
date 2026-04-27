# Changelog

## 0.1.0 — 2026-04-12

### Added

- Initial implementation of CMP05 DEFLATE compression and decompression.
- `compress(data)` — two-pass DEFLATE: LZSS tokenization followed by dual canonical Huffman coding.
- `decompress(data)` — full CMP05 wire-format decoder.
- Length code table (symbols 257–284) with extra bits for exact match lengths.
- Distance code table (codes 0–23) with extra bits for offsets 1–4096.
- CMP05 wire format: 8-byte header + 3-byte-per-entry LL and dist tables + LSB-first bit stream.
- `py.typed` marker for PEP 561 typing support.
- Comprehensive test suite with edge cases, round-trip tests, and compression ratio test.
- Dependency on `coding-adventures-lzss` (CMP02) for LZSS tokenization.
- Dependency on `coding-adventures-huffman-tree` (DT27) for Huffman tree construction.
