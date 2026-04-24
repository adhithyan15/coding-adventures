# Changelog — java/zstd

All notable changes to this package will be documented in this file.

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of `Zstd.compress(byte[])` and `Zstd.decompress(byte[])`.
- FSE (Finite State Entropy) encode/decode tables built from RFC 8878 Appendix B
  predefined distributions (LL, ML, OF).
- `RevBitWriter` and `RevBitReader` for the ZStd backward bit-stream codec.
- Raw_Literals section encoding/decoding with 1/2/3-byte variable-length header.
- Sequence section encoding/decoding with predefined FSE modes.
- Block types: Raw (00), RLE (01), Compressed (10).
- Multi-block support for inputs larger than 128 KB.
- 16 JUnit 5 unit tests covering empty input, single bytes, all-byte values,
  RLE detection, prose compression ratio, pseudo-random data, multi-block frames,
  repeat-offset patterns, determinism, and internal codec round-trips.
- `BUILD`, `BUILD_windows`, `required_capabilities.json`, `README.md`.
- Depends on `com.codingadventures:lzss` for LZ77 tokenisation.
