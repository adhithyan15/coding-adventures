# Changelog — CodingAdventures::LispLexer

## 0.01 — 2026-03-29

Initial release.

- Grammar-driven Lisp/Scheme tokenizer using `lisp.tokens` and Perl's `\G` scanning.
- Emits: NUMBER, SYMBOL, STRING, LPAREN, RPAREN, QUOTE, DOT, EOF.
- Silently skips whitespace and `;` line comments.
- Accurate line/column tracking on all tokens.
- Full Test2::V0 test suite covering all token types, comments, whitespace
  skipping, position tracking, composite expressions, and error cases.
