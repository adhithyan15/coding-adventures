# Changelog

## [0.1.1] - 2026-03-31

### Fixed

- `TestWhereLike` and `TestArithmetic` now pass. Both tests used string literals
  in WHERE clauses (e.g. `WHERE name LIKE 'A%'` and `WHERE name = 'Alice'`).
  The underlying SQL grammar is case-insensitive, so the Go `GrammarLexer` was
  lowercasing the entire source before tokenization — turning `'Alice'` into
  `'alice'`. This caused the WHERE predicate to never match because the in-memory
  test data stores names with original case. The fix is in the shared
  `go/lexer` package (GrammarLexer now stores an `originalSource` field and
  extracts STRING bodies from it).

## [0.1.0] — 2026-03-25

### Added

Initial release of the `sql-execution-engine` package.

**Core engine (`engine.go`)**
- `Execute(sql, source)` — parse and execute a single SELECT statement
- `ExecuteAll(sql, source)` — parse and execute multiple `;`-separated statements
- Returns `*QueryResult` with typed columns and rows
- Returns `*UnsupportedStatementError` for non-SELECT statements

**DataSource interface (`data_source.go`)**
- `DataSource` interface with `Schema(table)` and `Scan(table)` methods
- Decouples the query engine from any specific storage backend
- Supports nil (NULL), int64, float64, string, bool row values

**Execution pipeline (`executor.go`)**
- Stage 1: FROM — full table scan via `DataSource.Scan()`
- Stage 2: JOIN — nested-loop join for each `join_clause`
- Stage 3: WHERE — three-valued predicate filter
- Stage 4: GROUP BY — hash-based row partitioning
- Stage 5: HAVING — post-aggregation group filter
- Stage 6: SELECT — expression projection with aggregate support
- Stage 7: DISTINCT — duplicate row elimination
- Stage 8: ORDER BY — stable multi-key sort with NULLS LAST
- Stage 9: LIMIT/OFFSET — result set slicing for pagination

**Expression evaluator (`expression.go`)**
- `evalExpr` — recursive evaluator for the full SQL expression grammar
- OR / AND / NOT with three-valued logic (NULL propagation)
- Comparison operators: `=`, `!=`, `<>`, `<`, `>`, `<=`, `>=`
- `IS NULL`, `IS NOT NULL`
- `BETWEEN low AND high`, `NOT BETWEEN`
- `IN (list)`, `NOT IN (list)` — with SQL-correct NULL-in-list handling
- `LIKE pattern`, `NOT LIKE pattern` — `%` and `_` wildcards
- Arithmetic: `+`, `-`, `*`, `/`, `%` with NULL propagation and int64 preservation
- Unary minus
- Literals: NUMBER, STRING (single-quoted), TRUE, FALSE, NULL
- Column references: `column` and `table.column` qualified form
- `columnNotFound` error propagation through the expression chain

**Aggregate functions (`aggregate.go`)**
- `COUNT(*)` — count all rows
- `COUNT(expr)` — count non-NULL values
- `SUM(expr)` — sum of non-NULL values; returns NULL for empty set
- `AVG(expr)` — mean of non-NULL values; returns NULL for empty set
- `MIN(expr)` / `MAX(expr)` — works on numbers and strings
- All aggregates ignore NULL values except COUNT(*)

**Join types (`join.go`)**
- `INNER JOIN` — only matching rows
- `LEFT [OUTER] JOIN` — all left rows, NULLs for unmatched right
- `RIGHT [OUTER] JOIN` — all right rows, NULLs for unmatched left
- `FULL [OUTER] JOIN` — all rows from both sides
- `CROSS JOIN` — Cartesian product (requires ON clause per grammar)
- Column qualification with `tableName.colName` to avoid ambiguity

**Result type (`result.go`)**
- `QueryResult{Columns []string, Rows [][]interface{}}`
- `String()` — ASCII table rendering for debugging and REPLs

**Error types (`errors.go`)**
- `TableNotFoundError` — unknown table name
- `ColumnNotFoundError` — unknown column reference
- `UnsupportedStatementError` — non-SELECT statement
- `EvaluationError` — runtime evaluation failure

**Tests (`engine_test.go`)**
- 75 test cases covering all documented SQL features
- `InMemorySource` fixture with employees and departments tables
- Tests for all 27 specification scenarios plus additional coverage tests
- 82%+ statement coverage
