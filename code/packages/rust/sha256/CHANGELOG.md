# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial SHA-256 implementation (FIPS 180-4)
- One-shot `sha256()` returning `[u8; 32]`
- One-shot `sha256_hex()` returning 64-character hex String
- `Sha256Hasher` streaming struct with `update()`, `digest()`, `hex_digest()`, `clone_hasher()`
- Full FIPS 180-4 test vectors (empty, "abc", 56-byte, million-a)
- Block boundary, avalanche, edge case, and streaming tests
