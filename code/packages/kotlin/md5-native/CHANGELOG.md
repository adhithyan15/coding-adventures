# Changelog — md5-native (Kotlin)

## 0.1.0 — 2026-07-11

### Added

- Initial release: native-through-Rust MD5 for Kotlin, reusing the existing
  `md5_native_jni` cdylib (from `java/md5-native`) with no new Rust crate.
- `Md5Native.sumMd5` / `hexString` and a streaming `Digest` (`AutoCloseable`,
  `Cleaner`-managed handle) with `update` / non-destructive `digest` /
  `hexDigest` / `copy`.
- 8 tests through JNI: RFC 1321 vectors (incl. 0x00..0xFF), digest size,
  streaming parity, byte-at-a-time, non-destructive digest, copy independence,
  closed-handle guarding. `gradle test` green against the shared cdylib.
