# Changelog

All notable changes to `coding_adventures_ecmascript_es3_lexer` will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial release
- `CodingAdventures::EcmascriptEs3Lexer.tokenize(source)` method that tokenizes ES3 source code
- Loads `ecmascript/es3.tokens` grammar file and delegates to `GrammarLexer`
- Supports ES3-specific strict equality operators: `===`, `!==`
- Supports ES3 error handling keywords: `try`, `catch`, `finally`, `throw`
- Supports `instanceof` keyword
- Supports regex literal tokens (`/pattern/flags`)
- All ES1 features retained (var, operators, identifiers, literals)
- Full test suite with SimpleCov coverage >= 80%
