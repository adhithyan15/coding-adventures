# Changelog

All notable changes to this project will be documented in this file.

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
