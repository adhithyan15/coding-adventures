# Changelog

All notable changes to this package are documented here.

## [0.2.0] - 2026-08-10

### Added

- Add a complete deterministic verified-ancestry read for security audits that
  remains bounded by the repository graph limit instead of the interactive
  history limit.

## [0.1.0] - 2026-08-09

### Added

- Opaque repository address and announcement-ID derivation.
- Ordered immutable publication with exact read-back verification.
- Signed announcement discovery and bounded commit-DAG reconstruction.
- Explicit verified commit and encrypted-object reads for application hosts.
- Caller-owned head pins, deterministic ancestry history, and conservative GC
  planning.

### Security

- Mandatory cryptographic verifier with no unchecked repository path.
- Closed payload-free diagnostics and fail-closed bounds.
- Provider withholding, full-ancestry device-counter equivocation, graph cycle,
  and corruption detection.
