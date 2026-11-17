# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-15

### Added
- Initial from-scratch Java implementation of SHA-512 cryptographic hash function (FIPS 180-4).
- One-shot `sha512` and `sha512Hex` entrypoints.
- Incremental streaming `Sha512.Hasher` with `update`, `digest`, `hexDigest`, and `copy`.
- 128-bit bit-length encoding (`encodeBitLength`) supporting messages up to FIPS domain bound ($< 2^{128}$ bits).
- Standing Rule #20 compliant extracted message constants in `Sha512Messages`.
- 6-class SHA-512 resilience test suite and conformance verification against neutral `cases.txt`.
