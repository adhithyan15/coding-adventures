# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-30

### Added

- Bounded RFC 8017 EMSA-PSS validation for the fixed SHA-512, 64-byte-salt
  profile over one complete recovered encoded message.
- Caller-owned exact scratch storage, pre-mutation public length checks,
  complete DB padding and delimiter checks, full hash comparison, independent
  test vectors, stable redacted errors, and empty required capabilities.
