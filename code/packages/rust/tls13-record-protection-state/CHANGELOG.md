# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- One-direction TLS 1.3 record-protection state bound to a success-only traffic
  key and static-IV view.
- Internal nonce-policy construction from the same selected cipher suite,
  preventing detached IV length, AEAD routing, or record-ceiling inputs.
- Monotonic sequence consumption before reservation release, with no reset and
  fail-closed per-key exhaustion.
- Non-cloneable borrowed reservations that expose only the matching key, nonce,
  AEAD metadata, tag length, and consumed sequence while retaining redacted
  diagnostics and fixed storage.
