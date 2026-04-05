# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial implementation of SHA-512 (FIPS 180-4) from scratch.
- One-shot API: `sha512()` and `sha512_hex()`.
- Streaming API: `SHA512Hasher` with `update()`, `digest()`, `hex_digest()`, `copy()`.
- Full FIPS 180-4 test vectors including empty string, "abc", 896-bit message, and one million 'a' characters.
- Block boundary tests (111, 112, 128, 255, 256 bytes).
- Edge case tests (null bytes, all byte values, UTF-8 text).
- Literate programming style with extensive inline documentation.
