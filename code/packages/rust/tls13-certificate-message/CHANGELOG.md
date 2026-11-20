# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Allocation-free TLS 1.3 Certificate body decoding.
- Caller-owned total, vector, entry-count, certificate, and extension limits.
- Borrowed entry iteration with payload-redacted failures and success Debug.
- Borrowed exact length-prefixed and payload-only forms of each entry's
  extension vector for later policy composition without reconstruction.
- Standalone build fronts and a schema-v1 zero-capability declaration.
