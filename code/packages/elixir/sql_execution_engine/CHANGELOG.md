# Changelog

## [0.1.1] - 2026-06-02

### Fixed

- Caught up with the SQLite-compatible `sql.grammar`. The grammar grew two new
  expression-precedence layers (`collated` and `bitwise`) between `comparison`
  and `additive`, and switched `limit_clause` from bare `NUMBER` tokens to
  `signed_number` nodes. Three things broke once the engine was rebuilt:
  - `Expression.eval_rule/3` had no clause for `collated`/`bitwise`, raising
    `no function clause matching` on every `WHERE`/`SELECT` expression. Added
    transparent `collated` and `bitwise` clauses, plus the integer bitwise
    operators `& | << >>` in `apply_arith/3` using SQLite's signed-64-bit
    semantics (operands reduced mod 2**64, shift counts capped at 64 so a
    query like `1 << 1000000000` returns `0` rather than allocating a huge
    bignum).
  - `Executor.default_column_name/1` did not list `collated`/`bitwise` as
    transparent wrappers, so projected/ORDER BY columns were mislabelled
    `"expr"`. Added them to `@transparent_rules`.
  - `Executor.extract_limit/1` filtered for bare `NUMBER` tokens and crashed
    with `no case clause matching: []`. It now reads `signed_number` nodes,
    handles the SQLite `LIMIT offset, count` comma form, and treats a negative
    LIMIT as "no limit".

## [0.1.0] - 2026-03-25

### Added

- Initial release of the SELECT-only SQL execution engine.

- `SqlExecutionEngine.execute/2` — parse and execute a single SQL SELECT
  statement against a `DataSource`, returning `{:ok, %QueryResult{}}`.

- `SqlExecutionEngine.execute_all/2` — execute multiple `;`-separated SELECT
  statements, returning `{:ok, [%QueryResult{}, ...]}`.

- `DataSource` behaviour with `schema/1` and `scan/1` callbacks for pluggable
  storage backends. Any module implementing this behaviour can be queried.

- `QueryResult` struct with `columns: [String.t()]` and `rows: [[term()]]`.

- Full SELECT query pipeline:
  - `FROM` clause — scans the base table and builds per-row context maps
    with both qualified (`table.col`) and bare (`col`) key forms.
  - `JOIN` clauses — nested-loop join supporting INNER, LEFT, RIGHT, FULL,
    and CROSS join types. Outer joins produce NULL-filled rows for unmatched
    sides.
  - `WHERE` clause — expression-based row filtering with three-valued NULL
    logic (`sql_and`, `sql_or`, `sql_not` correctly propagate NULL).
  - `GROUP BY` — groups rows by expression keys; computes aggregate functions
    per group.
  - `HAVING` — filters groups by post-aggregate predicates.
  - `SELECT` projection — evaluates each select item expression and applies
    column aliases.
  - `DISTINCT` — deduplicates output rows by value equality.
  - `ORDER BY` — multi-key sort with per-column ASC/DESC direction; NULLs
    sort first (NULLS FIRST behaviour).
  - `LIMIT` / `OFFSET` — slice the output by count and starting position.

- Expression evaluator supporting:
  - Column references (bare and qualified `table.col` form)
  - Literals: NUMBER (integer and float), STRING, NULL, TRUE, FALSE
  - Arithmetic: `+`, `-`, `*`, `/`, `%` (modulo); NULL-safe (any NULL
    operand yields NULL)
  - Comparisons: `=`, `!=`, `<>`, `<`, `>`, `<=`, `>=`; NULL-safe
  - `BETWEEN low AND high`
  - `IN (value_list)`
  - `NOT BETWEEN`, `NOT IN`
  - `LIKE` pattern matching (`%` = any sequence, `_` = single character)
  - `NOT LIKE`
  - `IS NULL`, `IS NOT NULL`
  - `AND`, `OR`, `NOT` with three-valued logic

- Aggregate functions: `COUNT(*)`, `COUNT(col)`, `SUM(col)`, `AVG(col)`,
  `MIN(col)`, `MAX(col)`. All ignore NULL values except `COUNT(*)`.

- Error types:
  - `TableNotFoundError` — raised for unknown table names
  - `ColumnNotFoundError` — raised for unknown column references
  - `UnsupportedQueryError` — raised for non-SELECT statements
  - `ExecutionError` — catch-all for unexpected runtime failures

- Comprehensive test suite (65 tests) covering all supported features,
  including NULL semantics, join types, aggregates, GROUP BY + HAVING,
  three-valued logic unit tests, and error cases. 100% pass rate.
