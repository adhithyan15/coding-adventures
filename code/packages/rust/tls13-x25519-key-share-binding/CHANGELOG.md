# Changelog

## 0.2.0 — 2026-11-18

### Added

- X25519 derivation directly from the exact CH1, HelloRetryRequest, CH2, and
  final-ServerHello key-share evidence retained by the retry composer.
- A non-copyable result that retains both that complete message proof and the
  existing wipe-on-drop shared-secret owner.
- End-to-end retry-message coverage for RFC 7748 derivation, proof identity,
  redacted diagnostics, output wiping, and unchanged output on failure.

## 0.1.0 — 2026-11-17

### Added

- Exact binding between a caller's X25519 private scalar and the retained
  ClientHello public share.
- Selected-group-only composition with the exact decoded ServerHello share and
  repository-owned X25519 key-agreement boundary.
- Compiler-preserved full-scan comparison through the repository-owned fixed-
  length comparison primitive.
- Stable payload-free failures, unchanged output on failure, and inherited
  shared-secret wipe-on-drop coverage.
