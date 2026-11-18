# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-18

### Added

- Added scheme-bound repository-owned SHA-256, SHA-384, and SHA-512 hashing of
  exact framed TLS 1.3 `CertificateVerify` signed inputs.
- Added non-copyable hash evidence that owns the digest and retains the exact
  policy authorization and framed input.
- Added verification that accepts only retained hash evidence and delegates to
  exact data-driven RSA-PSS signature routing.
- Added independent OpenSSL fixtures for every supported signature scheme,
  input-substitution failures, pre-RSA mismatch tests, and diagnostic redaction
  coverage.
- Declared no ambient capabilities because the package performs only pure
  hashing and public verification over caller-supplied evidence and memory.
