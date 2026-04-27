# Changelog — ECMAScript 1 (1997) Lexer

## 0.1.0 (2026-04-04)

### Added

- Initial implementation of the ECMAScript 1 (1997) lexer
- Thin wrapper around `GrammarLexer` loading `es1.tokens`
- Public API: `create_es1_lexer(source)`, `tokenize_es1(source)`
- Comprehensive test suite covering keywords, operators, identifiers, literals
