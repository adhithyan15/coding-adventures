# Changelog — coding-adventures-wolfram-parser

All notable changes to this package are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
Semantic Versioning.

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
