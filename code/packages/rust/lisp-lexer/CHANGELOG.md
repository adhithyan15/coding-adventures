# Changelog

## Unreleased

### Changed
- Retired the hand-written scanner: `tokenize()` now runs the shared
  `lexer::grammar_lexer::GrammarLexer` over the checked-in
  `src/_grammar.rs` (compiled from `code/grammars/lisp/lisp.tokens`)
  and maps results back onto the unchanged public `Token`/`TokenType`/
  `LexerError` API. All 35 existing tests pass unmodified, as do all
  25 `lisp-parser` tests.
- Added `lisp_token_grammar_spec()` exposing the compiled grammar for
  tooling, plus a drift test pinning `_grammar.rs` to `lisp.tokens`.

### Fixed
- Two hand-scanner over-acceptances now agree with the spec, the
  grammar, and the Go lane: form-feed is a lex error (was skipped),
  and backslash-newline inside a string is unterminated (was scanned).

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
