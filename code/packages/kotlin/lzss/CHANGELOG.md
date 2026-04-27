# Changelog — lzss (Kotlin)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
