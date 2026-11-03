# Changelog -- EcmascriptES3Lexer (Swift)

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

## [0.2.0] -- 2026-07-13

### Changed
- `loadGrammar()` now returns the grammar embedded at compile time in the
  generated `_Grammar.swift` instead of reading `code/grammars/**` via
  `#filePath` at run time. The lexer/parser no longer depends on the
  monorepo layout and works when published standalone. Grammar source of
  truth is unchanged; regenerate via `swift/grammar-tools`' `grammar-tools-embed`.

## [0.1.0] -- 2026-04-05

### Added

- Initial implementation of `EcmascriptES3Lexer`.
- `tokenize(_:)` -- tokenizes ES3 source using `ecmascript/es3.tokens`.
- `loadGrammar()` -- loads and parses the ES3 token grammar.
- Strict equality (===, !==), try/catch/finally/throw, instanceof support.
- Comprehensive XCTest suite.
- `BUILD` and `BUILD_windows` scripts.
- `.gitignore` with `.build/`.
