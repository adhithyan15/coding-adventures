# Changelog — lzss (Haskell)

All notable changes to this package are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of LZSS (CMP02) in Haskell.
- `Token` ADT with `Literal Word8` and `Match { offset, matchLength }` constructors.
- `encode`: sliding-window greedy encoder with configurable window size, max match, and min match.
- `decode`: token-list decoder with byte-by-byte overlapping-copy support.
- `compress`: one-shot CMP02 wire-format serialiser (default params: window=4096, maxMatch=255, minMatch=3).
- `decompress`: one-shot CMP02 wire-format deserialiser with DoS-safe block_count capping.
- `serialiseTokens` / `deserialiseTokens` internal helpers for block-based flag-bit encoding.
- 35 hspec unit tests covering round-trips, known spec vectors, compression effectiveness, edge cases, and wire-format properties.
- `BangPatterns` extension for strict inner-loop bindings.
