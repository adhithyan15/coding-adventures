# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Allocation-free RFC 8017 MGF1 expansion specialized to a 32-byte SHA-256
  RSA-PSS verification seed.
- Independent RFC-algorithm vector coverage across multiple blocks and a
  partial final block.
- Caller-selected work limits, RFC maximum enforcement, failure atomicity,
  stable redacted errors, standalone BUILD fronts, and an empty capability
  declaration.
