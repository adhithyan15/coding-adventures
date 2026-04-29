# Changelog — twig-lexer

## [0.1.0] — 2026-04-29

### Added

- Initial Rust implementation of the Twig lexer (TW00).
- Token kinds: `LParen`, `RParen`, `Quote`, `BoolTrue`, `BoolFalse`,
  `Integer`, `Keyword`, `Name`, `Eof`.
- `tokenize(source)` — hand-written scanner that converts Twig source
  text into a `Vec<Token>` ending with `Eof`.
- 1-indexed `(line, column)` position tracking on every token, including
  `Eof` (positioned one column past the last consumed character).
- Keyword promotion for `define`, `lambda`, `let`, `if`, `begin`,
  `quote`, `nil` — these match before `Name`, so the parser can dispatch
  on token kind directly.
- `;`-to-end-of-line comments and ASCII whitespace are silently skipped.
- Disambiguation for `-`: bare `-` lexes as `Name`; `-` followed by a
  digit starts a signed-`Integer` literal.
- `LexerError { message, line, column }` type for unexpected characters,
  including a clear path for stray `#` (must be followed by `t` or `f`).
- 35 unit tests covering atoms, parens, quotes, booleans, comments,
  multi-line position tracking, keyword promotion, and error cases.
