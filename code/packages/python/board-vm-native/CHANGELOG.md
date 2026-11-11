# Changelog

## Unreleased

### Fixed — Windows build

- `BUILD_windows` set `PYTHONPATH` on its own line, but the build tool runs every
  BUILD line in a separate shell, so pytest could not import `board_vm_native`.
  The variable is now set on the same command line as pytest.

## 0.1.0

- Initial Python native Board VM bridge package.
