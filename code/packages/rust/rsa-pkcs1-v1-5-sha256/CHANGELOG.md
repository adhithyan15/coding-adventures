# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-19

### Added

- Exact RFC 8017 `00 01 || PS || 00 || DigestInfo` validation for SHA-256.
- Canonical fixed DER `DigestInfo` policy with present NULL parameters.
- Full 32-byte digest comparison and opaque zero-sized success evidence.
- Stable redacted errors, hostile structural tests, standalone BUILD fronts,
  and an empty capability declaration.
