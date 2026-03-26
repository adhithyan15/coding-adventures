# Changelog

All notable changes to the SQL parser package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added
- Initial release of the SQL parser thin wrapper.
- `parse_sql()` function for one-step parsing of SQL text into ASTs.
- `create_sql_parser()` factory for creating configured `GrammarParser` instances.
- Full ANSI SQL subset grammar support: SELECT, INSERT, UPDATE, DELETE,
  CREATE TABLE, DROP TABLE.
- SELECT clause features: `*`, multiple columns, `AS` aliases, `DISTINCT`, `ALL`.
- WHERE clause support with comparisons, `AND`/`OR`/`NOT`, `BETWEEN`, `IN`,
  `LIKE`, `IS NULL`, `IS NOT NULL`.
- JOIN support: `INNER JOIN`, `LEFT JOIN`, `RIGHT JOIN`, `FULL JOIN`, `CROSS JOIN`.
- Aggregate support: `GROUP BY`, `HAVING`, `ORDER BY` (ASC/DESC), `LIMIT`, `OFFSET`.
- CREATE TABLE with `IF NOT EXISTS`, column constraints (`NOT NULL`, `PRIMARY KEY`,
  `UNIQUE`, `DEFAULT`).
- DROP TABLE with `IF EXISTS`.
- Multiple semicolon-separated statements in a single `parse_sql()` call.
- Expression grammar: arithmetic, logical operators, function calls, column refs.
- Case-insensitive keyword matching (delegated to the SQL lexer).
- Produces generic `ASTNode` trees — root rule_name is `"program"`.
- `py.typed` marker for PEP 561 typing support.
- `_sql_grammar_path` module-level override for test error-path coverage.
