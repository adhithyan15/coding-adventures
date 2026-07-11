# Changelog — sha256-native-jni

## 0.1.0 — 2026-07-11

### Added

- Initial release: JNI bridge over `coding_adventures_sha256` via `jni-bridge`.
- `nativeDigest(byte[]) -> byte[]` and an opaque `long`-pointer streaming hasher
  (`nativeHasherNew` / `Update` / `Digest` / `Clone` / `Free`).
- Used by `java/sha256-native` (`System.loadLibrary("sha256_native_jni")`).
