# Changelog -- XMLLexer (Swift)

## [0.2.0] -- 2026-07-13

### Changed
- `loadGrammar()` now returns the grammar embedded at compile time in the
  generated `_Grammar.swift` instead of reading `code/grammars/**` via
  `#filePath` at run time. The lexer/parser no longer depends on the
  monorepo layout and works when published standalone. Grammar source of
  truth is unchanged; regenerate via `swift/grammar-tools`' `grammar-tools-embed`.

## [0.1.1] -- 2026-07-12

### Fixed
- `loadGrammar()` now reads the grammar from `grammars/xml/xml.tokens`.
  PR #7475 moved every grammar into a per-grammar `code/grammars/<name>/`
  subdirectory; this lexer still used the old flat `grammars/xml.tokens`
  path, so every test failed with "The file doesn't exist."

## [0.1.0] -- 2026-04-12

### Added

- Initial implementation of `XMLLexer`.
- `tokenize(_:)` -- tokenizes XML source using `xml.tokens`.
- `loadGrammar()` -- loads and parses the XML token grammar.
- Group stack parsing: default, tag, cdata, comment, pi.
- Comprehensive XCTest suite.
- `BUILD` and `BUILD_windows` scripts.
