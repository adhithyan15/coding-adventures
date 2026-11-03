## [Unreleased]

### Fixed
- `BUILD_windows`: `where swift` now discards its output with cmd's `>nul
  2>nul` instead of `>/dev/null`. Under `cmd /C` the `/dev/null` redirect
  always failed, so the line always fell through to its "Swift not available"
  echo and the Swift step never ran on Windows, even on runners with Swift
  installed.
- Regenerated `_Grammar.swift` from `code/grammars/ecmascript/es3.tokens`
  after #17901 changed it without regenerating the Swift copy (the
  `swift/grammar-tools` `--check` drift guard flagged it): line comments stop at `\r`, block comments ending in extra stars close, and a regex literal cannot hold a `\r`.

