# Changelog — sql-execution-engine (Perl)

## 0.01 — 2026-03-31

Initial release.

- Built-in SQL tokenizer and recursive-descent parser
- Materialized pipeline executor: FROM → WHERE → GROUP BY → HAVING → SELECT → DISTINCT → ORDER BY → LIMIT/OFFSET
- Three-valued NULL logic throughout
- Aggregate functions: COUNT(*), COUNT(col), SUM, AVG, MIN, MAX
- String functions: UPPER, LOWER, LENGTH
- Expressions: arithmetic, BETWEEN, IN, LIKE, IS NULL, IS NOT NULL
- JOIN support: INNER, LEFT, RIGHT, FULL, CROSS
- `InMemoryDataSource` for tests and examples
- `execute` and `execute_all` public API
