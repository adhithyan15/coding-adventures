# Changelog — lzw (Kotlin)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Security

- No code change: output is capped by DEFAULT_MAX_OUTPUT and nothing is sized from the header.
- Added HostileHeadersTest (JUnit 5 via kotlin.test): hostile headers under a preemptive timeout, plus round trips over every byte string of length <= 2 and a seeded random sample.

## [0.1.0] — 2026-04-25

### Added
- `LZW` object with `compress`, `decompress`, `encodeCodes`, `decodeCodes`,
  `packCodes`, `unpackCodes`.
- `BitWriter` inner class: LSB-first variable-width bit packing.
- `BitReader` inner class: LSB-first variable-width bit reading.
- `List<Byte>` used as dictionary keys (value-equality without wrapper class).
- Pre-seeded 256-entry dictionary (codes 0–255 = single bytes).
- CLEAR_CODE(256) and STOP_CODE(257) control codes.
- Variable-width codes starting at 9 bits; grows at power-of-2 boundaries;
  max 16 bits (65536 entries).
- Tricky-token handling: `code == next_code` → entry from prev + prev[0].
- Dictionary reset on CLEAR_CODE and when dictionary is full.
- CMP03 wire format: `original_length(4B BE) + LSB-first bit-packed codes`.
- Idiomatic Kotlin: `shl`/`shr`/`ushr`/`and`/`or`, `toLong() and 0xFFL`,
  `when` expressions, `isEmpty`/`isNotEmpty`, `mutableListOf`, `buildList`.
- `build.gradle.kts`, `settings.gradle.kts`, `BUILD`, `BUILD_windows`,
  `README.md`, `.gitignore`.
- 39 unit tests mirroring the Java suite.
