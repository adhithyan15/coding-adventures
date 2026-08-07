# Changelog — md5-c

## 0.1.0 — 2026-07-11

### Added
- Initial release: C ABI (staticlib + cdylib) over `coding_adventures_md5`.
- `md5_c_digest` (16-byte caller buffer) + opaque streaming hasher
  (`_new`/`_update`/`_digest`/`_clone`/`_free`). Used by `swift/md5-native`.
