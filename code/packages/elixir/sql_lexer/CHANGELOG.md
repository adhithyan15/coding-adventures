# Changelog

## 0.1.0 — 2026-03-23

### Added
- `SqlLexer.tokenize_sql/1` — tokenize SQL source code into a token list
- `SqlLexer.create_sql_lexer/1` — parse sql.tokens grammar (optional custom path)
- Grammar caching via `persistent_term` for repeated use
- Case-insensitive keyword normalization — `select` → `"SELECT"`
- Support for `-- line comments` and `/* block comments */` (silently skipped)
- 50+ tests covering keywords, case normalization, identifiers, numbers, strings,
  operators, punctuation, comment skipping, compound expressions, whitespace,
  position tracking, EOF, and error cases
