# Changelog -- EcmascriptES1Lexer (Swift)

## [Unreleased]

### Fixed
- `BUILD_windows`: `where swift` now discards its output with cmd's `>nul
  2>nul` instead of `>/dev/null`. Under `cmd /C` the `/dev/null` redirect
  always failed, so the line always fell through to its "Swift not available"
  echo and the Swift step never ran on Windows, even on runners with Swift
  installed.
- Regenerated `_Grammar.swift` from `code/grammars/ecmascript/es1.tokens`
  after #17901 changed it without regenerating the Swift copy (the
  `swift/grammar-tools` `--check` drift guard flagged it): line comments stop at `\r` as well as `\n`, and `/***/`, `/* a **/` are block comments.

## [0.2.0] -- 2026-07-13

### Changed
- `loadGrammar()` now returns the grammar embedded at compile time in the
  generated `_Grammar.swift` instead of reading `code/grammars/**` via
  `#filePath` at run time. The lexer/parser no longer depends on the
  monorepo layout and works when published standalone. Grammar source of
  truth is unchanged; regenerate via `swift/grammar-tools`' `grammar-tools-embed`.

## [0.1.0] -- 2026-04-05

### Added

- Initial implementation of `EcmascriptES1Lexer`.
- `tokenize(_:)` -- tokenizes ES1 source using `ecmascript/es1.tokens`.
- `loadGrammar()` -- loads and parses the ES1 token grammar.
- Full ES1 token set: 23 keywords, basic operators, literals.
- Comprehensive XCTest suite.
- `BUILD` and `BUILD_windows` scripts.
- `.gitignore` with `.build/`.
