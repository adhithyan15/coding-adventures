# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added

- Initial implementation of MD5 (RFC 1321) in pure Swift
- One-shot API: `md5(_:)` returns 16-byte `Data`, `md5Hex(_:)` returns 32-char hex string
- Streaming API: `MD5Hasher` struct with `update(_:)`, `digest()`, `hexDigest()`, `copy()`
- Little-endian byte ordering throughout (matching RFC 1321)
- T-table constants derived from `sin()` function
- Merkle-Damgard padding with 64-bit little-endian length field
- Full test suite with RFC 1321 test vectors, edge cases, and streaming tests
