# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-05-11

### Added

- `create_iso_prolog_lexer(source)` — returns a configured
  `lexer::GrammarLexer` ready to call `.tokenize()` on.
- `tokenize_iso_prolog(source)` — convenience wrapper that runs the
  lexer and returns `Vec<lexer::Token>` (with a trailing
  `TokenType::Eof`). Errors from the underlying lexer propagate as a
  panic, matching the convention of other `*-lexer` crates in this
  workspace.
- `src/_grammar.rs` — auto-generated embedding of the ISO/Core
  Prolog token grammar (`code/grammars/prolog/iso.tokens`). Regenerate
  with `cargo run -p prolog-lexer --example regenerate_grammar`.
- `examples/regenerate_grammar.rs` — reads `iso.tokens`, parses it via
  `grammar-tools::token_grammar::parse_token_grammar`, applies two
  Rust-specific post-parse transformations (see notes), compiles via
  `grammar-tools::compiler::compile_token_grammar`, and writes
  `src/_grammar.rs`. The transformations are documented in the
  generator so future maintainers see exactly what differs from the
  canonical grammar.
- 17 tests covering: empty source, whitespace and comments,
  every structural token (`LPAREN`/`RPAREN`/`COMMA`/`DOT`/`BAR`),
  the multi-character arrow tokens (`RULE`, `QUERY`, `DCG`),
  numeric literals (integer, float, scientific notation, the
  `42.` disambiguation), variables (uppercase- and `_`-led), atoms
  (lowercase, quoted, symbolic — all via the alias to `ATOM`),
  list brackets and pipe, cut, and a full small program.

### Rust-specific grammar adjustments

The canonical `iso.tokens` uses Python `re` syntax including
**negative look-ahead** (`_(?![A-Za-z0-9_])` for `ANON_VAR`). The
Rust `regex` crate does not support look-around. The
`regenerate_grammar` example applies two transformations
**after** parsing so the canonical grammar stays pristine:

1. `ANON_VAR`'s pattern is rewritten to plain `_` (no look-ahead).
2. The order of `ANON_VAR` and `VARIABLE` is swapped, so that
   `VARIABLE` (which requires at least one continuation char after
   the underscore) is tried first under the lexer's first-match-wins
   semantics. `_State` matches `VARIABLE`; `_` alone matches
   `ANON_VAR`. Semantically identical to the look-ahead version.

If `iso.tokens` ever drops look-around (e.g. through cross-impl
harmonization), the transformation can be deleted without
behavioural change.

### Architecture

Thin glue around `lexer::GrammarLexer`. Mirrors the
`algol-lexer` / `csharp-lexer` / `css-lexer` pattern in this
workspace. The Python equivalent is
`code/packages/python/iso-prolog-lexer`, which uses the same
pipeline (`grammar_tools` → `GrammarLexer`) sourced from the same
`iso.tokens` file. Token streams agree by construction.
