# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Bounded, duplicate-free client cipher-suite preference offers that retain
  explicitly configured unknown wire identifiers.
- Failure-atomic RFC 8446 vector encoding into caller-owned storage.
- Server-selection binding to both the original offer and the repository-owned
  cipher-suite policy, returning its complete selected-suite descriptor.
- Metadata-only diagnostics, payload-free failures, and zero external effects.
