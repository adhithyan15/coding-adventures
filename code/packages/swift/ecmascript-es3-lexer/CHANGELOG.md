# Changelog -- EcmascriptES3Lexer (Swift)

## [0.1.0] -- 2026-04-05

### Added

- Initial implementation of `EcmascriptES3Lexer`.
- `tokenize(_:)` -- tokenizes ES3 source using `ecmascript/es3.tokens`.
- `loadGrammar()` -- loads and parses the ES3 token grammar.
- Strict equality (===, !==), try/catch/finally/throw, instanceof support.
- Comprehensive XCTest suite.
- `BUILD` and `BUILD_windows` scripts.
- `.gitignore` with `.build/`.
