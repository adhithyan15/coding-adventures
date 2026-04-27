# Changelog — ECMAScript 5 (2009) Lexer

## 0.1.0 (2026-04-04)

### Added

- Initial implementation of the ECMAScript 5 (2009) lexer
- Thin wrapper around `GrammarLexer` loading `es5.tokens`
- Public API: `create_es5_lexer(source)`, `tokenize_es5(source)`
- Comprehensive test suite covering keywords, operators, identifiers, literals
