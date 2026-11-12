# Changelog — coding_adventures_activation_functions_native

## Unreleased

### Changed

- `BUILD_windows` now runs the real test suite on Windows through `tools\run-tests.ps1` (builds the Rust cdylib, points `dart:ffi` at the built DLL via the package's `*_NATIVE_PATH` variable, then `dart pub get` and `dart test`), mirroring `tools/run-tests.sh`, instead of skipping.

## 0.1.0 — 2026-07-11

### Added

- Initial release: native-through-Rust Dart bindings for the activation
  functions — the simplest native shape in the campaign (pure `double -> double`,
  no buffers/handles/allocation).
- Rust `cdylib` (`src/lib.rs`) exposing 12 `extern "C"` `af_*` wrappers (one per
  activation and derivative) over the pure `activation-functions` crate, via a
  small macro.
- Dart FFI layer (`lib/src/ffi.dart`) with `ACTIVATION_FUNCTIONS_NATIVE_PATH`
  loading (absolute-path validated) binding each `double(double)` symbol.
- Public API mirroring the pure port: `linear`, `sigmoid`, `relu`, `leakyRelu`,
  `tanh`, `softplus` and their derivatives.
- `tools/run-tests.sh` builds the release cdylib and runs the suite. 1 Rust ABI
  unit test + 7 Dart tests through FFI (reference values, overflow guards, tanh
  parity vs the exponential definition).
- Windows CI skipped (cdylib cross-compile out of scope); Linux and macOS build
  and test.
