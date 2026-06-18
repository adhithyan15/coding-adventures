# Changelog — coding-adventures-wolfram-parser

All notable changes to this package are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
Semantic Versioning.

## [0.2.0] — 2026-06-17

### Added (W-6 — operator sugar)

- New grammar rules for the W-6 operator sugar (`code/grammars/wolfram.grammar`,
  regenerated into `_grammar.rs`):
  - `mapapply = postfix { ( MAP | APPLY ) postfix }` — a new infix precedence
    level (between `power` and `postfix`, left-associative) for `f /@ x` and
    `f @@ x`.
  - `postfix` gains the part-sugar arm
    `LDBRACKET arglist RBRACKET RBRACKET` alongside `f[…]` application, so
    `x[[i]]`, `x[[i, j]]`, `x[[1]][[2]]`, and `f[x][[1]]` all parse as postfixes.
- The double-bracket form closes with two single `RBRACKET`s (there is no `]]`
  token), so nested ordinary application `f[g[x]]` is unaffected (regression
  guarded by a parser test).

### Notes

- An empty `x[[]]` is a syntax error (unlike `f[]`), since `[[ … ]]` requires at
  least one index. See `code/specs/MA04-wolfram-language.md` §9.

## [0.1.0] — 2026-06-16

### Added

- Initial release (W-3). A parser for the Wolfram Language M-expression subset,
  a thin wrapper over the generic `GrammarParser` with the committed `_grammar.rs`
  compiled from `code/grammars/wolfram.grammar`. Exposes `parse_wolfram` /
  `try_parse_wolfram` / `create_wolfram_parser`, producing a `GrammarASTNode`
  rooted at `program`.
- The parse tree's rule names (`assignment`, `replaceall`, `rule`, `additive`,
  `multiplicative`, `power`, `postfix`, `atom`, `list`, …) are the surface forms
  the W-4 `wolfram-runtime` will lower into canonical `symbolic-ir` heads.

### Notes

- Sibling of `r-parser` / `macsyma-parser`. See `code/specs/MA04-wolfram-language.md`.
