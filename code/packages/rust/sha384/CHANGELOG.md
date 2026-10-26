# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-25

### Added

- Fixed-memory one-shot and streaming SHA-384 APIs.
- FIPS 180-4 initial state, compression, padding, and 384-bit output rules.
- Checked streaming length accounting with payload-free failures.
- Volatile wiping for retained and temporary state through the repository-owned
  `zeroize` primitive.
- FIPS/NIST known-answer and streaming-boundary tests.
