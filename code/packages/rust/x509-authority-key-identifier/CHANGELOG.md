# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-30

### Added

- Exact RFC 5280 Authority Key Identifier OID recognition and bounded
  allocation-free decoding of the non-critical keyIdentifier-only profile.
- Fail-closed rejection for empty, over-policy, critical, issuer/serial, and
  trailing forms with stable redacted errors and empty capabilities.
