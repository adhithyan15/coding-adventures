# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-05

### Added

- Initial implementation of SHA-256 (FIPS 180-4)
- One-shot API: `sha256()` returns 32-byte `Data`, `sha256Hex()` returns 64-char hex string
- Streaming API: `SHA256Hasher` with `update()`, `digest()`, `hexDigest()`, `copy()`
- `SHA256Hasher` conforms to `Sendable` for concurrency safety
- Literate programming style with extensive inline documentation
- Full test suite with NIST test vectors, boundary conditions, and streaming tests
