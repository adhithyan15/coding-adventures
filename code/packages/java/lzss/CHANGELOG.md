# Changelog — lzss (Java)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-25

### Added
- `LZSS` class with `compress`, `decompress`, `encode`, `decode`,
  `serialiseTokens`, `deserialiseTokens`.
- Sealed `Token` interface with `Literal` record and `Match` record
  (Java 21 sealed hierarchy + pattern matching).
- Flag-byte scheme: 8 symbols per block; bit i=0 → Literal (1B),
  bit i=1 → Match (3B: offset BE uint16 + length uint8).
- CMP02 wire format: `original_length(4B) + block_count(4B) + blocks`.
- `original_length` stored in header (no sentinel in pure flag-bit stream).
- Overlapping-match support: byte-by-byte copy in `decode`.
- `build.gradle.kts`, `settings.gradle.kts`, `BUILD`, `BUILD_windows`,
  `README.md`, `.gitignore`.
- 35+ unit tests covering round-trip, token stream, wire format, edge cases,
  overlapping matches, effectiveness, and determinism.
