# Changelog

## [0.1.0] - 2026-07-13

### Added

- Initial grammar-driven Rust Derive tokenizer (MA07 §2, task D-2).
- Statically linked compiled token grammar (`code/grammars/derive/derive.tokens`),
  covering the D-1-scoped surface (MA07 §3): ordinary-parenthesis function
  application (`DIF(u, x)`, not `f[x]`), the single `:=` assign/define
  operator (shared by both variable assignment and function definition —
  the D-3 parser disambiguates by what precedes it), `=` as the *equation*
  operator (never assignment — Derive/Macsyma's shared convention),
  comparison (`<=`/`<`/`>`/`>=`), the three boolean-algebra keywords
  `AND`/`OR`/`NOT` (case-sensitive reserved words — lowercase `and`/`or`/`not`
  lex as ordinary `NAME`s), `[...]`/`[...;...]` vector/matrix literal
  delimiters (`;` as the row separator, no other use of `;` in this subset),
  and arithmetic (`+ - * / ^`).
- Bracket-interior newline hook (`drop_bracketed_newlines`), mirroring
  `wolfram-lexer`'s: a top-level `NEWLINE` ends a worksheet expression (each
  its own line at Derive's numbered `#n:` prompt), but one inside an open
  `(` or `[` is dropped so a call or a vector/matrix literal may span
  several physical lines. Derive has no `{ }` and no `[[ ]]` part-sugar
  (unlike Wolfram), so only `(`/`[` depth is tracked.
- 13 tests covering function application, the shared `:=` token, `=` vs
  `:=` disambiguation, vector/matrix literal delimiters, the case-sensitive
  `AND`/`OR`/`NOT` keywords (and that lowercase spellings are NOT promoted),
  comparison/arithmetic operators, longest-match precedence, and
  bracket-interior newline dropping.
