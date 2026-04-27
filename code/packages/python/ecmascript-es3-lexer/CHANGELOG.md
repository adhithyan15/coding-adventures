# Changelog — ECMAScript 3 (1999) Lexer

## 0.1.0 (2026-04-04)

### Added

- Initial implementation of the ECMAScript 3 (1999) lexer
- Thin wrapper around `GrammarLexer` loading `es3.tokens`
- Public API: `create_es3_lexer(source)`, `tokenize_es3(source)`
- Comprehensive test suite covering keywords, operators, identifiers, literals
