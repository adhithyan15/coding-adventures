# Changelog

All notable changes to `coding_adventures_ecmascript_es5_lexer` will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial release
- `CodingAdventures::EcmascriptEs5Lexer.tokenize(source)` method that tokenizes ES5 source code
- Loads `ecmascript/es5.tokens` grammar file and delegates to `GrammarLexer`
- Supports ES5-specific `debugger` keyword (promoted from future-reserved in ES3)
- `get` and `set` remain NAME tokens (contextual, not keywords)
- All ES3 features retained (strict equality, try/catch, regex literals)
- All ES1 features retained (var, operators, identifiers, literals)
- Full test suite with SimpleCov coverage >= 80%
