# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-17

### Added

- Exact context-policy validation for retained ClientHello, ServerHello, and
  HelloRetryRequest extension blocks.
- Mandatory `supported_versions` enforcement in both retained messages.
- Exact count-, order-, value-, and encoded-length-sensitive binding between
  the ClientHello extension and caller-supplied version offer.
- Offered-version and exact TLS 1.3 validation for the server selection.
- Non-copyable retention of the prior selection proof with redacted failures
  and diagnostics.
