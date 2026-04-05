# Changelog -- EcmascriptES1Lexer (Swift)

## [0.1.0] -- 2026-04-05

### Added

- Initial implementation of `EcmascriptES1Lexer`.
- `tokenize(_:)` -- tokenizes ES1 source using `ecmascript/es1.tokens`.
- `loadGrammar()` -- loads and parses the ES1 token grammar.
- Full ES1 token set: 23 keywords, basic operators, literals.
- Comprehensive XCTest suite.
- `BUILD` and `BUILD_windows` scripts.
- `.gitignore` with `.build/`.
