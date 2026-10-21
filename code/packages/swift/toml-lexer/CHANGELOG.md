# Changelog -- TOMLLexer (Swift)

## [Unreleased]

### Fixed
- Regenerated `_Grammar.swift` from `code/grammars/toml/toml.tokens` after
  #17722 changed that file without regenerating the Swift copy, which left
  `swift/grammar-tools`' `--check` drift guard (and so every Swift
  lexer/parser package) red on main. The embedded grammar now carries the
  linear-time string patterns (`ML_BASIC_STRING`, `ML_LITERAL_STRING`,
  `BASIC_STRING`, `LITERAL_STRING` refuse lone `\r`), the
  control-character-free `COMMENT`, and the TOML 1.0 number shapes (no
  leading zeros, `_` only between digits) that the other TOML lexers
  already use, with line numbers shifted to match.

## [0.2.0] -- 2026-07-13

### Changed
- `loadGrammar()` now returns the grammar embedded at compile time in the
  generated `_Grammar.swift` instead of reading `code/grammars/**` via
  `#filePath` at run time. The lexer/parser no longer depends on the
  monorepo layout and works when published standalone. Grammar source of
  truth is unchanged; regenerate via `swift/grammar-tools`' `grammar-tools-embed`.

## [0.1.1] -- 2026-07-12

### Fixed
- `loadGrammar()` now reads the grammar from `grammars/toml/toml.tokens`.
  PR #7475 moved every grammar into a per-grammar `code/grammars/<name>/`
  subdirectory; this lexer still used the old flat `grammars/toml.tokens`
  path, so every test failed with "The file doesn't exist."

## [0.1.0] -- 2026-04-12

### Added

- Initial implementation of `TOMLLexer`.
- `tokenize(_:)` -- tokenizes TOML source using `toml.tokens`.
- `loadGrammar()` -- loads and parses the TOML token grammar.
- Comprehensive XCTest suite covering TOML complexities (strings, dates, numbers, keys).
- `BUILD` and `BUILD_windows` scripts.
