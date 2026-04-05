# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial SHA-512 implementation using native u64 arithmetic
- One-shot `sum512()` and `hex_string()` functions
- Streaming `Digest` struct with `update()`, `sum512()`, `hex_digest()`, `clone_digest()`
- Full FIPS 180-4 test vector coverage
- Block boundary, edge case, and streaming tests
