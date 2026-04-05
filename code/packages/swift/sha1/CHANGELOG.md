# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added

- Initial implementation of SHA-1 (FIPS 180-4) in pure Swift
- One-shot API: `sha1(_:)` returns 20-byte `Data`, `sha1Hex(_:)` returns 40-char hex string
- Streaming API: `SHA1Hasher` struct with `update(_:)`, `digest()`, `hexDigest()`, `copy()`
- Big-endian byte ordering throughout (matching FIPS 180-4)
- 80-round compression with message expansion via XOR+rotate
- Merkle-Damgard padding with 64-bit big-endian length field
- Full test suite with FIPS 180-4 test vectors, edge cases, and streaming tests
