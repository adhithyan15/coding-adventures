# Changelog -- EcmascriptES5Lexer (Swift)

## [0.1.0] -- 2026-04-05

### Added

- Initial implementation of `EcmascriptES5Lexer`.
- `tokenize(_:)` -- tokenizes ES5 source using `ecmascript/es5.tokens`.
- `loadGrammar()` -- loads and parses the ES5 token grammar.
- `debugger` keyword support (new in ES5).
- All ES3 features retained.
- Comprehensive XCTest suite.
- `BUILD` and `BUILD_windows` scripts.
- `.gitignore` with `.build/`.
