# Changelog — lzw (Java)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-25

### Added
- `LZW` class with `compress`, `decompress`, `encodeCodes`, `decodeCodes`,
  `packCodes`, `unpackCodes`.
- `BitWriter` inner class: LSB-first variable-width bit packing.
- `BitReader` inner class: LSB-first variable-width bit reading.
- `ByteKey` inner class: value-based byte-array HashMap key.
- Pre-seeded 256-entry dictionary (codes 0–255 = single bytes).
- CLEAR_CODE(256) and STOP_CODE(257) control codes.
- Variable-width codes starting at 9 bits; grows at power-of-2 boundaries;
  max 16 bits (65536 entries).
- Tricky-token handling: `code == next_code` → entry from prev + prev[0].
- Dictionary reset on CLEAR_CODE and when dictionary is full.
- CMP03 wire format: `original_length(4B BE) + LSB-first bit-packed codes`.
- `build.gradle.kts`, `settings.gradle.kts`, `BUILD`, `BUILD_windows`,
  `README.md`, `.gitignore`.
- 38+ unit tests covering round-trip, code stream, wire format, edge cases,
  tricky token, bit I/O, effectiveness, and determinism.
