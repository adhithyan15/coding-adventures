# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Cipher-suite-bound derivation of the RFC 8446 `"finished"` key from an
  already-derived BaseKey.
- Fixed-memory SHA-256/SHA-384 verify-data generation over an exact transcript
  hash and compiler-preserved full-scan peer verification.
- Caller-owned non-cloneable wipe-on-drop key storage with success-only
  borrowed access, exact public-length checks, stable redacted failures, and
  success evidence bound to the verified transcript and client/server role.
- RFC 8448 SHA-256 key evidence plus independently generated SHA-256 and
  SHA-384 verify-data vectors.
