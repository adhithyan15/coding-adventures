# Changelog — lz77 (Java)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Security

- CWE-789/CWE-400: decode copied the whole output (out.toByteArray()) for every back-referenced byte, making decode quadratic in output size (a few MB of 'A' took minutes); it now uses a growable array. deserialiseTokens reads the header count as unsigned (a count >= 2^31 went negative and made new ArrayList<>(negative) throw) before capping it to the payload. Invalid offsets (0 or beyond the output) still throw IllegalArgumentException.
- Added HostileHeadersTest (JUnit 5): hostile headers run under a preemptive timeout, plus round trips over every byte string of length <= 2 and a seeded random sample.

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
