## [Unreleased] — Windows runs get an 8 MiB stack and report the exit code

### Fixed

- **MSVC-built test executables now link with `/STACK:8388608`.** MSVC defaults
  to a 1 MiB main-thread stack against 8 MiB on Linux and macOS, so tests with
  deep recursion or large automatic buffers could crash only on Windows.

### Changed

- **`RUNTIME FAILURE` now names the exit code**, in decimal and hex
  (`0xC00000FD` is a stack overflow, `0xC0000005` an access violation), so a
  Windows crash can be diagnosed from the log instead of a bare failure.
