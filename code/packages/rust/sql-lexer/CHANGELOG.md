# Changelog

All notable changes to this project will be documented in this file.

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
