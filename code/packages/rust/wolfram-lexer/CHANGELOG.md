# Changelog — coding-adventures-wolfram-lexer

All notable changes to this package are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
Semantic Versioning.

## [0.1.0] — 2026-06-16

### Added

- Initial release (W-2). A tokenizer for the Wolfram Language M-expression
  subset, a thin wrapper over the generic `GrammarLexer` with the committed
  `_grammar.rs` compiled from `code/grammars/wolfram.tokens`. Exposes
  `tokenize_wolfram` / `try_tokenize_wolfram` / `create_wolfram_lexer`.
- The bracket-interior newline hook (`drop_bracketed_newlines`): a `NEWLINE`
  inside an open `(`, `[`, or `{` is dropped (unlike R, Wolfram's `{ }` is a
  list whose interior newlines are insignificant), so a group / `f[…]`
  application / `{…}` list may span lines; top-level newlines are kept as
  statement terminators.

### Notes

- Sibling of `r-lexer` / `macsyma-lexer`. See `code/specs/MA04-wolfram-language.md`.
