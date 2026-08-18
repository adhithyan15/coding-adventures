# Changelog — coding-adventures-lisp-lexer

## Unreleased

### Fixed

- Eliminated runtime grammar loading: `tokenize` now requires a pre-compiled
  `_grammar` module instead of reading and parsing the `lisp.tokens` file
  from `code/grammars/` on every call. The old code walked out of the
  installed package's own directory to a monorepo-relative path that a
  published LuaRocks package does not ship.

## 0.1.0 — 2026-03-29

Initial release.

- Grammar-driven Lisp/Scheme tokenizer using `lisp.tokens` and `GrammarLexer`.
- Emits: NUMBER, SYMBOL, STRING, LPAREN, RPAREN, QUOTE, DOT, EOF.
- Silently skips whitespace and `;` line comments.
- Accurate line/column tracking on all tokens.
- Full busted test suite covering all token types, comments, whitespace
  skipping, position tracking, composite expressions, and error cases.
