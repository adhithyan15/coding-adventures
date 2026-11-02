# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Checked caller policy for selected message-hash and salt lengths plus a
  canonical `M'` work ceiling.
- Allocation-free RFC 8017 `zeroes(8) || mHash || salt` construction.
- Borrowed non-copyable output evidence with redacted diagnostics.
- Independent boundary coverage, standalone build fronts, and a schema-v1
  zero-capability declaration.
