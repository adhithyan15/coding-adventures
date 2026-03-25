# Changelog — coding_adventures_sql_execution_engine

## [0.1.0] — 2026-03-25

### Added
- Initial implementation of the SQL execution engine.
- `DataSource` module with `schema` and `scan` abstract methods.
- `SqlExecutionEngine.execute(sql, source)` and `execute_all(sql, source)` public API.
- Full SELECT pipeline: FROM → JOIN → WHERE → GROUP BY → HAVING →
  SELECT projection → DISTINCT → ORDER BY → LIMIT/OFFSET.
- Five join types: INNER, LEFT, RIGHT, FULL OUTER, CROSS.
- Expression evaluator supporting arithmetic, comparisons, BETWEEN, IN,
  LIKE (prefix/suffix % patterns), IS NULL, IS NOT NULL, AND, OR, NOT.
- Aggregate functions: COUNT(*), COUNT(col), SUM, AVG, MIN, MAX.
- `QueryResult` struct holding columns and rows.
- Error classes: `ExecutionError`, `TableNotFoundError`, `ColumnNotFoundError`.
- Comprehensive minitest test suite with 25+ assertions.
