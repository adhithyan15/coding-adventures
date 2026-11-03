## Unreleased

### Fixed

- `BUILD_windows`: `where swift` now discards its output with cmd's `>nul
  2>nul` instead of `>/dev/null`. Under `cmd /C` the `/dev/null` redirect
  always failed, so the line always fell through to its "Swift not available"
  echo and the Swift step never ran on Windows, even on runners with Swift
  installed.

### Added

- An explicit schema-v1 empty capability profile documenting that rendering is
  pure in-memory computation. Terminal output remains the caller's
  responsibility.

