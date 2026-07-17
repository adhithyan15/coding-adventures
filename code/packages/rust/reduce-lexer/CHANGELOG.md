# Changelog

## [0.1.0] - 2026-07-17

### Added

- Initial grammar-driven Rust REDUCE tokenizer (MA08 §2, task R-2).
- Statically linked compiled token grammar
  (`code/grammars/reduce/reduce.tokens`), covering the R-1-scoped surface
  (MA08 §3): ordinary-parenthesis function/procedure application
  (`df(x, y)`, `h(l, m)`), the single `:=` assignment operator, `=` as the
  *equation* operator (never assignment — shared with Derive/Macsyma's
  convention), comparison (`< > <= >= neq`), the word-spelled logical
  keywords `and`/`or`/`not` (lowercase reserved words — the mirror image
  of `derive-lexer`'s uppercase `AND`/`OR`/`NOT`), the `if`/`then`/`else`
  conditional keywords, `{a, b, c}` curly-brace list literals (not
  Derive's `[a,b,c]` vector literal, and not APL/J/MATLAB array syntax),
  the `.` cons operator (never ambiguous with a decimal point, since
  `NUMBER` always requires a leading digit), `<< ... >>` group-statement
  delimiters, both statement terminators `;` and `$` (kept as distinct
  `SEMI`/`DOLLAR` tokens, mirroring `macsyma-lexer`'s identical split),
  and arithmetic `+ - * / ^ **` (`^`/`**` are the same power operator per
  the manual, kept as distinct `CARET`/`POW` tokens — the parser, not this
  lexer, is where the two spellings collapse onto one production).
- No `NEWLINE` token and no post-tokenize hook: REDUCE's `;`/`$`-
  terminated statement model (manual §5.1) has no significant newlines,
  mirroring `macsyma-lexer` exactly rather than `derive-lexer`/
  `wolfram-lexer`'s worksheet-style significant-newline model.
- 17 tests covering function/procedure application, `:=` vs `=`
  disambiguation, curly-brace list literals, the cons `.` operator vs.
  decimal points, the lowercase `and`/`or`/`not`/`neq` keywords (and that
  uppercase spellings are NOT promoted — the mirror image of
  `derive-lexer`'s own case-sensitivity test), `if`/`then`/`else`
  keyword promotion, `<< >>` group-statement delimiters, `;`/`$`
  terminator distinctness, comparison/arithmetic operators, longest-match
  precedence (`:=`, `**`, `<<`/`>>`), newlines being plain whitespace, and
  case-sensitive names.
