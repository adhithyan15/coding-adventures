# Changelog — sha256-native (Java)

## 0.1.0 — 2026-07-11

### Added

- Initial release: native-through-Rust SHA-256 for the JVM — establishes the
  "Rust cdylib + jni-bridge JNI + System.loadLibrary" JVM native pattern.
- Calls `coding_adventures_sha256` through the `sha256_native_jni` cdylib:
  `sha256(byte[])` / `sha256Hex(byte[])` and a streaming `Hasher` (an
  `AutoCloseable` with `update` / non-destructive `digest` / `hexDigest` /
  `copy`), backed by an opaque `long` peer pointer freed via `close()` with a
  `Cleaner` safety net.
- `build.gradle.kts` sets `-Djava.library.path` to the Rust `target/release`.
- 9 JUnit tests through JNI: FIPS 180-4 vectors (incl. one-million-'a'), block
  boundaries, streaming parity, byte-at-a-time, non-destructive digest, copy
  independence, and closed-handle guarding. `gradle test` green against the
  linked Rust cdylib.
