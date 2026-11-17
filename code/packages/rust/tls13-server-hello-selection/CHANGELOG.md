# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-17

### Added

- Exact count-, value-, and order-sensitive binding between a decoded
  ClientHello cipher-suite vector and its policy-bearing offer evidence.
- RFC 8446 legacy session-ID echo enforcement for ordinary and retry-shaped
  ServerHello messages.
- Original-offer membership and data-driven policy validation for the
  server-selected cipher suite.
- Non-copyable retention of both decoded messages plus the validated offer for
  later exact-evidence composition.
- Stable redacted failures and metadata-only success diagnostics.
