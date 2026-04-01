# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-01

### Added

- `deflate_compress()` — raw deflate compression (RFC 1951)
- `zlib_compress()` — zlib-wrapped deflate (RFC 1950)
- `adler32()` — Adler-32 checksum
- LZ77 matching with 32KB sliding window
- Fixed Huffman codes (RFC 1951 §3.2.6)
