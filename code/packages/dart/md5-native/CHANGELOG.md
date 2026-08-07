# Changelog — coding_adventures_md5_native

## 0.1.0 — 2026-07-11

### Added

- Initial release: native-through-Rust Dart bindings for MD5, reusing the
  "Rust cdylib + C ABI + dart:ffi" pattern with binary byte buffers and an
  opaque streaming handle (same shape as sha256-native).
- Rust `cdylib` (`src/lib.rs`) over the pure `coding_adventures_md5` crate:
  `md5_digest` (writes 16 bytes into a caller buffer), `md5_hex` + free, and an
  opaque `Digest` handle with `md5_hasher_new` / `_update` / `_digest` /
  `_clone` / `_free`.
- Dart FFI layer with `MD5_NATIVE_PATH` loading (absolute-path validated),
  byte-buffer marshalling, leak-free digest path, and a NativeFinalizer-backed
  handle wrapper with eager `dispose()`.
- Public API mirroring the pure port: `sumMd5` / `hexString` and an `Md5Digest`
  with `update` / `sumMd5` / `hexDigest` / `cloneDigest` / `dispose`.
- `tools/run-tests.sh` builds the release cdylib and runs the suite. 5 Rust ABI
  unit tests + 14 Dart tests through FFI (RFC 1321 vectors, little-endian
  0x00..0xFF digest, block boundaries, streaming parity, clone independence,
  disposed-handle safety).
- Windows CI skipped (cdylib cross-compile out of scope); Linux and macOS build
  and test.
