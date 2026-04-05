# Changelog -- CodingAdventures::EcmascriptES1Lexer (Perl)

All notable changes to this package are documented here.

## [0.01] -- 2026-04-05

### Added

- Initial implementation of `CodingAdventures::EcmascriptES1Lexer`.
- `tokenize($source)` -- tokenizes an ECMAScript 1 string using rules compiled
  from the shared `ecmascript/es1.tokens` grammar file.
- Grammar cached in package-level variables for process lifetime.
- Full ES1 token set: 23 keywords, basic operators (no ===), literals.
- `BUILD` and `BUILD_windows` scripts.
- Test suite with `t/00-load.t` and `t/01-basic.t`.
