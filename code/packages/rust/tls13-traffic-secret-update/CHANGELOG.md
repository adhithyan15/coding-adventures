# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Exact RFC 8446 section 7.2 `"traffic upd"` derivation over the
  repository-owned fixed-memory SHA-256/SHA-384 HKDF boundary.
- Caller-owned output with pre-mutation Hash.length validation and secret-free
  failures.
- Independent SHA-256 and SHA-384 vectors, repeat-update evidence, and failure
  atomicity tests.
