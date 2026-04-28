# Changelog

All notable changes to the SQL lexer package will be documented in this file.

## [0.10.0] - 2026-04-28

### Added — Phase 9: SQL Triggers

- **6 new keyword tokens** — `TRIGGER`, `BEFORE`, `AFTER`, `FOR`, `EACH`, `ROW`
  added to `sql.tokens`.  `NEW` and `OLD` remain `NAME` tokens (not keywords)
  so they can still be used as table or column aliases without quoting.

## [0.9.0] - 2026-04-27

### Added — Phase 8: Window Functions (OVER / PARTITION BY)

- `OVER` and `PARTITION` registered as SQL keywords in `sql.tokens` and the
  compiled `_grammar.py`, so these words tokenize as KEYWORD rather than NAME
  when they appear in window-function expressions.

## [0.8.0] - 2026-04-27

### Added — Phase 7: SAVEPOINT / RELEASE / ROLLBACK TO

- `SAVEPOINT`, `RELEASE`, and `TO` registered as SQL keywords in `sql.tokens`
  and the compiled `_grammar.py`, so these words tokenize as KEYWORD rather
  than NAME when they appear at the start of a savepoint statement.

## [0.7.0] - 2026-04-27

### Added — Phase 6: CREATE / DROP VIEW

- `VIEW` registered as a SQL keyword in `sql.tokens` and the compiled
  `_grammar.py`, enabling `CREATE VIEW` and `DROP VIEW` to tokenize `VIEW`
  as KEYWORD rather than NAME.

## [0.6.0] - 2026-04-27

### Added — Phase 5b: Recursive CTEs

- `RECURSIVE` registered as a SQL keyword in `sql.tokens` and the compiled
  `_grammar.py`, enabling `WITH RECURSIVE name AS (...)` to tokenize `RECURSIVE`
  as KEYWORD rather than NAME.

## [0.5.0] - 2026-04-27

### Added — Phase 5a: Non-recursive CTEs

- `WITH` registered as a SQL keyword so `WITH name AS (...)` common table
  expressions tokenize `WITH` as KEYWORD rather than NAME.

## [0.4.0] - 2026-04-27

### Added — Phase 4b: FOREIGN KEY constraints

- `REFERENCES` registered as a SQL keyword so `REFERENCES table(col)` column
  constraints tokenize as KEYWORD rather than NAME.

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

