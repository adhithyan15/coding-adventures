# Changelog -- CodingAdventures::EcmascriptES5Lexer (Perl)

## [0.01] -- 2026-04-05

### Added

- Initial implementation of `CodingAdventures::EcmascriptES5Lexer`.
- `tokenize($source)` using `ecmascript/es5.tokens` grammar.
- `debugger` keyword support (new in ES5).
- All ES3 features retained: strict equality, try/catch/finally/throw, instanceof.
- `BUILD` and `BUILD_windows` scripts.
- Test suite with `t/00-load.t` and `t/01-basic.t`.
