# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-25

### Added

- Add a bounded, caller-configured IV-length and per-key record-limit policy.
- Add non-clonable sequence state that starts at zero, refuses exhaustion, and
  has no reset path.
- Derive RFC 8446 per-record nonces into caller-owned storage only after full
  preflight validation.
- Cover network byte order, left padding, XOR behavior, policy bounds,
  exhaustion, separate direction state, and failure non-mutation.
