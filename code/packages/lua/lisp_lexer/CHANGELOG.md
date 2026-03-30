# Changelog — coding-adventures-lisp-lexer

## 0.1.0 — 2026-03-29

Initial release.

- Grammar-driven Lisp/Scheme tokenizer using `lisp.tokens` and `GrammarLexer`.
- Emits: NUMBER, SYMBOL, STRING, LPAREN, RPAREN, QUOTE, DOT, EOF.
- Silently skips whitespace and `;` line comments.
- Accurate line/column tracking on all tokens.
- Full busted test suite covering all token types, comments, whitespace
  skipping, position tracking, composite expressions, and error cases.
