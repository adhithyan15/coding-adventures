# Changelog — coding-adventures-wolfram-lexer

All notable changes to this package are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
Semantic Versioning.

## [0.3.0] — 2026-06-19

### Added (W-11 — pure-function tokens)

- `SLOTSEQ` (`##`), `HASH` (`#`), and `AMP` (`&`), for Wolfram's pure-function
  syntax. Longest-match-first: `##` is listed before `#` (so `##` is one
  `SlotSequence`, never two slots) and the existing two-char `&&` (`AND`) is
  matched before a lone `&` (`AMP`). A numbered slot `#n` lexes as `HASH`
  followed by the ordinary `NUMBER` token — there is no dedicated slot-number
  token, so the lexer's regex set is otherwise unchanged. The embedded
  `_grammar.rs` was regenerated via the Rust grammar-tools CLI.

## [0.2.0] — 2026-06-17

### Added (W-6 — operator sugar)

- New tokens for the W-6 operator sugar (`code/grammars/wolfram.tokens`,
  regenerated into `_grammar.rs`): `MAP` (`/@`), `APPLY` (`@@`), and the
  part-sugar opener `LDBRACKET` (`[[`), all under the longest-match-first
  convention (`/@` before `/.`/`/`).
- `drop_bracketed_newlines` now accounts for `[[`: the opener is one token but
  two bracket levels (it closes with two single `]`), so it adds `2` to the
  depth, keeping the count balanced. A newline inside `x[[\n i\n]]` is dropped
  like one inside `f[\n a\n]`.

### Notes

- There is **no** `]]` token — a closing `]]` lexes as two ordinary `RBRACKET`s,
  on purpose: a greedy `]]` token would mis-lex the tail of nested ordinary
  application `f[g[x]]` (two unrelated single `]`). Only the opener gets a
  dedicated token. See `code/specs/MA04-wolfram-language.md` §9.

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
