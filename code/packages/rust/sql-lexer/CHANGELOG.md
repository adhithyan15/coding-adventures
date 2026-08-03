# Changelog

All notable changes to this project will be documented in this file.

## [0.1.6] - Unreleased

### Added

- **`CAST` recognised as a keyword.** Added `CAST` to the case-insensitive
  keyword list so `cast(x AS t)`, `Cast(…)`, and `CAST(…)` all tokenize the same
  way. Previously only uppercase `CAST` matched the grammar's `Literal "CAST"`;
  lowercase/mixed `cast` came through as a plain `NAME` and the parser rejected
  the inner `AS`. Like SQLite, keywords are case-insensitive; this brings `CAST`
  in line with the other keywords (`SELECT`, `CASE`, …). NB: as with those, `CAST`
  now cannot be used as a bare unquoted identifier — a follow-up will make it a
  context-sensitive (soft) keyword to restore that narrow SQLite behavior.

## [0.1.5] - Unreleased

### Added

- **Hexadecimal integer-literal token `HEX_INT`.** A new token
  `HEX_INT = /0[xX][0-9A-Fa-f]+/ -> NUMBER` recognises SQLite hex integers like
  `0x1F`, `0X10`, and `0xff`. It is placed **before** `NUMBER` so first-match-wins
  takes the whole `0x…` literal — otherwise `NUMBER` would consume the leading `0`
  and leave `x1F` to mis-lex as a `NAME` (the previous behaviour, which made
  `SELECT 0x1F` a parse error). It aliases to `NUMBER`, so it flows through the
  existing grammar with no new parser rule; the planner's number-literal decoder
  recognises the `0x` prefix and decodes it as a 64-bit value (see `sql-planner`).
  A bare `0` (no `x`) still lexes as an ordinary `NUMBER` — the `[xX]` is
  mandatory. The pattern has no nested quantifier over an overlapping class, so
  there is no catastrophic-backtracking (ReDoS) risk.

## [0.1.4] - Unreleased

### Added

- **Scientific-notation exponents in the `NUMBER` token.** The pattern grew an
  optional trailing `([eE][+-]?[0-9]+)`, so `1e3`, `2.5e2`, `1.5E-3`, and `10e+2`
  now tokenise as a single `NUMBER` (previously `1e3` lexed as `1` followed by a
  `NAME` `e3`, causing a parse error). The exponent requires at least one digit
  after `e`, so ordinary subtraction (`5-3`) is unaffected — the `-` is only
  consumed as an exponent sign directly after `e`/`E`. The planner already
  decodes any exponent form to a REAL (`f64` parse succeeds where `i64` fails), so
  `typeof(1e3)` is `'real'`; no planner change was needed. The regex has no nested
  quantifier over an overlapping class, so there is no catastrophic-backtracking
  (ReDoS) risk.

## [0.1.3] - Unreleased

### Added

- **Blob-literal token `BLOB_HEX`.** The token grammar already declared
  `BLOB_HEX = /[xX]'[0-9A-Fa-f]*'/ -> BLOB` but the generated token table did
  not carry it, so `x'414243'` mis-lexed as a `NAME` (`x`) followed by a
  `STRING` (`'414243'`). The rule is now emitted, placed **before** `NAME` so
  first-match-wins picks the whole `x'…'` blob literal. It aliases to `BLOB`;
  because `TokenType` has no blob variant the lexer records the name in
  `type_name` (with `type_` falling back to `Name`), which the planner keys on.

## [0.1.2] - Unreleased

### Added

- **Bitwise operator tokens `& | ~ << >>`.** New literal tokens for bitwise AND,
  OR, NOT, and the two shifts. `<<`/`>>` are declared before the single `<`/`>`
  and `|` after `||`, so first-match-in-order scanning preserves maximal munch
  (`<<` is one token, `||` still beats a single `|`).

## [0.1.1] - Unreleased

### Fixed

- **`''` escaped single quotes** in string literals. The generated token grammar
  had a stale regex (`'([^'\\]|\\.)*'`, C-style backslash escapes) that didn't
  match the `sql.tokens` source (`'(''|[^'])*'`), so `'it''s'` tokenized as two
  adjacent strings and failed to parse. The `STRING_SQ` pattern is corrected to
  SQL semantics: `''` is a literal quote, a backslash is an ordinary character.
  (The `''` → `'` unescaping happens in sql-planner when the token becomes a
  string value.)

## [0.1.0] - 2026-03-23

### Added

- Initial release of the SQL lexer crate.
- `create_sql_lexer()` factory function returning `Result<GrammarLexer, String>` configured for SQL.
- `create_sql_lexer_with_path()` helper for custom grammar file paths (used in error-path tests).
- `tokenize_sql()` convenience function returning `Result<Vec<Token>, String>` directly.
- Loads the `sql.tokens` grammar file at runtime from the shared `grammars/` directory.
- Supports all SQL token types: keywords (50+), NAME, NUMBER, STRING, operators (=, !=, <>, <=, >=, <, >, +, -, *, /, %), and punctuation ((, ), ,, ;, .).
- Case-insensitive keyword matching: `select`, `SELECT`, and `Select` all produce `Keyword("SELECT")`.
- Both inequality spellings (`!=` and `<>`) produce type_name `NOT_EQUALS` via grammar alias.
- Line comments (`-- ...`) and block comments (`/* ... */`) are silently skipped.
- Result-returning API (unlike json-lexer) for clean error propagation.
- 28 unit tests covering: keyword normalization (case variants), numbers (integer, decimal), strings (single-quoted, with spaces), all operators, punctuation, comment skipping (line and block), NULL/TRUE/FALSE as keywords, qualified column refs, full SELECT/INSERT statements, SELECT *, semicolons, whitespace, factory function, and error paths (non-existent grammar file).
