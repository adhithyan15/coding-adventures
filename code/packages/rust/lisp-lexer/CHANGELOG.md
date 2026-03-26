# Changelog

## 0.1.0 — 2026-03-21

### Added
- Initial implementation of the Lisp lexer, ported from the Python `lisp-lexer` package.
- Token types: Number, Symbol, String, LParen, RParen, Quote, Dot, Eof.
- `tokenize()` function that converts Lisp source text into a token vector.
- Whitespace and comment (`;` to end-of-line) skipping.
- Support for negative number literals (`-42`).
- Support for Lisp symbol characters: `+`, `-`, `*`, `/`, `=`, `<`, `>`, `!`, `?`, `&`.
- Support for escaped characters in string literals.
- Custom `LexerError` type with position information.
- Comprehensive test suite covering atoms, operators, delimiters, expressions, and edge cases.
