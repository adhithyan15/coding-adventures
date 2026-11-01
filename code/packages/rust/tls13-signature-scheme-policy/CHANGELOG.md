# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Immutable RFC 8446 metadata for the SHA-256, SHA-384, and SHA-512
  RSASSA-PSS RSAE signature schemes.
- Ordered non-empty, duplicate-free caller policy and exact allowlisted
  selection.
- Explicit fail-closed exclusions for EdDSA, ECDSA, legacy signatures, and
  RSASSA-PSS public-key OID schemes.
- Stable payload-free errors, standalone BUILD fronts, tests, documentation,
  and an empty capability declaration.
