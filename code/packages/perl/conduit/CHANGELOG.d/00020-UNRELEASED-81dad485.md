## [Unreleased]

### Fixed

- `BUILD_windows` rewritten in cmd syntax. It was sh syntax (`command -v`,
  `$(...)`, `if [ ... ]; then`), which `cmd /C` rejects ("-v was unexpected at
  this time"). It now builds the extension, stages it under `lib/auto` with a
  `perl -e` one-liner (named with the PATH perl's `$Config{dlext}`), and runs
  the tests.

