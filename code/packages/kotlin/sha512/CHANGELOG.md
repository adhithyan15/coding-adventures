# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-15

### Added
- Initial from-scratch Kotlin implementation of SHA-512 cryptographic hash function (FIPS 180-4).
- Idiomatic Kotlin top-level `sha512(ByteArray): ByteArray` and `sha512Hex(ByteArray): String`.
- Streaming `Sha512.Hasher` with `update`, `digest`, `hexDigest`, and `copy`.
- 128-bit bit-length encoding (`encodeBitLength`) supporting messages up to FIPS domain bound ($< 2^{128}$ bits).
- Extracted error and diagnostic messages in `Sha512Messages` enforcing Standing Rule #20.
- Full 6-class SHA-512 resilience suite and FIPS 180-4 conformance against `cases.txt`.
