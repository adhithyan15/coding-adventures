# Changelog — lz77 (Kotlin)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Security

- CWE-789/CWE-400: decode copied the whole output (out.toByteArray()) for every back-referenced byte, making decode quadratic in output size; it now uses a growable array. deserialiseTokens reads the header count as unsigned (a count >= 2^31 went negative and silently decoded nothing) before capping it to the payload. Invalid offsets still throw IllegalArgumentException.
- Added HostileHeadersTest (JUnit 5 via kotlin.test): hostile headers under a preemptive timeout, plus round trips over every byte string of length <= 2 and a seeded random sample.

## [0.1.0] — 2026-04-25

### Added
- `LZ77` object with `compress`, `decompress`, `encode`, `decode`,
  `serialiseTokens`, `deserialiseTokens`.
- `Token` data class with `isLiteral` property and `literal`/`match`
  companion factory functions.
- Idiomatic Kotlin: `Pair<Int,Int>` from `findLongestMatch`, `buildList{}`,
  `ByteArrayOutputStream`, `minOf`/`maxOf`.
- `decode` accepts optional `initialBuffer` seed for streaming use.
- 38 unit tests mirroring the Java suite.
- `build.gradle.kts`, `settings.gradle.kts`, `BUILD`, `BUILD_windows`,
  `README.md`, `.gitignore`.
