# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-06-16

### Added

- Initial release of the MATLAB lexer crate — item **MA-3b** of the MATLAB
  frontend on `array-runtime` (spec
  [`MA01`](../../../specs/MA01-matlab-language.md)), built the same way as the
  S/R lexers (a thin wrapper over the generic `GrammarLexer`).
- `tokenize_matlab()` / `try_tokenize_matlab()` and the `create_matlab_lexer()`
  factory.
- Embedded `matlab.tokens` grammar (`src/_grammar.rs`), generated ahead of time:
  element-wise `.`-operators (`.* ./ .\ .^ .'`) distinct from matrix `* / \ ^`,
  comparison/logical (`== ~= <= >= && ||`), the colon, `@` handles, matrix/cell
  brackets, numbers (a digit required after the dot so `3.*4` is `3 .* 4`),
  identifiers, the MATLAB keywords (incl. `end`), double-quoted strings, and `%`
  line comments.
- **The transpose/char-array disambiguation** (`'` is both the transpose
  operator and the char-array delimiter): a pre-tokenize hook resolves it by the
  preceding-token context (transpose after a value-terminator with no
  intervening whitespace; a string otherwise), leaving transpose quotes bare and
  rewriting char-array literals — `''` escape and all — to `` `N` `` backtick
  placeholders that a post-tokenize hook restores to their decoded content.
- **The inverted bracket-newline rule**: newlines are dropped inside `( )` but
  **kept** inside `[ ]`/`{ }` (where they separate matrix/cell rows) — the
  inverse of the S/R rule.
- `%{ %}` block-comment stripping and `...` line-continuation splicing
  (pre-tokenize passes).
- 19 unit tests + 1 doctest covering transpose-vs-string (incl. `A' * B'`,
  `[1 'a']`, `'it''s'`), element-wise vs matrix operators, `3.*4`, numbers,
  comparison/logical operators, matrix literals and ranges, keywords, the
  bracket-newline rule, comments, continuations, and error/EOF handling.
