# Changelog — kotlin/zstd

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-24

### Added
- Initial implementation of ZStd (CMP07) compression and decompression in Kotlin.
- `Zstd.compress(ByteArray): ByteArray` — produces a conforming RFC 8878 ZStd frame.
- `Zstd.decompress(ByteArray): ByteArray` — decodes Raw, RLE, and Compressed blocks.
- FSE predefined tables for LL, ML, and OF coding (RFC 8878 Appendix B).
- `RevBitWriter` / `RevBitReader` — backward bitstream codec for the FSE sequence section.
- Raw literals encoding/decoding (1-byte, 2-byte, and 3-byte header variants).
- LZSS integration via `com.codingadventures:lzss` for LZ77 token generation.
- 24 unit tests covering round-trips, wire format, internal codec helpers, and edge cases.
