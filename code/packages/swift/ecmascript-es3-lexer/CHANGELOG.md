# Changelog -- EcmascriptES3Lexer (Swift)

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
