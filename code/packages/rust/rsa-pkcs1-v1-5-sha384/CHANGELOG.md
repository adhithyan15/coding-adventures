# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Exact RFC 8017 `00 01 || PS || 00 || DigestInfo` validation for SHA-384.
- Canonical fixed DER `id-sha384` `DigestInfo` policy with present NULL
  parameters.
- Complete 48-byte digest comparison and opaque zero-sized success evidence.
- Stable redacted failures, hostile structural tests, standalone BUILD fronts,
  and an empty capability declaration.
