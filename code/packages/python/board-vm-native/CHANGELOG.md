# Changelog

## Unreleased

### Fixed — Windows build

- Integer arguments are read with `PyLong_AsLongLong` instead of `PyLong_AsLong`.
  C `long` is 32 bits on Windows, so any `u32` above `i32::MAX` (a random
  `host_nonce`) raised `TypeError: host_nonce must be an integer`.
- Tests compare the Cargo workspace `cwd` with `str(pathlib.Path(...))`, so the
  `/repo/...` fixture matches the backslash form `pathlib` produces on Windows.
- `BUILD_windows` set `PYTHONPATH` on its own line, but the build tool runs every
  BUILD line in a separate shell, so pytest could not import `board_vm_native`.
  The variable is now set on the same command line as pytest.

## 0.1.0

- Initial Python native Board VM bridge package.
