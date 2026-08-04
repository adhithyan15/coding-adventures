# Changelog — md5-native (Java)

## 0.1.0 — 2026-07-11

### Added

- Initial release: native-through-Rust MD5 for the JVM — reuses the JVM native
  pattern (Rust cdylib + jni-bridge JNI + System.loadLibrary) established by
  `java/sha256-native`, specialised to MD5's 16-byte digest.
- Calls `coding_adventures_md5` through the `md5_native_jni` cdylib: `sumMd5` /
  `hexString` and a streaming `Digest` (`AutoCloseable`, `update` /
  non-destructive `digest` / `hexDigest` / `copy`, `Cleaner`-managed handle).
- 9 JUnit tests through JNI: RFC 1321 vectors (incl. 0x00..0xFF), block
  boundaries, streaming parity, byte-at-a-time, non-destructive digest, copy
  independence, closed-handle guarding. `gradle test` green against the cdylib.
