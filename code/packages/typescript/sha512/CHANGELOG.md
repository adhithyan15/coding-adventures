# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial SHA-512 implementation using BigInt for 64-bit arithmetic
- One-shot `sha512()` and `sha512Hex()` functions
- Streaming `SHA512Hasher` class with `update()`, `digest()`, `hexDigest()`, `copy()`
- `toHex()` utility function
- Full FIPS 180-4 test vector coverage
- Block boundary and edge case tests
