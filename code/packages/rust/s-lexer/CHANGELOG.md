# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-06-15

### Added

- `PERCENT_OP` token: one token per `%…%` block (`%%`, `%/%`, `%in%`, `%o%`,
  user-defined `%name%`).
- `DOLLAR` token (`$`) for data-frame column access.
- Regenerated the embedded `_grammar.rs` accordingly.

## [0.1.0] - 2026-06-15

### Added

- Initial release of the historical Bell Labs S lexer crate.
- `tokenize_s()` and `try_tokenize_s()` entry points returning `Vec<Token>`.
- `create_s_lexer()` factory returning a configured `GrammarLexer`.
- Embedded `s.tokens` grammar (`src/_grammar.rs`), generated ahead of time —
  no runtime grammar parsing.
- Faithful historical `_` assignment operator (`UNDERSCORE` token); the `NAME`
  pattern excludes underscores accordingly.
- Assignment operators `<-`, `->`, `<<-`; comparison `== != < > <= >=`;
  arithmetic `+ - * / ^`; the sequence operator `:`; grouping/index/block
  delimiters; and the named-argument `=`.
- Reserved control keywords (`if else for while repeat function break next in`)
  and constants (`TRUE FALSE T F NULL NA Inf NaN`), promoted case-sensitively.
- Both `"..."` and `'...'` string literals; `#` line comments.
- `drop_bracketed_newlines` post-tokenize hook: newlines inside `( )` / `[ ]`
  are dropped (insignificant), while newlines inside `{ }` and at top level are
  kept as statement terminators.
