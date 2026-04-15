# Changelog -- XMLLexer (Swift)

## [0.1.0] -- 2026-04-12

### Added

- Initial implementation of `XMLLexer`.
- `tokenize(_:)` -- tokenizes XML source using `xml.tokens`.
- `loadGrammar()` -- loads and parses the XML token grammar.
- Group stack parsing: default, tag, cdata, comment, pi.
- Comprehensive XCTest suite.
- `BUILD` and `BUILD_windows` scripts.
