# Changelog — sha256-c

## 0.1.0 — 2026-07-11

### Added

- Initial release: C ABI (staticlib + cdylib) over `coding_adventures_sha256`.
- `sha256_c_digest` (writes 32 bytes into a caller buffer) and an opaque
  streaming hasher (`_new` / `_update` / `_digest` / `_clone` / `_free`).
- Used by `swift/sha256-native` via compile-time C linkage.
