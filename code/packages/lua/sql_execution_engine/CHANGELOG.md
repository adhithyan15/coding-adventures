# Changelog — sql_execution_engine (Lua)

## 0.1.0 — 2026-03-31

Initial release.

- Built-in SQL tokenizer (keyword, identifier, string, number, operator, punctuation tokens)
- Recursive-descent SQL parser producing a structured AST
- Materialized pipeline executor: FROM → WHERE → GROUP BY → HAVING → SELECT → DISTINCT → ORDER BY → LIMIT/OFFSET
- Full expression evaluation with three-valued NULL logic
- Support for BETWEEN, IN, LIKE (with `%` and `_` wildcards), IS NULL, IS NOT NULL
- Aggregate functions: COUNT(*), COUNT(col), SUM, AVG, MIN, MAX
- String functions: UPPER, LOWER, LENGTH
- JOIN support (INNER, LEFT, RIGHT, FULL, CROSS) with ON clause
- DISTINCT deduplication
- `InMemoryDataSource` for tests and examples
- `execute(sql, ds)` → `true, {columns, rows}` or `false, error_message`
- `execute_all(sql, ds)` for multiple semicolon-separated statements
