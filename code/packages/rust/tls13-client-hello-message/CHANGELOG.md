# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-11-17

### Fixed

- Accept RFC 9846's corrected seven-byte minimum ClientHello extension-vector
  payload, with an exact one-version `supported_versions` regression test.

## [0.1.0] - 2026-10-29

### Added

- Exact borrowed TLS 1.3 ClientHello body decoding.
- Bounded cipher-suite and extension framing with fixed legacy-field checks.
- Payload-free failures and byte-redacted message and iterator diagnostics.
