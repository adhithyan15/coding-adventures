# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-18

### Added

- Selected-transcript-only `CertificateVerify` input framing for SHA-256 and
  SHA-384 TLS 1.3 cipher suites.
- Non-copyable evidence retaining the exact transcript state, snapshot, role,
  framed bytes, authorized RSA-PSS scheme, and scheme-bound digest.
- Atomic short-output rejection and redacted diagnostics.
