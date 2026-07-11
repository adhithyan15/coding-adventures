# Changelog — coding_adventures_sha256_native

## 0.1.0 — 2026-07-11

### Added

- Initial release: native-through-Rust Dart bindings for SHA-256, reusing the
  established "Rust cdylib + C ABI + dart:ffi" pattern and extending it to
  binary byte buffers and an opaque streaming handle.
- Rust `cdylib` (`src/lib.rs`) over the pure `coding_adventures_sha256` crate:
  `sha256_digest` (writes 32 bytes into a caller buffer), `sha256_hex`
  (allocated C string) + `sha256_free_string`, and an opaque `Sha256Hasher`
  handle with `sha256_hasher_new` / `_update` / `_digest` / `_clone` / `_free`.
- Dart FFI layer (`lib/src/ffi.dart`): library loading via `SHA256_NATIVE_PATH`,
  byte-buffer marshalling, leak-free digest path (caller-owned 32-byte buffer),
  and a `NativeFinalizer`-backed handle wrapper with eager `dispose()`.
- Public API mirroring the pure port: `sha256` / `sha256Hex` and a
  `Sha256Hasher` with `update` / `digest` / `hexDigest` / `cloneHasher` /
  `dispose`.
- `tools/run-tests.sh` builds the release cdylib and runs the suite with the FFI
  path wired up. 5 Rust ABI unit tests + 14 Dart tests through FFI (FIPS 180-4
  vectors incl. one-million-'a', block boundaries, streaming parity, clone
  independence, and disposed-handle safety).
- Windows CI skipped (cdylib cross-compile out of scope); Linux and macOS build
  and test.
