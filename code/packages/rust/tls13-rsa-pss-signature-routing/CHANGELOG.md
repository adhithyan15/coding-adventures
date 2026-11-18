# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-18

### Added

- Added fail-closed routing from policy-authorized TLS 1.3 RSA-PSS RSAE
  schemes to the matching repository-owned SHA-256, SHA-384, or SHA-512
  signature verifier.
- Added typed digest and verifier-limit profiles so cross-profile inputs fail
  before RSA work or scratch mutation.
- Added non-copyable evidence binding the exact authorization, structurally
  decoded message, validated RSA key, digest, and fixed-verifier evidence.
- Added independently generated OpenSSL fixtures for every supported route,
  mismatch tests, fixed-verifier failure propagation, and diagnostic redaction
  coverage.
- Declared no ambient capabilities because the package performs only pure
  public-data routing and verification over caller-owned memory.
