# Changelog — coding_adventures_caesar_cipher_native

## 0.1.0 — 2026-07-11

### Added

- Initial release: native-through-Rust Dart bindings for the Caesar cipher —
  the first `dart/*-native` package in the monorepo, establishing the reusable
  "Rust cdylib + C ABI + dart:ffi" pattern.
- Rust `cdylib` (`src/lib.rs`) exposing five `extern "C"` functions over the pure
  `caesar-cipher` crate: `caesar_encrypt`, `caesar_decrypt`, `caesar_rot13`,
  `caesar_frequency_analysis` (shift via out-param), and `caesar_free_string`.
  `bruteForce` is composed on the Dart side from 25 native `decrypt` calls, so
  it is correct for any input (tabs/newlines included) with no serialisation.
- Dart FFI layer (`lib/src/ffi.dart`) with library loading via
  `CAESAR_CIPHER_NATIVE_PATH`, UTF-8 marshalling, and leak-free ownership
  transfer (every returned `char*` is freed after copying to a Dart `String`).
- Public API mirroring the pure port: `encrypt` / `decrypt` / `rot13`,
  `frequencyAnalysis` → `({int shift, String plaintext})`, and `bruteForce` →
  `List<BruteForceResult>`.
- `tools/run-tests.sh` builds the release cdylib and runs the suite with the FFI
  path wired up. 5 Rust unit tests over the C ABI + 15 Dart tests asserting
  parity with the pure port (round-trips over shifts −30..30, frequency-analysis
  recovery, brute-force ordering, non-ASCII passthrough, and brute-forcing
  ciphertext containing tabs/newlines).
- Windows CI is skipped (cdylib cross-compile out of scope); Linux and macOS
  build and test.
