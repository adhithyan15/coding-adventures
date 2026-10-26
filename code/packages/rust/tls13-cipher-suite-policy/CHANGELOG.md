# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-25

### Added

- Immutable RFC 8446 metadata for TLS AES-128-GCM/SHA-256,
  AES-256-GCM/SHA-384, and ChaCha20-Poly1305/SHA-256.
- Allocation-free caller allowlists with duplicate detection and explicit
  per-suite record ceilings.
- Exact, fail-closed wire selection that keeps algorithm, hash, length, and
  record-policy data bound in one validated result.
- Stable redacted failures and no normal dependencies or external effects.
