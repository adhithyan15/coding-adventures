# Changelog -- CodingAdventures::EcmascriptES3Lexer (Perl)

## [0.01] -- 2026-04-05

### Added

- Initial implementation of `CodingAdventures::EcmascriptES3Lexer`.
- `tokenize($source)` using `ecmascript/es3.tokens` grammar.
- Strict equality (===, !==), try/catch/finally/throw, instanceof support.
- `BUILD` and `BUILD_windows` scripts.
- Test suite with `t/00-load.t` and `t/01-basic.t`.
