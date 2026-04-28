# Changelog — coding_adventures_sql_execution_engine

## Unreleased

### Fixed

- `IN` and `NOT IN` expression evaluation now locates the parsed
  `value_list` node directly, so multi-value predicates evaluate every listed
  value instead of only the first one.

## [0.1.1] - 2026-03-31

### Fixed

- **Aggregate column names now use uppercase function names**: When a query
  contained an aggregate like `SUM(salary)`, the output column was labelled
  `sum(salary)` (lowercase) because the SQL lexer normalises keywords to
  lowercase in case-insensitive mode. The SELECT projection now unwraps the
  deep pass-through rule chain (`expr` → `or_expr` → ... → `primary`) to
  find the underlying `function_call` node, then uppercases the function name
  token to produce `SUM(salary)` instead of `sum(salary)`. Test
  `test_group_by_sum` now passes.
- **Qualified column references (e.g. `employees.name`) now return their full
  dotted text** instead of just the last name part, preserving the column
  lookup key used by JOIN queries.

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
