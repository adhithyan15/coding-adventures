# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- Full SHA-1 implementation from scratch (FIPS 180-4) with literate-programming commentary
- One-shot API: `sha1/1` returning binary and `sha1_hex/1` returning hex string
- Uses Elixir bitstring pattern matching for big-endian word parsing
- 22 tests covering FIPS 180-4 vectors, block boundaries, edge cases, and avalanche
- Knuth-style explanations for every function, constant, and algorithm step
