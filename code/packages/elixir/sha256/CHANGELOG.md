# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial SHA-256 implementation (FIPS 180-4)
- One-shot `sha256/1` returning 32-byte binary
- One-shot `sha256_hex/1` returning 64-character hex string
- `CodingAdventures.Sha256.Hasher` streaming module with `new/0`, `update/2`, `digest/1`, `hex_digest/1`, `copy/1`
- Full FIPS 180-4 test vectors (empty, "abc", 56-byte, million-a)
- Block boundary, avalanche, edge case, and streaming tests
