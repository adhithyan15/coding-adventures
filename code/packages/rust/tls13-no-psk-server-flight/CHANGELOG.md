# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Exact no-PSK server-flight ordering after a separately validated
  `ServerHello`.
- Optional single `CertificateRequest` handling with a retained client-
  authentication requirement.
- Failure-before-mutation for unexpected or post-completion messages.
- Stable diagnostics that retain no received message type or peer bytes.
