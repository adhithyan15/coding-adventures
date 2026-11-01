# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Checked caller policy for selected salt length and a public work ceiling.
- Canonical RFC 8017 `PS || 0x01 || salt` validation with bounded scanning.
- Borrowed non-copyable evidence retaining the exact block and salt without
  allocating or granting signature authority.
- Stable redacted failures, standalone build fronts, independent boundary
  coverage, and a schema-v1 zero-capability declaration.
