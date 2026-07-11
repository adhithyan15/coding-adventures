# Changelog — coding_adventures_sha1_native

## 0.1.0 — 2026-07-11

### Added

- Initial release: native-through-Rust Dart bindings for SHA-1, reusing the
  "Rust cdylib + C ABI + dart:ffi" pattern with binary byte buffers and an
  opaque streaming handle (same shape as sha256-native/md5-native).
- Rust `cdylib` (`src/lib.rs`) over the pure `coding_adventures_sha1` crate:
  `sha1_digest` (writes 20 bytes into a caller buffer), `sha1_hex` + free, and
  an opaque `Digest` handle with `sha1_hasher_new` / `_update` / `_digest` /
  `_clone` / `_free`.
- Dart FFI layer with `SHA1_NATIVE_PATH` loading (absolute-path validated),
  byte-buffer marshalling, leak-free digest path, and a NativeFinalizer-backed
  handle wrapper with eager `dispose()`.
- Public API mirroring the pure port: `sum1` / `hexString` and a `Sha1Digest`
  with `update` / `sum1` / `hexDigest` / `cloneDigest` / `dispose`.
- `tools/run-tests.sh` builds the release cdylib and runs the suite. 5 Rust ABI
  unit tests + 13 Dart tests through FFI (FIPS/RFC vectors, block boundaries,
  streaming parity, clone independence, disposed-handle safety).
- Windows CI skipped (cdylib cross-compile out of scope); Linux and macOS build
  and test.
