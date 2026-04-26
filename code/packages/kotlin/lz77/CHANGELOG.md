# Changelog — lz77 (Kotlin)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
