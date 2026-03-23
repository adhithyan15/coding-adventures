# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- Full SHA-1 implementation from scratch (FIPS 180-4) with literate-programming commentary
- One-shot API: `sum1(data: &[u8]) -> [u8; 20]` and `hex_string(data: &[u8]) -> String`
- Streaming API: `Digest` struct with `update()`, `sum1()`, `hex_digest()`, `clone_digest()` methods
- Uses `wrapping_add` for explicit mod-2^32 arithmetic and `u32::rotate_left` for ROTL
- 27 unit tests + 4 doc tests covering FIPS 180-4 vectors, block boundaries, edge cases, streaming
- Knuth-style explanations for every function, constant, and algorithm step
