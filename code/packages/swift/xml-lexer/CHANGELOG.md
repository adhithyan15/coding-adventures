# Changelog -- XMLLexer (Swift)

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
