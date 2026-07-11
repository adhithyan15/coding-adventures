# Changelog — sha256-native (Kotlin)

## 0.1.0 — 2026-07-11

### Added

- Initial release: native-through-Rust SHA-256 for Kotlin — establishes the
  Kotlin JVM native pattern, **reusing the existing `sha256_native_jni` cdylib**
  (from `java/sha256-native`) with no new Rust crate and no workspace change.
- `Sha256Native.sha256` / `sha256Hex` and a streaming `Hasher` (`AutoCloseable`,
  `Cleaner`-managed native handle) with `update` / non-destructive `digest` /
  `hexDigest` / `copy`.
- Kotlin `object Native` `external` functions resolve to the same
  `Java_com_codingadventures_sha256native_Native_*` JNI exports as the Java
  binding.
- 8 tests through JNI: FIPS 180-4 vectors (incl. one-million-'a'), digest size,
  streaming parity, byte-at-a-time, non-destructive digest, copy independence,
  closed-handle guarding. `gradle test` green against the shared cdylib.
