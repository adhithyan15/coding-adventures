# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Exact borrowed decoding for the TLS 1.3 `CertificateVerify` message body.
- Failure-atomic encoding into caller-owned output.
- Full zero-through-65,535-byte signature-vector syntax with exact-length checks.
- Stable redacted errors and no normal dependencies or external effects.
