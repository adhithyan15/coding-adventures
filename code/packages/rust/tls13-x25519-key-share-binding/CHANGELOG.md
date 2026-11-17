# Changelog

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
