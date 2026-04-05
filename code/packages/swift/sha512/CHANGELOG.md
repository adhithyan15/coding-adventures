# Changelog

All notable changes to the SHA-512 Swift package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial implementation of SHA-512 (FIPS 180-4)
- `sha512(Data) -> Data` one-shot function returning 64-byte digest
- `sha512Hex(Data) -> String` returning 128-character lowercase hex string
- `SHA512Hasher` streaming hasher with `update`, `digest`, `hexDigest`, `copy`
- Full FIPS 180-4 test vectors
- Literate-style inline documentation explaining the algorithm
