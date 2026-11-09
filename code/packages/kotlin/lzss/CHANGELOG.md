# Changelog — lzss (Kotlin)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Security

- CWE-789/CWE-400: decode copied the whole output per back-referenced byte (quadratic); it now uses a growable array and stops once originalLength is reached. deserialiseTokens reads block_count as unsigned before capping it to the payload (>= 2^31 previously decoded zero blocks). Match offsets of 0 are rejected with IllegalArgumentException.
- Added HostileHeadersTest (JUnit 5 via kotlin.test): hostile headers under a preemptive timeout, plus round trips over every byte string of length <= 2 and a seeded random sample.

## [0.1.0] — 2026-04-25

### Added
- `LZSS` object with `compress`, `decompress`, `encode`, `decode`,
  `serialiseTokens`, `deserialiseTokens`.
- Sealed `Token` interface with `Literal` and `Match` data classes.
- Flag-byte scheme: 8 symbols per block; bit i=0 → Literal (1B),
  bit i=1 → Match (3B: offset BE uint16 + length uint8).
- CMP02 wire format: `original_length(4B) + block_count(4B) + blocks`.
- Overlapping-match support: byte-by-byte copy in `decode`.
- Idiomatic Kotlin: `sealed interface`, `when` expressions, `shl`/`shr`/`and`,
  `minOf`/`maxOf`, `toInt() and 0xFF` for unsigned byte handling.
- `build.gradle.kts`, `settings.gradle.kts`, `BUILD`, `BUILD_windows`,
  `README.md`, `.gitignore`.
- 37 unit tests mirroring the Java suite.
