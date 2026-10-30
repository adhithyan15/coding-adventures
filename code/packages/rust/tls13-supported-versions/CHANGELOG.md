# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Allocation-free validation and encoding for TLS 1.3 ClientHello
  `supported_versions` extension data.
- Exact ServerHello and HelloRetryRequest selected-version decoding bound to
  the caller's original offer.
- Duplicate-free, caller-ordered version policy that requires TLS 1.3 while
  preserving explicitly configured unknown version identifiers.
- Stable payload-free failures, failure-atomic output, and no normal
  dependencies or external effects.
