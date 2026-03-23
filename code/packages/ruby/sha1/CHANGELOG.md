# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- Full SHA-1 implementation from scratch (FIPS 180-4) with literate-programming commentary
- One-shot API: `sha1(data)` returning binary String and `sha1_hex(data)` returning hex String
- Streaming API: `Digest` class with `update()`, `digest()`, `hexdigest()`, `copy()` methods; `<<` alias
- 37 tests covering FIPS 180-4 vectors, block boundaries, edge cases, and streaming
- Knuth-style explanations for every function, constant, and algorithm step
