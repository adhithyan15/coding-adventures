# Changelog - coding-adventures-sql-execution-engine (Rust)

## [0.1.1] - 2026-03-31

### Fixed
- **Aggregate column names normalized to uppercase**: `SUM(salary)`, `COUNT(*)`,
  etc. now appear as `"SUM(salary)"` and `"COUNT(*)"` in result row maps
  regardless of the casing in the source SQL. Previously, the SQL lexer's
  `@case_insensitive true` directive lowercased the entire source text before
  tokenizing, causing function names in column labels to be lowercase (e.g.,
  `"sum(salary)"`). This was a Windows-only failure because main-branch CI only
  runs an Ubuntu full build - the Windows runner was exposing the bug. Fixed by
  uppercasing the first token in `infer_col_name` when the expression resolves
  to a `function_call`.

## [0.1.0] - 2026-03-25

### Added
- Initial implementation of the SQL execution engine.
- `DataSource` trait with `schema()` and `scan()` methods.
- `execute()` and `execute_all()` public API.
- Full SELECT pipeline: FROM -> JOIN -> WHERE -> GROUP BY -> HAVING ->
  SELECT projection -> DISTINCT -> ORDER BY -> LIMIT/OFFSET.
- Five join types: INNER, LEFT, RIGHT, FULL OUTER, CROSS.
- Expression evaluator supporting arithmetic, comparisons, BETWEEN, IN,
  LIKE (% wildcards), IS NULL, IS NOT NULL, AND, OR, NOT.
- Aggregate functions: COUNT(*), COUNT(col), SUM, AVG, MIN, MAX.
- `QueryResult` struct with column names and rows.
- `ExecutionError` enum with `TableNotFound` and `ColumnNotFound` variants.
- `SqlValue` type alias and `SqlPrimitive` enum.
- Comprehensive integration tests covering all 22 test cases.
