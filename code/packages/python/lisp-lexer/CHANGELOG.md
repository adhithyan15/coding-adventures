# Changelog

## 0.1.0 — 2026-03-20

### Added

- **Lisp lexer** — thin wrapper around grammar-tools GrammarLexer
- Loads `lisp.tokens` grammar file for token definitions
- `create_lisp_lexer()` factory and `tokenize_lisp()` convenience function
- Tokens: NUMBER, SYMBOL, STRING, LPAREN, RPAREN, QUOTE, DOT
- Skips WHITESPACE and COMMENT tokens
- 33 tests, 100% coverage
