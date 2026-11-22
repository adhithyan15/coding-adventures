# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-11-17

### Added

- Exact extraction of retry `supported_groups` and `key_share` values from the
  four retained ClientHello, HelloRetryRequest, and ServerHello messages.
- One retained data-driven named-group policy and caller-selected decoding
  ceilings for the complete retry transition.
- Non-copyable final ServerHello share evidence borrowing the message-bound
  retry transition, with stable payload-redacted failures and diagnostics.

## [0.1.0] - 2026-11-17

### Added

- Exact HelloRetryRequest-first and ServerHello-second kind binding.
- Exact retained ClientHello random, legacy session ID, cipher-suite offer,
  and supported-version offer continuity checks.
- Exact extension-context policy continuity across both retained proofs.
- Policy-authorized cipher-suite and selected-version continuity checks.
- Non-copyable retention of both prior version-selection proofs with redacted
  failures and diagnostics.
