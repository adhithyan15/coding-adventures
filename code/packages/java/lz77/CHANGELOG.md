# Changelog — lz77 (Java)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-25

### Added
- `LZ77.compress(byte[])` / `decompress(byte[])` — one-shot CMP00 wire format.
- `LZ77.encode(byte[])` — encode to `List<Token>` with configurable
  `windowSize`, `maxMatch`, `minMatch` (defaults: 4096 / 255 / 3).
- `LZ77.decode(List<Token>)` — decode token stream; optional `initialBuffer`
  seed for streaming decompression.
- `LZ77.Token` record `(offset, length, nextChar)` with `isLiteral()`,
  `literal(int)`, `match(int,int,int)` factory methods.
- `serialiseTokens` / `deserialiseTokens` for the CMP00 wire format.
- `findLongestMatch` — greedy O(n × window) search with overlapping-match
  support and one-byte next_char reservation.
- 38 unit tests covering round-trip, token stream, wire format, edge cases,
  overlapping matches, compression effectiveness, and determinism.
- `build.gradle.kts`, `settings.gradle.kts`, `BUILD`, `BUILD_windows`,
  `README.md`, `.gitignore`.
