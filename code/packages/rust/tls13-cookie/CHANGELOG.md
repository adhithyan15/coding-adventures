# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-17

### Added

- Exact bounded decoding and atomic encoding for RFC 9846 `Cookie` extension
  data.
- Non-copyable evidence that binds a HelloRetryRequest cookie to its exact
  second-ClientHello echo.
- Caller-selected resource limits, stable failures, and redacted diagnostics.
