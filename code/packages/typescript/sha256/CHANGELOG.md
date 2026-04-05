# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial SHA-256 implementation (FIPS 180-4)
- One-shot `sha256()` returning 32-byte Uint8Array
- One-shot `sha256Hex()` returning 64-character hex string
- `toHex()` utility for byte-to-hex conversion
- `SHA256Hasher` streaming class with `update()`, `digest()`, `hexDigest()`, `copy()`
- Full FIPS 180-4 test vectors (empty, "abc", 56-byte, million-a)
- Block boundary tests (55, 56, 63, 64, 127, 128 bytes)
- Avalanche effect test
- Streaming API tests (split, byte-at-a-time, copy, non-destructive digest)
