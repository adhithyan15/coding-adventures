# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Added a non-copyable required-response echo proof for the modern RFC 8954
  32-octet client nonce profile.
- Added fail-closed non-critical extension, exact OID, embedded OCTET STRING,
  1–32-octet wire-length, and complete full-scan equality checks.
- Added payload-free stable failures and a strict boundary before response
  authentication, certificate/path binding, freshness, revocation, transport,
  or concrete HTTPS readiness.
