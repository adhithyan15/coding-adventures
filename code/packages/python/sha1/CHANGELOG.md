# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- Full SHA-1 implementation from scratch (FIPS 180-4) with literate-programming commentary
- One-shot API: `sha1(data: bytes) -> bytes` and `sha1_hex(data: bytes) -> str`
- Streaming API: `SHA1` class with `update()`, `digest()`, `hexdigest()`, `copy()` methods
- 37 tests covering FIPS 180-4 vectors, block boundaries, edge cases, and streaming
- 100% test coverage
- Knuth-style explanations for every function, constant, and algorithm step
