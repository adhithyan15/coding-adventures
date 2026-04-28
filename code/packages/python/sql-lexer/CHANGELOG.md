# Changelog

All notable changes to the SQL lexer package will be documented in this file.

## [0.3.0] - 2026-04-27

### Added — Phase 4a: CHECK constraints

- `CHECK` registered as a SQL keyword in `sql.tokens` and the compiled
  `_grammar.py`, enabling `CHECK (expr)` column constraints to tokenize as
  KEYWORD rather than NAME.

## [0.2.0] - 2026-04-27

### Added
- `ALTER`, `ADD`, `COLUMN` registered as SQL keywords in `sql.tokens` and
  the compiled `_grammar.py`, enabling ALTER TABLE to tokenize correctly.

## [0.1.0] - 2026-03-23

### Added
- Initial release of the SQL lexer thin wrapper.
- `tokenize_sql()` function for one-step tokenization of ANSI SQL text.
- `create_sql_lexer()` factory for creating configured `GrammarLexer` instances.
- Full ANSI SQL token support: KEYWORD, NAME, NUMBER, STRING, all comparison
  and arithmetic operators, and punctuation (, ; . ( )).
- Case-insensitive keyword normalization via `# @case_insensitive true` in
  `sql.tokens`: `select`, `SELECT`, and `Select` all produce `KEYWORD("SELECT")`.
- Both `!=` and `<>` produce `NOT_EQUALS` tokens (NEQ_ANSI aliased).
- Compound operators `<=`, `>=` matched as single tokens (longest-match rule).
- Single-quoted string literals aliased to STRING (quotes stripped).
- Backtick-quoted identifiers aliased to NAME (backticks preserved in value).
- `--` line comments and `/* */` block comments silently skipped.

