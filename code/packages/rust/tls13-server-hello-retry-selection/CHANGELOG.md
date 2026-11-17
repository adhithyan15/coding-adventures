# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-17

### Added

- Exact HelloRetryRequest-first and ServerHello-second kind binding.
- Exact retained ClientHello random, legacy session ID, cipher-suite offer,
  and supported-version offer continuity checks.
- Policy-authorized cipher-suite and selected-version continuity checks.
- Non-copyable retention of both prior version-selection proofs with redacted
  failures and diagnostics.
