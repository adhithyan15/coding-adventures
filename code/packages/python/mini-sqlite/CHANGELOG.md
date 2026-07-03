# Changelog

## [2.29.0] - 2026-06-30

### Added

- **Level 1 conformance fixtures (17–24)** — eight new fixtures in
  `code/specs/mini-sqlite-conformance/fixtures/` covering:

  | # | Name | What it tests |
  |---|------|---------------|
  | 17 | null-aggregate-semantics | COUNT(\*)/COUNT(col) on empty tables; SUM/AVG/MIN/MAX → NULL; NULL-skipping |
  | 18 | string-functions | LENGTH, UPPER, LOWER, SUBSTR, TRIM, LTRIM, RTRIM, REPLACE |
  | 19 | math-functions | ABS (int/float/NULL), ROUND (0/2 decimals, half-away-from-zero) |
  | 20 | limit-edge-cases | LIMIT 0, large OFFSET past end, LIMIT -1, LIMIT with OFFSET |
  | 21 | distinct-aggregate | COUNT(DISTINCT v) with duplicates and NULLs |
  | 22 | string-concat-null | `||` operator; NULL propagation; COALESCE with 2–3 arguments |
  | 23 | null-in-order-by | NULL sort order (FIRST in ASC, LAST in DESC) and IS NOT NULL filter |
  | 24 | having-aggregate | HAVING COUNT(\*) > N, HAVING SUM >= threshold, compound HAVING AND |

  `manifest.json` updated to version `1.1.0` to reflect the new level.

- **Parametrised conformance test runner** (`tests/test_conformance.py`) —
  reads `manifest.json` and auto-generates one pytest test per fixture.
  Supports all op types: `execute`, `executemany`, `query`, `expect_error`,
  `commit`, `rollback`, `fetchone_test`, `fetchmany_test`, `fetchall_test`,
  `fetchall_empty_test`, `connect_expect_error`.  Adding a new fixture JSON
  and listing it in the manifest is sufficient to have it exercised.

### Fixed

- **`cursor.description` is now set correctly after `SELECT … LIMIT 0`**
  (and any other zero-row SELECT).  PEP 249 requires `cursor.description`
  to be a tuple of column-name seven-tuples after any DQL statement,
  regardless of whether it returns rows.

  Root cause: the cursor previously used `if result.columns:` to decide
  whether the statement was a SELECT.  For `LIMIT 0` (and for any SELECT
  that returns no rows) the optimizer produces an `EmptyResult` node; the
  codegen emitted `SetResultSchema(columns=())` so `result.columns` was an
  empty tuple — falsy — and the cursor wrongly took the DML/DDL branch,
  setting `description = None`.

  The fix spans two layers:

  1. **`sql-optimizer` `DeadCodeElimination`** — `Limit(count=0)` now
     produces `EmptyResult(columns=_schema_of_plan(inner))` so the column
     schema is preserved through the optimizer.  A new `_schema_of_plan`
     helper walks the inner plan to the nearest `Project` node.

  2. **`cursor.py`** — the branch condition changed from `if result.columns:`
     to `if result.rows_affected is None or result.columns:`.  This
     correctly handles three cases:
     - SELECT (rows_affected=None) — always a result set, even with 0 rows.
     - DML + RETURNING (rows_affected≥1, columns non-empty) — exposed as a
       result set, matching the real sqlite3 module's behaviour.
     - DML/DDL without RETURNING (rows_affected≥1, columns=()) — treated as
       a mutation with no result set.

## [2.28.0] - 2026-06-19

### Added

- **`CREATE TRIGGER IF NOT EXISTS` is now accepted** — SQLite allows an
  optional `IF NOT EXISTS` guard on `CREATE TRIGGER`:

  ```sql
  CREATE TRIGGER IF NOT EXISTS trg AFTER INSERT ON t
  FOR EACH ROW BEGIN INSERT INTO log VALUES (1); END
  ```

  Without the guard, creating a trigger whose name already exists raises
  an error as before.  With the guard the statement is silently ignored,
  leaving the original trigger intact.

  The fix propagates through the full pipeline:

  | Layer      | File                             | Change                                  |
  |------------|----------------------------------|-----------------------------------------|
  | Grammar    | `sql.grammar`                    | `[ "IF" "NOT" "EXISTS" ]` added to rule |
  | AST        | `sql_planner/ast.py`             | `CreateTriggerStmt.if_not_exists` field |
  | Plan       | `sql_planner/plan.py`            | `CreateTrigger.if_not_exists` field     |
  | Planner    | `sql_planner/planner.py`         | forwards field through plan node        |
  | IR         | `sql_codegen/ir.py`              | `CreateTriggerDef.if_not_exists` field  |
  | Compiler   | `sql_codegen/compiler.py`        | destructures and forwards field         |
  | VM         | `sql_vm/vm.py`                   | catches `TriggerAlreadyExists` silently |
  | Adapter    | `mini_sqlite/adapter.py`         | detects keyword sequence, sets flag     |

  16 new oracle-style tests in `test_tier3_trigger_if_not_exists.py`
  cover all six trigger forms (BEFORE/AFTER × INSERT/UPDATE/DELETE),
  idempotent duplicate semantics, and the original-trigger-survives
  invariant.

- **`UPDATE OR <conflict>` conflict resolution** — the five conflict-resolution
  strategies that SQLite supports for `INSERT` now also work with `UPDATE`:

  ```sql
  UPDATE OR IGNORE t SET id = 1 WHERE id = 2  -- skip row if constraint violated
  UPDATE OR REPLACE t SET id = 1 WHERE id = 2 -- delete conflicting rows, then update
  UPDATE OR ABORT t SET id = 1 WHERE id = 2   -- raise (same as no modifier)
  UPDATE OR FAIL t SET id = 1 WHERE id = 2    -- raise (same as ABORT here)
  UPDATE OR ROLLBACK t SET id = 1 WHERE id = 2 -- raise (same as ABORT here)
  ```

  Implementation spans the full pipeline:

  - **Grammar** (`sql.grammar`): `update_stmt` gains an optional
    `[ conflict_clause ]` immediately after the `UPDATE` keyword, reusing the
    same rule already used by `INSERT`.
  - **Adapter** (`mini_sqlite/adapter.py`): `_update()` extracts the conflict
    action via the shared `_conflict_action()` helper.
  - **AST/Planner** (`sql-planner`): `UpdateStmt` and the `Update` plan node
    each gain an `on_conflict: str | None` field.  The planner forwards the
    field through without transformation.
  - **Constant-folding optimizer** (`sql-optimizer`): the `Update` pattern
    match now captures and preserves `on_conflict` so it is not silently
    dropped during folding.
  - **Codegen** (`sql-codegen`): the `UpdateRows` IR node gains `on_conflict`;
    the compiler passes it through from the plan.
  - **VM** (`sql-vm`): `_do_update()` dispatches on `on_conflict`:
    - `IGNORE` — if `_check_constraints` raises `ConstraintViolation`, the
      current row is silently skipped and the loop continues.  `rows_affected`
      counts only rows that were actually changed.
    - `REPLACE` — any other row whose unique-column values conflict with the
      post-update merged row is deleted before the current row is updated in
      place.  The scan cursor's index is corrected for each pre-cursor
      deletion so the outer scan loop resumes correctly.
    - `ABORT`, `FAIL`, `ROLLBACK` — raise `ConstraintViolation` (same as the
      default behaviour; full transactional semantics are deferred).

  21 oracle tests in `tests/test_tier3_update_or_conflict.py` compare
  mini-sqlite output byte-for-byte against `stdlib sqlite3`.

## [2.27.0] - 2026-06-19

### Fixed

- **Column-level `ON CONFLICT` clause now accepted in `CREATE TABLE`** —
  SQLite allows each `NOT NULL`, `UNIQUE`, and `PRIMARY KEY` column
  constraint to carry its own conflict-resolution policy:

  ```sql
  CREATE TABLE t (x INT NOT NULL ON CONFLICT IGNORE, y TEXT)
  CREATE TABLE t (x INT UNIQUE ON CONFLICT REPLACE, y TEXT)
  CREATE TABLE t (x INT PRIMARY KEY ON CONFLICT ABORT, y TEXT)
  CREATE TABLE t (x INTEGER PRIMARY KEY AUTOINCREMENT ON CONFLICT REPLACE, y TEXT)
  ```

  Previously all such forms raised a parse error.  The fix adds an optional
  nested `col_conflict_clause` sub-rule to `col_constraint` in `sql.grammar`.
  Using a *nested* sub-rule is deliberate: the adapter's keyword-sequence
  matching in `_col_def` collects only the *direct* keyword children of each
  `col_constraint` node, so the `ON / CONFLICT / action` tokens stay inside
  the sub-node and existing logic (`NOT NULL → not_null=True`, `UNIQUE →
  unique=True`, etc.) is completely unaffected.

  Mini-sqlite always uses `ABORT` semantics for constraint violations; the
  per-column action is parsed and silently ignored.

  | SQL form                                              | Before      | After |
  |-------------------------------------------------------|-------------|-------|
  | `x INT NOT NULL ON CONFLICT IGNORE`                   | Parse error | OK    |
  | `x INT NOT NULL ON CONFLICT REPLACE`                  | Parse error | OK    |
  | `x INT UNIQUE ON CONFLICT ABORT`                      | Parse error | OK    |
  | `x INT PRIMARY KEY ON CONFLICT FAIL`                  | Parse error | OK    |
  | `x INT PRIMARY KEY AUTOINCREMENT ON CONFLICT REPLACE` | Parse error | OK    |
  | All five actions: ROLLBACK, ABORT, FAIL, IGNORE, REPLACE | Parse error | OK |

- **26 new oracle tests** in `test_tier3_col_conflict_clause.py` cover
  all three constraint types × all five conflict actions, mixed
  multi-column tables, COLLATE coexistence, table-level constraint
  coexistence, and WITHOUT ROWID.

## [2.26.0] - 2026-06-16

### Fixed

- **Table-level `PRIMARY KEY` and `UNIQUE` constraints now accepted in
  `CREATE TABLE`** — SQLite allows constraints to appear after the column
  list instead of (or in addition to) column-level constraint keywords:

  ```sql
  CREATE TABLE t (x INT, y INT, PRIMARY KEY(x))
  CREATE TABLE t (x INT, y INT, UNIQUE(x, y))
  CREATE TABLE t (x INT, y INT, PRIMARY KEY(x, y))
  CREATE TABLE t (x INT, y INT, CHECK(x > 0))
  CREATE TABLE orders (id INT, cid INT, FOREIGN KEY(cid) REFERENCES c(id))
  ```

  Previously these raised a parse error because the grammar did not define
  `table_constraint` at all.  Three-part fix:

  1. `sql.grammar`: Added `table_constraint` rule after `col_def` in
     `create_table_stmt` (`{ "," table_constraint }`) covering `PRIMARY KEY`,
     `UNIQUE`, `CHECK`, and `FOREIGN KEY` variants.
  2. `adapter._create_table`: After building the `cols` tuple, iterates
     `table_constraint` child nodes; `PRIMARY KEY(col)` and `UNIQUE(col)` for
     a single named column promote `primary_key=True` / `unique=True` on the
     matching `ColumnDef` via `dataclasses.replace`.  Composite (multi-column)
     constraints and `CHECK`/`FOREIGN KEY` are parsed and silently accepted —
     mini-sqlite has no multi-column constraint representation in `ColumnDef`.
  3. `dataclasses.replace` added to the `from dataclasses import …` line.

  | SQL form                                          | Before      | After |
  |---------------------------------------------------|-------------|-------|
  | `CREATE TABLE t (x INT, y INT, PRIMARY KEY(x))`  | Parse error | OK    |
  | `CREATE TABLE t (x INT, y INT, UNIQUE(x, y))`    | Parse error | OK    |
  | `CREATE TABLE t (x INT, y INT, PRIMARY KEY(x,y))`| Parse error | OK    |
  | `CREATE TABLE t (x INT, y INT, CHECK(x > 0))`    | Parse error | OK    |
  | `… WITHOUT ROWID` with table-level PK            | Parse error | OK    |
  | `… STRICT` with table-level PK                   | Parse error | OK    |

- **21 new oracle tests** in ``test_tier3_table_constraints.py`` cover
  single/multi-column PKs, single-column UNIQUE, CHECK, FOREIGN KEY,
  mixed column-level and table-level constraints, WITH ROWID, and STRICT.

## [2.25.0] - 2026-06-16

### Fixed

- **`SELECT expr alias` (bare alias without `AS`) now accepted** — SQLite
  allows column aliases in `SELECT` items without the `AS` keyword:
  `SELECT 1 x` is equivalent to `SELECT 1 AS x`.  Previously mini-sqlite
  required `AS` and raised a parse error on the bare-alias form:
  ```
  Parse error: Expected STAR or "/" or "%" or "+" or "-", got 'x'
  ```
  Two-part fix:
  1. `sql.grammar` line `select_item`: `[ "AS" NAME ]` → `[ [ "AS" ] NAME ]`
  2. `adapter._select_item`: extended to recognise a direct `NAME` child
     that is not preceded by `AS` as the alias.

  `NAME` never matches SQL keywords (`FROM`, `WHERE`, `GROUP`, …) so there
  is no ambiguity — the grammar's PEG tokeniser emits those as `KEYWORD`
  tokens which cannot satisfy the `NAME` alternative.

  Several previously failing constructs now work correctly:

  | SQL form                                        | Before      | After |
  |-------------------------------------------------|-------------|-------|
  | `SELECT 1 x`                                    | Parse error | `1`   |
  | `SELECT a + b total FROM t`                     | Parse error | OK    |
  | `SELECT group_concat(x, \|) FROM (SELECT 1 x …)`| Parse error | OK    |
  | `SELECT expr AS alias` (with AS)                | OK          | OK    |

- **14 new oracle tests** in ``test_tier3_select_bare_alias.py`` verify
  the bare-alias form against the real sqlite3 reference engine.

## [2.24.0] - 2026-06-16

### Fixed

- **`CREATE TRIGGER` without `FOR EACH ROW` now accepted** — SQLite makes
  the `FOR EACH ROW` clause optional (SQLite has never supported
  statement-level triggers, so the clause is redundant and permitted-but-not-
  required).  Previously mini-sqlite required `FOR EACH ROW` and raised a
  parse error when it was omitted:
  ```
  Parse error at 1:1: Expected program, got 'CREATE'
  ```
  The fix is a one-character grammar change: `"FOR" "EACH" "ROW"` →
  `[ "FOR" "EACH" "ROW" ]`.  The adapter ignores the clause either way
  because it scans by keyword type, and FOR/EACH/ROW are not emitted as
  KEYWORD tokens.

  | Trigger syntax                   | Before            | After     |
  |----------------------------------|-------------------|-----------|
  | `… FOR EACH ROW BEGIN … END`     | OK (correct)      | OK        |
  | `… BEGIN … END` (no FOR EACH ROW)| Parse error       | OK        |

- **12 new oracle tests** in ``test_tier3_trigger_for_each_row_optional.py``
  verify that triggers fire correctly with and without `FOR EACH ROW`, and
  that both forms match the real sqlite3 reference engine output.

## [2.23.0] - 2026-06-16

### Fixed

- **`INTERSECT ALL` and `EXCEPT ALL` now raise `OperationalError: near "ALL": syntax error`** —
  SQLite does not support bag semantics for `INTERSECT` or `EXCEPT`; only
  `UNION ALL` is valid.  Previously mini-sqlite silently accepted these two
  forms and returned results (treating them like plain `INTERSECT`/`EXCEPT`),
  diverging from real SQLite behaviour.

  Root cause: the SQL grammar accepts `[ "ALL" ]` for all three set operators
  so the PEG parser can produce a meaningful token stream; the adapter
  (`_set_op_clause`) is now the enforcement point.  When it detects
  `INTERSECT ALL` or `EXCEPT ALL`, it raises `OperationalError` with the same
  message byte-for-byte as the real engine.

  | SQL form            | Before       | After                              |
  |---------------------|--------------|------------------------------------|
  | `UNION ALL`         | OK (correct) | OK (unchanged)                     |
  | `INTERSECT ALL`     | returns rows | `OperationalError: near "ALL": …`  |
  | `EXCEPT ALL`        | returns rows | `OperationalError: near "ALL": …`  |

- **16 new oracle tests** in ``test_tier3_intersect_except_all_rejected.py``
  verify that both operators raise the correct error type and message, that
  plain `INTERSECT`/`EXCEPT` (without `ALL`) continue to work, and that
  `UNION ALL` is unaffected.

## [2.22.0] - 2026-06-16

### Fixed

- **`SELECT * FROM (SELECT 1, 2)` now returns `(1, 2)` instead of `(2,)`** —
  when a derived-table subquery contains unnamed literal columns (e.g.
  ``SELECT 1, 2``), the column names are now ``"1"`` and ``"2"`` (matching
  SQLite's surface representation) instead of ``"?"`` for every column.

  Root cause: ``_column_display_name()`` in sql-codegen returned ``None``
  for ``Literal`` nodes, so every constant projection fell back to the
  placeholder ``"?"``.  Two identical ``"?"`` keys in the same result set
  caused ``dict(zip(cols, row))`` inside the VM's ``_do_run_subquery`` to
  drop all columns but the last.  (sql-codegen v1.43.0)

  The fix applies to all literal types:

  | SQL expression | Column name before | Column name after |
  |----------------|--------------------|-------------------|
  | ``SELECT 1``   | ``"?"``            | ``"1"``           |
  | ``SELECT 1, 2``| ``"?", "?"``       | ``"1", "2"``      |
  | ``SELECT NULL``| ``"?"``            | ``"NULL"``        |
  | ``SELECT 'hi'``| ``"?"``            | ``"'hi'"``        |

- **21 new oracle tests** in ``test_tier3_subquery_literal_colnames.py``
  cover column-name accuracy and row-value correctness for all literal
  types, with byte-for-byte comparison against stdlib ``sqlite3``.
- **10 new unit tests** in the sql-codegen test suite cover
  ``_column_display_name`` directly, pushing sql-codegen coverage to
  **80.48 %** (from the borderline pre-existing 79.61 %).

## [2.21.0] - 2026-06-16

### Fixed

- **`PARTITION BY` + external `ORDER BY` column no longer crashes** —
  queries like ``SELECT grp, SUM(val) OVER (PARTITION BY grp) FROM t
  ORDER BY grp, val`` previously raised ``InternalError: ValueError:
  tuple.index(x): x not in tuple`` because ``val`` was projected away
  by ``ComputeWindowFunctions`` before ``SortResult`` could look it up.
  Fixed by extending hidden-column injection in sql-codegen to cover
  ``PlanWindowAgg`` inner nodes (was only ``Project``).  (sql-codegen
  v1.42.0)

- **RANGE mode peer-group expansion for cumulative window functions** —
  ``COUNT(*)``, ``SUM``, ``AVG``, and other aggregate window functions
  with a default ``ORDER BY`` frame now correctly include all tied rows
  in the current row's frame.  Under ``RANGE BETWEEN UNBOUNDED
  PRECEDING AND CURRENT ROW`` (the SQL default when ``ORDER BY`` is
  present), ``CURRENT ROW`` means the *end of the peer group*, not just
  the physical row position.  Fixes wrong counts/sums when ``ORDER BY``
  values repeat.  (sql-vm v1.60.0)

- 15 new oracle tests in ``test_tier3_window_correctness.py`` cover
  both fixes with byte-for-byte comparison against stdlib ``sqlite3``.

## [2.20.0] - 2026-06-16

### Added

- **Named WINDOW clause** — ``SELECT`` statements may now define window
  specifications by name in a trailing ``WINDOW`` clause and reference
  them with ``OVER <name>`` instead of an inline ``OVER (...)`` spec:

  ```sql
  SELECT a, ROW_NUMBER() OVER w
  FROM   t
  WINDOW w AS (PARTITION BY grp ORDER BY a)
  ORDER  BY a;
  ```

  - Multiple named windows in one query are supported
    (``WINDOW w1 AS (...), w2 AS (...)``).
  - Named and inline ``OVER (...)`` references may be mixed freely in the
    same query.
  - All existing window functions work via a named window: ``ROW_NUMBER``,
    ``RANK``, ``DENSE_RANK``, ``SUM``, ``COUNT(*)``, ``AVG``, ``MIN``,
    ``MAX``, ``LAG``, ``LEAD``, ``NTILE``, ``FIRST_VALUE``,
    ``LAST_VALUE``.
  - Referencing an undefined window name raises ``OperationalError``.

  **Implementation:** ``WINDOW`` was added to ``sql.tokens`` so the lexer
  recognises it as a keyword.  ``sql.grammar`` gained a ``window_clause``
  rule and a ``window_name_ref`` alternative in ``window_func_call``.
  The adapter's ``_PlaceholderCounter`` gained a ``window_defs`` dict;
  ``_extract_window_clause()`` populates it before the select list is
  processed, and ``_window_func_call()`` resolves name references against
  it.  The planner, optimizer, codegen, and VM are unchanged.

  18 oracle tests added in ``tests/test_tier3_named_window.py``.

## [2.19.0] - 2026-06-15

### Added

- **Row-value comparisons** — multi-column predicates are now supported in
  ``WHERE``, ``ON``, and expression contexts, matching SQLite's behaviour:

  - ``(a, b) = (1, 2)`` — pairwise equality (expands to ``a=1 AND b=2``)
  - ``(a, b) != (1, 2)`` — pairwise inequality (expands to ``a!=1 OR b!=2``)
  - ``(a, b) < (1, 2)`` — lexicographic less-than
  - ``(a, b) <= / > / >= (x, y)`` — other ordered comparisons
  - ``(a, b) IN ((1,2),(3,4))`` — multi-column IN membership
  - ``(a, b) NOT IN ((1,2),(3,4))`` — negated IN

  Works for any number of columns; 3-column and wider row values expand
  recursively using the same lexicographic rule as SQLite.

  **Implementation:** the grammar's ``comparison`` rule gained three new
  PEG alternatives (``row_value cmp_op row_value``,
  ``row_value NOT IN (row_value_list)``, ``row_value IN (row_value_list)``)
  that fire before the existing scalar ``collated`` form.  The adapter
  expands each row-value comparison into an equivalent scalar
  ``BinaryExpr`` tree, so the planner, optimizer, codegen, and VM require
  no changes.  Scalar regressions are not affected — non-parenthesised
  expressions do not match ``row_value``.

## [2.18.0] - 2026-06-15

### Added

- ``CREATE TABLE dst AS SELECT … FROM src`` (CTAS — *Create Table As
  Select*) is now supported.  The statement:

  1. Executes the source SELECT.
  2. Creates the destination table with one column per SELECT output,
     named after the output alias (or the source column name for bare
     column references).
  3. Bulk-inserts all rows from the SELECT result into the new table.

  ``IF NOT EXISTS`` is honoured: if the destination table already exists
  the entire statement becomes a no-op (no rows are inserted into the
  existing table).

  The destination is created even when the source SELECT returns zero
  rows — column names are inferred by planning the SELECT statement
  against the source schema without executing it.

  CTAS respects all standard SELECT modifiers: ``WHERE``, ``ORDER BY``,
  ``LIMIT``, ``GROUP BY``, ``HAVING``, aggregates, CTEs, ``TEMP`` /
  ``TEMPORARY``, and so on.

  CTAS is blocked under ``PRAGMA query_only = 1`` (it is a DDL write).

  Known limitations versus real SQLite:

  * Column types in the destination are always ``BLOB`` affinity
    regardless of the source column's declared type.  Queries return
    correct values because SQLite / mini-sqlite use dynamic typing;
    only ``PRAGMA table_info`` shows ``BLOB`` rather than the original
    type.
  * Unnamed computed expression columns (e.g. ``SELECT x * 2 FROM t``)
    are assigned a positional name (``col_0``, ``col_1``, …).  Real
    SQLite uses the expression text (``x * 2``) as the column name.

## [2.17.0] - 2026-05-24

### Fixed

- ``DETACH DATABASE <name>`` now raises SQLite-compatible
  ``OperationalError`` messages instead of silently succeeding:

  * ``DETACH DATABASE main`` → ``"cannot detach database main"``
  * ``DETACH DATABASE <any-other>`` → ``"no such database: <name>"``
    (mini-sqlite has no concept of attached databases, so any name
    that is not "main" is, by definition, not attached)

  The ``DATABASE`` keyword is optional (``DETACH aux`` and
  ``DETACH DATABASE aux`` are both handled).  Quoted schema names
  (double-quoted, single-quoted, backtick-quoted, or
  bracket-quoted) are stripped before the comparison so
  ``DETACH "aux"`` produces the same message as ``DETACH aux``.

  Previously mini-sqlite returned an empty success result for all
  DETACH statements; callers that inspect the error to detect
  mis-typed or missing schema names would silently succeed and miss
  the problem.

  ``ATTACH DATABASE`` remains a no-op (returns success) because
  mini-sqlite does not implement multi-database schema routing and
  there is no appropriate error to produce for a fresh attach.

## [2.16.0] - 2026-05-24

### Added

- ``PRAGMA query_only = 1`` now enforces read-only mode.  Any DML
  (INSERT / UPDATE / DELETE) or DDL (CREATE TABLE / DROP TABLE /
  CREATE INDEX / DROP INDEX / ALTER TABLE / CREATE VIEW / DROP VIEW /
  CREATE TRIGGER / DROP TRIGGER) executed while ``query_only = 1``
  is active on the connection raises::

      OperationalError: attempt to write a readonly database

  — matching SQLite's ``SQLITE_READONLY`` (code 8) behaviour.
  ``PRAGMA query_only = 0`` always lifts the gate, even while it is
  engaged.  SELECT, BEGIN/COMMIT/ROLLBACK, SAVEPOINT/RELEASE, and
  PRAGMA statements are never affected.

  Lifts the "Scope limit" noted in the 2.13.0 entry.  Previously
  the PRAGMA value round-tripped correctly but had no semantic
  effect — writes silently executed regardless of the setting.

### Fixed

- ``Connection.close()`` now evicts per-connection PRAGMA state from
  the engine's ``_PRAGMA_STATE`` dict.  Previously, if the underlying
  backend object was garbage-collected and its memory address reused
  by a new connection, the new connection could silently inherit the
  closed connection's PRAGMA settings (e.g. ``query_only=1`` from a
  previous test or caller).  A ``__del__`` hook ensures cleanup
  even when ``close()`` is not called explicitly.

## [2.15.0] - 2026-05-24

### Fixed

- ``CAST(<blob> AS TEXT)`` now UTF-8-decodes the BLOB bytes rather
  than hex-encoding them — matches SQLite.  So ``CAST(x'48656c6c6f'
  AS TEXT)`` returns ``'Hello'`` (was ``'48656c6c6f'``) and
  ``CAST(CAST(42 AS BLOB) AS TEXT)`` round-trips to ``'42'`` (was
  ``'3432'``).  Together with the 2.14.0 fix to ``CAST(<numeric> AS
  BLOB)``, this restores SQLite's documented round-trip identity::

      CAST(CAST(n AS BLOB) AS TEXT) == CAST(n AS TEXT)

  Lifts the "Known limitation" noted in 2.14.0.  Invalid UTF-8
  bytes are mapped to U+FFFD via ``errors="replace"`` so the cast
  is total (mini's SQL-engine layer stays lenient; sqlite3's Python
  binding raises at fetch time via ``text_factory`` instead — a
  binding-layer divergence, not an engine-layer one).  See sql-vm
  1.59.0 for the scalar-function-level fix.

## [2.14.0] - 2026-05-24

### Fixed

- ``CAST(<numeric> AS BLOB)`` now matches SQLite — the BLOB is the
  UTF-8 encoding of the numeric's text form (``CAST(1 AS BLOB)``
  → ``b'1'``, not ``b'\x00\x00\x00\x00\x00\x00\x00\x01'``).
  Previously the integer and float paths used ``struct.pack`` to
  produce 8-byte big-endian binary blobs that didn't survive
  round-tripping through SQLite's text-first conversion rules.
  See sql-vm 1.58.0 for the scalar-function-level fix.

## [2.13.0] - 2026-05-24

### Added

- ``PRAGMA read_uncommitted`` and ``PRAGMA query_only`` are now
  recognised as accept-and-store boolean PRAGMAs (default ``0``).
  Reads return the current integer; writes accept
  ``1``/``0``/``ON``/``OFF``/``TRUE``/``FALSE``/``YES``/``NO`` and
  the value persists for the lifetime of the connection.  Both are
  also advertised in ``PRAGMA pragma_list``.

  Previously both returned ``[]`` (vs sqlite3's documented
  ``[(0,)]`` default) and silently dropped writes — defensive callers
  in ORMs and migration tools that probe these PRAGMAs to decide
  whether to issue an explicit reset would trip on the missing
  default.

### Scope limits

- ``read_uncommitted`` controls SQLite's shared-cache isolation
  level.  Mini-sqlite has no shared cache, so the PRAGMA's value
  has no semantic effect — it just round-trips per connection.
- ``query_only = 1`` should reject writes in SQLite ("attempt to
  write a readonly database").  Mini-sqlite does NOT yet enforce
  the read-only gate — INSERT/UPDATE/DELETE still execute even
  when ``query_only = 1``.  Enforcement is a deferred increment,
  pinned by an explicit regression test that documents the current
  divergence.

## [2.12.0] - 2026-05-24

### Fixed

- ``CAST(TRUE AS TEXT)`` / ``CAST(FALSE AS TEXT)`` (and the
  ``VARCHAR`` / ``CHAR`` / ``NVARCHAR`` aliases) now return ``'1'``
  / ``'0'`` instead of Python's ``'True'`` / ``'False'``.  Matches
  SQLite — see the sql-vm 1.57 entry for the scalar-function-level
  fix.

  Common idioms that the bug used to corrupt::

      SELECT 'is_active=' || CAST(is_active AS TEXT) FROM users;
      WHERE CAST(flag AS TEXT) = '1'

  Now both sides agree with sqlite3.

## [2.11.0] - 2026-05-24

### Changed

- PRIMARY KEY uniqueness violations now surface as ``UNIQUE
  constraint failed: <table>.<col>`` (matching SQLite) instead of
  ``PRIMARY KEY constraint failed: …``.  PRIMARY KEY implies UNIQUE
  in SQL, so SQLite never emits a dedicated "PRIMARY KEY constraint
  failed" message; mini-sqlite now follows the same convention.

  Covers all three uniqueness paths:

  * Named-column PRIMARY KEY (``a INT PRIMARY KEY``).
  * INTEGER PRIMARY KEY (rowid alias) — both InMemoryBackend and
    storage-sqlite paths.
  * UPDATE that creates a duplicate PK (regression — pinned by a
    test).

  See the matching entries in sql-backend 0.22 and storage-sqlite
  0.19 for the layered fix.

## [2.10.0] - 2026-05-24

### Changed

- CHECK constraint violation messages now match SQLite::

      CHECK constraint failed: a > 0

  Previously emitted ``CHECK constraint failed: <table>.<col>`` —
  which doesn't tell the user *why* the check rejected the row and
  broke tests that pin error strings against the sqlite3 oracle.

  Implementation: the adapter captures the source text of each
  CHECK predicate by walking the parsed expression's leaf tokens
  (joined with single spaces, with no-space-around rules for parens
  / commas / function-call parens).  The text rides on a new
  ``ColumnDef.check_expr_text`` field through the planner → IR →
  VM pipeline.  See the matching entries in sql-backend 0.21,
  sql-codegen 1.41, and sql-vm 1.56.

  Covered: ``a > 0``, ``a >= 0 AND a <= 100``, ``a = 1 OR a = 2``,
  ``name <> 'bad'``, ``LENGTH(name) > 0``, ``ABS(a) < 10``, ``a IN
  (1, 2, 3)`` — exact-string match against the sqlite3 oracle for
  all forms.

## [2.9.0] - 2026-05-23

### Added

- ``PRAGMA writable_schema`` is now recognised as a read/write
  boolean PRAGMA with the SQLite-compatible default of ``0`` (off).
  Reads return the current integer (0 or 1); writes accept any of
  ``1``/``0``/``ON``/``OFF``/``TRUE``/``FALSE``/``YES``/``NO`` and
  the value persists for the lifetime of the connection.  It is also
  now advertised in ``PRAGMA pragma_list``.

  Mini-sqlite synthesises ``sqlite_master`` on every read (no backing
  table), so honouring writes through the catalog is a much larger
  change.  This PR fills only the read/write round-trip surface so
  defensive callers (ORMs, migration tools, database-repair flows)
  that toggle the PRAGMA before deciding whether to attempt a fix
  see the expected value instead of an empty result or a "unknown
  PRAGMA" error.

  Previously: ``PRAGMA writable_schema`` returned ``[]`` instead of
  the documented ``[(0,)]`` and writes were silently dropped.

## [2.8.0] - 2026-05-23

### Added

- ``LIMIT`` clause now supports SQLite's two non-standard extensions:

  * **Negative count** means "no limit" (unbounded).  ``SELECT v FROM
    t LIMIT -1`` returns all rows; ``LIMIT -1 OFFSET 10`` returns
    everything from row 11 onwards — the canonical "skip N, take
    rest" idiom.

  * **MySQL-compatible ``LIMIT m, n``** is now accepted as a synonym
    for ``LIMIT n OFFSET m``.  Note the reversed argument order: the
    FIRST number is the offset, the SECOND is the count.  This is the
    only place in SQL where the order swaps.

  * **Negative offset** is treated as zero (matches SQLite).  ``LIMIT
    5 OFFSET -3`` returns the first five rows with no skip.

  Previously all three raised ``Parse error … Expected NUMBER, got
  '-'`` or ``Unexpected token: ','`` because the grammar only
  accepted ``LIMIT NUMBER [ OFFSET NUMBER ]``.

  Implementation: new grammar rule ``signed_number = [ "-" ] NUMBER``
  used in both ``LIMIT`` slots, plus a comma-form alternative in the
  trailing position.  The adapter detects the comma form by the
  presence of a COMMA token and swaps the argument interpretation.
  Negative counts map to ``Limit.count=None`` so the planner / codegen
  paths that already understand "no limit" don't need any changes.

## [2.7.0] - 2026-05-23

### Added

- SQLite's NULL-safe equality operators ``x IS y`` and ``x IS NOT y``
  (general RHS form, beyond the existing ``IS NULL`` /
  ``IS NOT NULL`` / ``IS DISTINCT FROM …`` shapes) are now supported.
  ``IS`` is equivalent to ``IS NOT DISTINCT FROM`` (true iff both
  sides are equal *or* both are NULL); ``IS NOT`` is the negation.

  Examples that previously raised ``Parse error … Expected "NULL" or
  "NOT" or "DISTINCT", got '1'``::

      SELECT 1 IS 1                ⟶  1
      SELECT NULL IS NULL          ⟶  1
      SELECT NULL IS 1             ⟶  0
      SELECT 'a' IS 'a'            ⟶  1
      SELECT 1 IS NOT 2            ⟶  1
      WHERE a IS b                 (NULL-safe column comparison)

  Implementation: two new grammar alternatives — ``"IS" collated``
  and ``"IS" "NOT" collated`` — at the end of the IS family (PEG
  order: the more specific ``NULL`` / ``DISTINCT`` forms still get
  the first shot at matching).  The adapter detects the bare-RHS
  form by the count of ``collated`` children (one child = ``IS
  NULL`` shape; two = ``IS <expr>`` shape) and routes through the
  existing ``IS_[NOT_]DISTINCT_FROM`` planner/codegen/VM paths.

## [2.6.0] - 2026-05-23

### Added

- Unary ``+`` prefix operator (e.g. ``SELECT +5``,
  ``SELECT 1 + +2``, ``SELECT -+5``) is now accepted as the documented
  SQLite no-op identity.  Previously raised
  ``ProgrammingError: Parse error … Expected "-" or "~" or NUMBER …,
  got '+'`` because the grammar's ``unary`` rule only listed ``-`` and
  ``~`` as prefixes.

  Implementation: one-character grammar change — ``unary = ( "-" | "~"
  | "+" ) unary | primary`` — plus an adapter pass-through that
  unwraps the ``+`` without emitting an IR node (the operand value is
  unchanged, so adding a layer would just be work for the planner and
  codegen to peel).  Regenerated sql-parser's ``_grammar.py`` cache.

  All combinations work: ``+5.5``, ``+(-3)``, ``-+5``, ``++5``,
  ``+~5``, ``+a`` in SELECT/WHERE/ORDER BY/CASE/function-argument
  positions.

## [2.5.0] - 2026-05-23

### Added

- ``CURRENT_DATE``, ``CURRENT_TIME``, and ``CURRENT_TIMESTAMP`` SQL
  keyword expressions are now supported, matching SQLite::

      CURRENT_DATE      ⟶  'YYYY-MM-DD'             (10 chars)
      CURRENT_TIME      ⟶  'HH:MM:SS'               (8  chars)
      CURRENT_TIMESTAMP ⟶  'YYYY-MM-DD HH:MM:SS'    (19 chars)

  Previously these raised ``OperationalError: unknown column:
  'CURRENT_TIMESTAMP'`` because the SQL token grammar doesn't list
  them as keywords — the lexer emits them as bare NAME tokens, and
  the planner couldn't resolve any matching column.

  Implementation: the adapter's ``_column_ref_to_expr`` intercepts
  single-name column refs whose value (case-insensitive) matches
  ``CURRENT_DATE`` / ``CURRENT_TIME`` / ``CURRENT_TIMESTAMP`` and
  rewrites them to the equivalent scalar-function call
  (``date('now')`` / ``time('now')`` / ``datetime('now')``) — both
  paths already implemented in the VM.

### Known limitation

- A column literally named ``CURRENT_DATE`` (or ``CURRENT_TIME`` /
  ``CURRENT_TIMESTAMP``) is shadowed by the keyword even when
  referenced via the double-quoted form (e.g.
  ``SELECT "CURRENT_DATE" FROM t``).  SQLite distinguishes the two
  cases because it preserves the "was quoted" flag through tokenization;
  mini-sqlite's lexer post-processing strips quotes from quoted
  identifiers and loses that distinction.  Workaround: don't name
  columns after SQL keyword expressions.  Tracked for follow-up if
  it ever bites a real user.

## [2.4.0] - 2026-05-23

### Added

- ``WITH RECURSIVE`` CTEs now accept a ``VALUES`` anchor, matching
  SQLite.  The canonical "count from N" idiom works::

      WITH RECURSIVE c(n) AS (
          VALUES(1)
          UNION ALL
          SELECT n + 1 FROM c WHERE n < 5
      ) SELECT n FROM c

  Previously this raised ``ProgrammingError: expected child rule
  'select_stmt' under query_stmt`` because the recursive CTE branch
  of ``mini_sqlite.adapter`` only looked for a ``select_stmt`` child
  in the inner ``query_stmt``.  The non-recursive branch already
  supported VALUES via ``_query_stmt`` recursion.

  Implementation: the adapter first tries ``values_stmt`` and, if
  present, runs ``_values_stmt`` to build the anchor; otherwise falls
  back to the existing ``select_stmt`` path.  Single-row VALUES
  (which is what every realistic recursive anchor needs) maps cleanly
  onto ``RecursiveCTERef.anchor: SelectStmt``.  Multi-row VALUES
  anchors are rejected with a clear pointer to the
  ``SELECT … UNION ALL SELECT …`` rewrite — the planner's recursive
  anchor path expects a single SELECT.

## [2.3.0] - 2026-05-23

### Fixed

- ``ORDER BY <expr>`` with arbitrary expressions (``ORDER BY a+b``,
  ``ORDER BY UPPER(name)``, ``ORDER BY CASE WHEN … END``, …) now
  matches SQLite row-for-row instead of raising
  ``InternalError: unexpected error: ValueError: tuple.index(x): x not
  in tuple``.  Root cause was in sql-codegen's hidden-column
  injection pass — see sql-codegen CHANGELOG 1.40.0 for the full fix.

  Also covers multiple expression sort keys (``ORDER BY a+1, b-1``),
  mixed expression + column keys, and the ``LIMIT`` / ``OFFSET`` /
  ``DISTINCT`` interactions on top.

## [2.2.0] - 2026-05-23

### Added

- ``INSERT INTO t DEFAULT VALUES`` is now supported.  Inserts a single
  row consisting entirely of column defaults — equivalent to
  ``INSERT INTO t () VALUES ()``.  Useful for tables where every column
  either has a DEFAULT clause, is NULLable, or is an auto-assigned
  INTEGER PRIMARY KEY.  Matches SQLite semantics: NOT NULL columns
  without a DEFAULT still raise IntegrityError, RETURNING works as
  expected, and sequential ``DEFAULT VALUES`` inserts increment the
  IPK rowid.

  Implementation: grammar adds ``"DEFAULT" "VALUES"`` as an alternative
  in ``insert_body``; adapter detects the keyword and emits
  ``InsertValuesStmt(rows=((),), columns=())``, which the existing
  ``_apply_defaults`` / ``_autoassign_ipk`` paths in the backend
  already handle correctly for empty-tuple row inputs.

## [2.1.0] - 2026-05-23

### Fixed

- ``INSERT(v) VALUES (...) RETURNING *`` on a table with an
  auto-assigned INTEGER PRIMARY KEY column now returns the
  assigned id instead of ``NULL``.  Was the documented "known
  limitation" of 2.0.  Backend ``insert()`` reflects auto-
  assigned values back to the caller's dict, so the VM's
  ``LoadLastInsertedColumn`` path sees the post-assign state.
- DEFAULT column values are similarly reflected — ``INSERT INTO
  t(id) VALUES (1) RETURNING *`` on ``v TEXT DEFAULT 'hi'``
  surfaces ``'hi'`` rather than ``NULL``.

## [2.0.0] - 2026-05-23

### Added

- ``RETURNING *`` shorthand is now supported on INSERT, UPDATE, and
  DELETE — expands to one column per table column in declaration
  order (matches SQLite).  Mixed forms like ``RETURNING id, *`` are
  also accepted.

  Implementation: parser accepts ``returning_item = "*" | expr``;
  adapter emits :class:`Wildcard` sentinel; planner's
  ``_expand_returning_wildcards()`` expands to ``Column(table, col)``
  references at resolution time.

### Known limitation

- ``INSERT INTO t(v) VALUES (...) RETURNING *`` reports the
  auto-assigned INTEGER PRIMARY KEY column as NULL when the user
  omitted the id.  Pre-existing bug in INSERT RETURNING that's
  independent of the ``*`` shorthand — explicit-id INSERTs and
  UPDATE/DELETE RETURNING work correctly.  Filed for follow-up.

## [1.99.0] - 2026-05-23

### Added

- ``PRAGMA foreign_keys`` is now honoured at INSERT/UPDATE/DELETE.
  The engine reads the per-connection PRAGMA value and forwards it
  to the VM's new ``fk_enabled`` flag.  Setting ``PRAGMA foreign_keys
  = OFF`` disables FK enforcement for subsequent statements on the
  same connection; ``PRAGMA foreign_keys = ON`` re-enables it.

### Changed

- ``_PRAGMA_DEFAULTS["foreign_keys"]`` flipped from ``0`` (OFF) to
  ``1`` (ON) so the read value matches mini-sqlite's enforce-by-
  default behaviour.  This is a *documented* deviation from SQLite,
  which defaults the pragma to OFF.  ORMs and migration tools that
  explicitly toggle the pragma get correct behaviour either way.
- Two pragma-additions tests updated for the new default:
  ``test_foreign_keys_default_off`` renamed/rewritten as
  ``test_foreign_keys_default_on_in_mini_sqlite``;
  ``test_foreign_keys_isolated_between_connections`` now toggles
  c1 to OFF (rather than ON) so the isolation test still verifies
  per-connection state.

## [1.98.0] - 2026-05-23

### Added

- ``PRAGMA foreign_key_check`` is now implemented.  Walks every (or
  one named) child table and reports one row per FK violation::

      table   TEXT    — child table holding the bad row
      rowid   INTEGER — the bad row's rowid
      parent  TEXT    — referenced parent table
      fkid    INTEGER — 0-based FK position (matches
                        ``foreign_key_list.id``)

  NULL child FK values pass unconditionally (SQL standard).  When the
  ``REFERENCES`` clause omits a parent column, the parent's first
  PRIMARY KEY column is used — matches the existing INSERT-time FK
  validation.  ``foreign_key_check(<table>)`` restricts the scan to
  one child table.
- ``foreign_key_check`` added to ``PRAGMA pragma_list``.

## [1.97.0] - 2026-05-23

### Changed

- ``PRAGMA table_info`` now matches real ``sqlite3`` exactly:

  * ``notnull`` distinguishes explicit ``NOT NULL`` from the implicit
    ``NOT NULL`` that ``PRIMARY KEY`` introduces — ``CREATE TABLE t
    (id INTEGER PRIMARY KEY)`` reports ``notnull=0``, while
    ``... PRIMARY KEY NOT NULL`` reports ``notnull=1``.  Runtime
    NOT NULL enforcement is unchanged — the backend still treats PK
    as implicit NOT NULL (TEXT PK still rejects ``INSERT VALUES
    (NULL, ...)``, INTEGER PK still auto-assigns the next rowid).
  * ``dflt_value`` returns the literal source text instead of the
    parsed Python value: ``DEFAULT 42`` → ``'42'``, ``DEFAULT 'x'``
    → ``"'x'"`` (single quotes preserved), ``DEFAULT NULL`` →
    ``'NULL'``, ``DEFAULT 3.14`` → ``'3.14'``.  ``X'hex'`` BLOB
    literals also round-trip.

- The adapter no longer sets ``not_null=True`` for PRIMARY KEY columns
  (the codegen now reads the raw flag through to the backend).
  Internal ``effective_not_null()`` continues to OR the PK bit in for
  constraint validation.

## [1.96.0] - 2026-05-23

### Added

- ``PRAGMA index_info(<index-name>)`` is now implemented.  Returns
  one row per indexed column with the SQLite-standard triple
  ``(seqno, cid, name)``: the position in the index key, the column
  id in the parent table, and the column name.  Returns zero rows
  (no error) for an unknown index — matches SQLite.

### Changed

- ``PRAGMA index_list(<table>)`` now returns the SQLite-standard
  5-column shape ``(seq, name, unique, origin, partial)`` instead of
  the previous 3-column ``(seq, name, unique)``.  ``origin`` is
  ``'c'`` for user-created indexes and ``'u'`` for auto-created
  ``sqlite_autoindex_*`` indexes.  ``partial`` is always 0 —
  mini-sqlite doesn't support partial indexes.
- ``index_info`` added to the ``PRAGMA pragma_list`` enumeration.

## [1.95.0] - 2026-05-23

### Added

- ``SELECT ... FROM sqlite_sequence`` now returns the high-water
  rowid for each AUTOINCREMENT table — completes the AUTOINCREMENT
  story started in 1.94.  The table materializes lazily: ``SELECT
  * FROM sqlite_sequence`` on a fresh database errors with
  "no such table" until at least one AUTOINCREMENT table is created
  (matches SQLite).
- ``CREATE TABLE sqlite_sequence``, ``DROP TABLE sqlite_sequence``,
  and ``INSERT INTO sqlite_sequence`` are rejected with the same
  reserved-name guard as ``sqlite_master``.

## [1.94.0] - 2026-05-23

### Added

- ``CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, ...)`` is
  now fully end-to-end.  The adapter parses the ``AUTOINCREMENT``
  keyword and forwards ``autoincrement=True`` to the backend
  ``ColumnDef``.  The in-memory backend's ``_next_rowid`` counter is
  never decremented (already monotonic), so SQLite's "deleted rowids
  never reuse" guarantee holds.  The keyword round-trips through
  ``sqlite_master.sql``.

## [1.93.0] - 2026-05-23

### Added

- ``INSERT INTO t(other_col) VALUES (...)`` and ``INSERT INTO t
  VALUES (NULL, ...)`` now work end-to-end on a table with an
  ``INTEGER PRIMARY KEY`` column.  The id auto-assigns to the next
  rowid — SQLite's "INTEGER PRIMARY KEY is an alias for rowid"
  semantics.  Previously these forms failed with a NOT NULL
  violation, blocking ORM patterns.
- The rowid pseudo-column now aliases the INTEGER PRIMARY KEY
  column when one exists: ``SELECT rowid, id FROM t`` returns
  identical values per row.
- ``last_insert_rowid()`` correctly reports the auto-assigned id
  (or the explicit id when one was supplied).

### Changed

- ``SELECT *`` on a table with a partially-omitted INSERT now
  returns columns in declaration order rather than insertion order
  (where the omitted columns previously moved to the end).  See
  ``sql-backend`` 0.18 for the underlying ``_apply_defaults`` fix.

## [1.92.0] - 2026-05-23

### Added

- ``EXPLAIN QUERY PLAN`` SEARCH detail rows now include the matched
  index bounds in SQLite's compact format::

      SEARCH t USING INDEX ix_x (x=?)
      SEARCH t USING INDEX ix_x (x>?)
      SEARCH t USING INDEX ix_x (x>? AND x<?)
      SEARCH t USING INDEX ix_xy (x=? AND y=?)
      SEARCH t USING INDEX ix_xy (x=? AND y>?)

  The previous format emitted just ``SEARCH t USING INDEX <name>``
  without the bound suffix.  Inclusivity markers (``>=`` and ``<=``)
  are collapsed to ``>`` and ``<`` for compactness — matches real
  SQLite's behaviour.

## [1.91.0] - 2026-05-23

### Added

- ``... FROM t INDEXED BY <name> ...`` and ``... FROM t NOT INDEXED
  ...`` query hints are now wired end-to-end.  The adapter parses
  the new ``index_hint`` grammar node and attaches it to ``TableRef``;
  the planner forwards it to ``Scan``; the optimizer's
  ``_try_index_scan`` honours both hints.
- An unknown index in ``INDEXED BY <name>`` raises ``OperationalError``
  with the message ``no such index on table <t>: <name>`` — mirrors
  SQLite's behaviour.  Wired through ``mini_sqlite.errors.translate``
  which now also maps ``sql_planner.errors.IndexNotFound``.

## [1.90.0] - 2026-05-23

### Added

- ``EXPLAIN QUERY PLAN <stmt>`` is now implemented end-to-end.  The
  engine parses and plans the inner statement (without executing it),
  walks the optimised ``LogicalPlan``, and emits a four-column row set
  (``id``, ``parent``, ``notused``, ``detail``) that mirrors SQLite's
  output shape.  Detail strings cover the common plan shapes:
  ``SCAN <table>``, ``SEARCH <table> USING INDEX <name>``, ``USE TEMP
  B-TREE FOR ORDER BY / GROUP BY / DISTINCT / WINDOW FUNCTION``, and
  ``SCAN SUBQUERY <alias>``.  Pure transforms (Filter, Project, Limit,
  Having, Join) are elided so children reparent to the elided node's
  parent — matching SQLite's output topology.
- Bare ``EXPLAIN`` (without ``QUERY PLAN``) continues to return an
  empty result — mini-sqlite does not expose its internal IR as VDBE
  bytecode.

### Changed

- ``sqlite_master.rootpage`` now reports a stable non-zero monotonic
  integer for tables and indexes (matching SQLite's convention) instead
  of always returning 0.  Triggers still report 0 (not a b-tree
  object).  See ``sql-backend`` 0.17 for the underlying change.

## [1.89.0] - 2026-05-23

### Added

- ``SELECT ... FROM sqlite_master`` (and its alias ``sqlite_schema``)
  now works on ``:memory:`` databases.  The in-memory backend
  synthesizes the catalog rows from current schema state on every
  scan — no storage, no maintenance, no DDL hook plumbing.  Common
  migration-tool queries like ``SELECT name FROM sqlite_master WHERE
  type='table'`` return rows identical to real ``sqlite3``.
- ``INSERT`` / ``DROP TABLE`` / ``CREATE TABLE`` targeting the
  reserved names ``sqlite_master`` / ``sqlite_schema`` are rejected
  with ``IntegrityError``.

## [1.88.0] - 2026-05-23

### Added

- ``CREATE TABLE ... STRICT`` is now fully end-to-end:
  * Parser already accepted ``table_options = table_option {","
    table_option}`` and ``table_option = "STRICT" | "WITHOUT" NAME``;
    the adapter now lifts the ``STRICT`` keyword into
    ``CreateTableStmt.strict``.
  * The in-memory backend rejects column types outside SQLite's
    ``{INT, INTEGER, REAL, TEXT, BLOB, ANY}`` set at CREATE TABLE time,
    and validates value types on every INSERT/UPDATE.  ``ANY`` columns
    opt back into lenient typing inside a STRICT table — matching
    SQLite's escape hatch.
  * ``WITHOUT ROWID`` continues to be parsed and silently accepted —
    mini-sqlite always uses a rowid table internally.

## [1.87.0] - 2026-05-23

### Added

- **ALTER TABLE RENAME TO / RENAME COLUMN / DROP COLUMN** — three
  new forms beyond the existing ADD COLUMN, matching SQLite 3.25+ /
  3.35+ syntax::

      ALTER TABLE old RENAME TO new
      ALTER TABLE t RENAME [COLUMN] old_col TO new_col
      ALTER TABLE t DROP [COLUMN] col_name

  The ``COLUMN`` keyword is optional everywhere.  Rows survive the
  rename intact; the dropped column's values are removed from each
  row.  Indexes follow renames automatically (their ``table`` /
  ``columns`` fields get rewritten).
- 16 oracle tests in ``test_tier3_alter_table_extras.py`` covering
  all three new forms (with and without the ``COLUMN`` keyword),
  rename-then-query, index follow-along, dropped column
  unavailability, and SQLite's restrictions (cannot drop the only
  column, the PRIMARY KEY, or a column referenced by an index).
- Adapter ``_alter_table`` now parses all four forms via keyword
  dispatch, building an ``AlterTableStmt`` with exactly one of
  ``column`` / ``rename_to`` / ``rename_column`` / ``drop_column``
  set.

### Fixed

- ``ALTER TABLE … ADD COLUMN x TEXT DEFAULT 'foo'`` now backfills
  existing rows with the DEFAULT value (``'foo'``) rather than NULL.
  The bug was in sql-vm's ``_do_alter_table``, which constructed the
  backend ColumnDef without forwarding ``default``.

## [1.86.0] - 2026-05-23

### Added

- **Implicit COLLATE in WHERE / UPDATE / DELETE**.  The column's
  declared ``COLLATE`` clause now propagates into any comparison the
  column participates in — matching SQLite's implicit collation
  semantics.  So::

      CREATE TABLE users(email TEXT COLLATE NOCASE);
      INSERT INTO users VALUES ('Adhithya@example.com');
      SELECT * FROM users WHERE email = 'adhithya@example.com';
      -- ('Adhithya@example.com',)  ← case-insensitive match

  works without an explicit ``COLLATE NOCASE`` on the WHERE.
  Equally for ``UPDATE … WHERE name = 'X'`` and ``DELETE FROM …
  WHERE name = 'X'`` against a NOCASE-declared column.
- Same propagation applies to ``<``, ``<=``, ``>``, ``>=``,
  ``<>``, ``IS [NOT] DISTINCT FROM``, and ``BETWEEN`` — every
  comparison op whose value semantics depend on string ordering.
- 18 oracle tests in
  ``tests/test_tier3_collate_implicit_from_column.py``: basic
  equality, ordering, multi-column independence, complex
  predicates (AND / OR / NOT / BETWEEN / IS DISTINCT FROM),
  RTRIM column, UPDATE / DELETE, and the explicit-override path.

### Known limitations

- Explicit ``COLLATE BINARY`` postfix does NOT override a
  column-declared NOCASE (because the explicit-BINARY postfix is an
  identity transform at the adapter, leaving no marker for the
  propagation pass to recognise).  Override with ``COLLATE NOCASE``
  or ``COLLATE RTRIM`` instead — those work as expected.
- HAVING clauses don't currently propagate column collation through
  GROUP BY.  Use explicit ``COLLATE NOCASE`` on the HAVING.

## [1.85.0] - 2026-05-23

### Added

- ``COLLATE name`` accepted as a column constraint in CREATE TABLE:

      CREATE TABLE users(email TEXT COLLATE NOCASE);

  Plays through the full pipeline (parser → adapter → codegen → VM
  → backend) and is then read back by the planner when an ORDER BY
  references the column without an explicit COLLATE override.  So::

      CREATE TABLE t(name TEXT COLLATE NOCASE);
      INSERT INTO t VALUES ('Banana'), ('apple'), ('CHERRY');
      SELECT name FROM t ORDER BY name;
      -- ('apple',), ('Banana',), ('CHERRY',)   ← NOCASE order

  matches stdlib sqlite3 byte-for-byte.  Explicit ``COLLATE`` on the
  ORDER BY clause still overrides the column's declared collation.

- 15 oracle tests in ``tests/test_tier3_column_collate.py``: parsing
  of COLLATE alongside every other column constraint (NOT NULL,
  DEFAULT, UNIQUE, PRIMARY KEY), implicit / explicit / RTRIM
  propagation, per-column independence, and ``ALTER TABLE ADD COLUMN
  … COLLATE`` round-trip.

### Changed

- Adapter ``_col_def`` recognises the new ``COLLATE NAME``
  ``col_constraint`` and stores the upper-cased name on the column's
  ``BackendColumnDef.collation`` field.

## [1.84.0] - 2026-05-23

### Added

- ``expr COLLATE name`` postfix accepted on either operand of any
  comparison operator (``=``, ``<>``, ``<``, ``<=``, ``>``, ``>=``,
  ``BETWEEN`` / ``NOT BETWEEN``, ``IS DISTINCT FROM`` /
  ``IS NOT DISTINCT FROM``, ``IN``).  Implements byte-identical
  semantics with stdlib ``sqlite3``: ``'Foo' = 'foo' COLLATE NOCASE``
  evaluates to TRUE, and a ``WHERE`` clause with a COLLATE postfix
  filters rows case-insensitively.
- 22 oracle tests in ``tests/test_tier3_collate_in_comparisons.py``
  covering NOCASE equality (both LHS and RHS collation, both-sides
  collation, default-BINARY behaviour), comparison ops, RTRIM,
  BETWEEN / NOT BETWEEN, IS DISTINCT FROM, multi-predicate composition
  with WHERE + ORDER BY, NULL propagation, integer operand regression
  guard, and unknown-collation fallback.

### Changed

- Implementation is a pure adapter-level rewrite — no
  planner / codegen / VM changes.  When ``_comparison`` builds a
  comparison expression and either operand has a COLLATE clause, it
  wraps **both** operands in ``lower()`` (for NOCASE) or ``rtrim()``
  (for RTRIM).  ``BINARY`` and unknown names fall through to
  identity, matching SQLite's "validate lazily" behaviour.
- ``_order_item`` now also walks the inner ``collated`` subtree to
  pick up COLLATE clauses that the PEG parser greedily consumed at
  the inner level (which would otherwise leave the outer
  ``order_item`` slot empty and silently drop the collation).  Both
  ``ORDER BY x COLLATE NOCASE`` and direct comparisons now work
  end-to-end.

## [1.83.0] - 2026-05-22

### Added

- ``ORDER BY expr COLLATE name`` — collation-aware sorting,
  byte-identical with stdlib ``sqlite3``.  Recognises SQLite's
  three built-in collations:
  - ``BINARY`` (default, what ``None`` means too): byte-for-byte
  - ``NOCASE``: ASCII case-insensitive (``'A' == 'a'``)
  - ``RTRIM``: strip trailing spaces before BINARY-comparing
  Unknown collation names fall through to BINARY rather than
  raising — matching SQLite's "validate lazily" approach (the user
  may have registered a custom collation we don't know about).
- 18 oracle tests in ``tests/test_tier3_order_by_collate.py``
  covering all three collations, with/without DESC, with/without
  NULLS FIRST/LAST, multi-column ORDER BY, positional
  (``ORDER BY 1 COLLATE NOCASE``), aliased columns, mixed-case
  word lists, NULL values, integer regression guard (collations
  inert on non-strings), and the unknown-collation fallback.

### Changed

- Adapter ``_order_item`` parses the optional ``COLLATE name``
  clause and stores it (upper-cased) on
  ``SortKey.collation``.  The planner, optimizer, and codegen
  thread it through unchanged; the VM ``_do_sort`` applies the
  named transform when building the sort key.

## [1.82.0] - 2026-05-22

### Added

- **Compound ORDER BY / LIMIT** — trailing ``ORDER BY`` and ``LIMIT``
  on a ``UNION / INTERSECT / EXCEPT`` chain now apply to the whole
  compound, matching SQLite (and the SQL standard).  Previously the
  grammar parsed them into the rightmost SELECT, where the column
  name from the leftmost SELECT's projection was invisible — the
  result was a confusing ``unknown column: 'x'`` error on perfectly
  valid SQL like::

      SELECT 1 AS x UNION ALL SELECT 2 ORDER BY x

  The fix hoists trailing ``ORDER BY``/``LIMIT`` from the rightmost
  SELECT onto a wrapper ``SELECT * FROM (compound) ORDER BY … LIMIT
  …``, which makes the compound's output column names (inherited
  from the leftmost SELECT, matching SQLite) visible to the ORDER BY
  clause.
- 18 oracle tests in ``tests/test_tier3_compound_order_limit.py``
  covering UNION ALL, UNION (dedup), INTERSECT, EXCEPT, ORDER BY by
  name and by position, LIMIT, LIMIT/OFFSET, and the interaction
  with VALUES (from PR #3968).

## [1.81.0] - 2026-05-22

### Added

- **VALUES clause** support, end-to-end and byte-identical with
  stdlib `sqlite3`.  ``VALUES (a, b), (c, d), …`` now works anywhere
  a SELECT does:
  - **Standalone**: ``VALUES (1, 'a'), (2, 'b')`` is a top-level
    statement that returns a rowset.
  - **Derived table**: ``SELECT * FROM (VALUES (1), (2))``.
  - **CTE body** (with or without column alias list):
    ``WITH t(n) AS (VALUES (1), (2)) SELECT n FROM t``.
  - **Set-op operand**, either side: ``SELECT 1 UNION ALL VALUES (2)``
    and the symmetric ``VALUES (1) UNION SELECT 2``.
- Default column names are ``column1``, ``column2``, … (1-indexed)
  when no explicit alias list is given — matching SQLite.
- 21 oracle tests in ``tests/test_tier3_values_clause.py``.

### Changed

- Adapter desugars VALUES into a left-deep ``UNION ALL`` chain of
  single-row SELECTs, so the planner, codegen, and VM see only
  constructs they already handle.
- ``_set_op_clause`` now returns a 4-tuple ``(op, all_flag, body_node,
  body_rule)`` so callers can dispatch on whether the operand is a
  SELECT or a VALUES list.  Recursive-CTE callers reject VALUES as
  the recursive step with a clear error (the standard requires the
  step to be a SELECT referencing the CTE's working set).

## [1.80.0] - 2026-05-22

### Added

- **PRAGMA pragma_list** — returns the alphabetical catalog of the
  PRAGMAs mini-sqlite actually implements (39 names — the dedicated
  handlers plus every writable scalar in ``_PRAGMA_DEFAULTS``).  Apps
  and ORMs probe this to learn what's safe to call; until now we
  returned an empty rowset, which made tools assume nothing was
  supported.
- **PRAGMA data_version** — returns ``(1,)`` matching stdlib
  ``sqlite3`` on a fresh ``:memory:`` connection.  The PRAGMA is
  defined as a counter that bumps when another *connection* writes
  to the same database file; mini-sqlite has no shared backing file
  so the counter has no real semantics, but the baseline value of 1
  is what every well-behaved client expects on first read.
- 23 new oracle-or-shape tests in
  ``tests/test_tier3_pragma_introspection.py`` pinning the response
  shape of every read-only introspection PRAGMA mini-sqlite supports.
  Oracle-tested against stdlib ``sqlite3`` where the value is
  meaningful (``database_list``, ``data_version``, ``page_size``,
  ``encoding``, ``integrity_check``, ``quick_check``,
  ``page_count``, ``freelist_count``).  For the inherently
  implementation-dependent lists (``pragma_list``,
  ``compile_options``, ``function_list``) we assert shape and
  ordering rather than membership, since mini-sqlite advertises only
  what it actually implements.

## [1.79.0] - 2026-05-21

### Added

- **UNION / INTERSECT / EXCEPT in non-recursive CTE bodies.**  Closes
  the same gap PR #3817 fixed for derived tables, but for *named*
  CTEs.  Previously the adapter raised
  `"CTE 'x' body must be a plain SELECT, not a set operation"`
  even though the parser accepts the syntax.  Now the body of a
  ``WITH x AS (SELECT … UNION SELECT …)`` plays through the same set-
  op tree that derived tables use.
- CTE column aliases on a set-op body — `WITH u(label) AS (SELECT 'a'
  UNION SELECT 'b') …` — work too.  The aliases apply to the
  *leftmost* SelectStmt in the set-op tree (matching SQLite, which
  derives output column names from the leftmost operand of a set-op
  chain).
- 21 oracle tests in `tests/test_tier3_materialized_cte.py` that pin
  both behaviours: the `[NOT] MATERIALIZED` parse-and-ignore contract
  (existing behaviour, previously untested) and the new set-op-in-CTE
  support, including column aliases and 4-way UNION chains.

### Changed

- `_apply_cte_col_aliases` now walks down the `.left` spine of a
  set-op tree until it reaches the leftmost SelectStmt, then rewrites
  its items.  Returns the same tree shape on the way back out.
- `ctes` dict type widened from
  `dict[str, SelectStmt | RecursiveCTERef]` to
  `dict[str, SelectStmt | UnionStmt | IntersectStmt | ExceptStmt | RecursiveCTERef]`
  across the adapter so set-op bodies can be stored and substituted
  into FROM clauses via the existing `DerivedTableRef.select` union
  type.

## [1.78.0] - 2026-05-21

### Added

- SQLite hex integer literals: `0x1F`, `0XDEADBEEF`, etc.  Accepted
  anywhere a decimal integer is — expressions, WHERE clauses, LIMIT /
  OFFSET clauses, window-frame offsets, INSERT values.  Case-
  insensitive in both the prefix (`0x` vs `0X`) and the hex digits.
- Two SQLite-faithful semantics for these literals:
  1. **16-digit cap.**  More than 16 hex digits (i.e. > 64 bits)
     raises `OperationalError("hex literal too big: ...")`, matching
     the message stdlib `sqlite3` emits.  This also doubles as a
     parser-layer DoS guard: a 1MB hex literal can't pin the parser
     thread in Python's O(N²) `int(s, 16)` path (which is *not*
     covered by Python's PYTHONINTMAXSTRDIGITS guard — that one
     applies to base-10 only).
  2. **64-bit two's-complement wrap.**  `0xFFFFFFFFFFFFFFFF`
     evaluates to `-1` (not `+18446744073709551615`), and
     `0x8000000000000000` evaluates to `-2^63`, matching SQLite's
     INTEGER affinity.
- 31 oracle tests in `tests/test_tier3_hex_int_literals.py` covering
  basic forms, arithmetic, bitwise composition (where these literals
  shine), table data, LIMIT/OFFSET, INSERT, and the wrap/size-cap
  edge cases.

### Changed

- `_parse_number` now recognises the `0x` / `0X` prefix and routes
  through `int(s, 16)` instead of failing in `int(s)`.  The LIMIT /
  OFFSET and frame-offset code paths route through `_parse_number`
  too, where they used to do bare `int(c.value)` / `int(float(...))`
  and would have raised on hex input.

## [1.77.0] - 2026-05-21

### Added

- End-to-end support for the five SQLite bitwise operators
  (`&`, `|`, `<<`, `>>`, `~`).  All five thread through every layer of
  the pipeline (lexer → parser → adapter → planner → optimizer →
  codegen → VM) with byte-identical results vs. stdlib `sqlite3`:
  - 64-bit two's-complement wrap (`1 << 63` → `-9223372036854775808`).
  - Shift saturation at ±64 bits.
  - Negative shift counts flip direction (`a << -k` ≡ `a >> k`).
  - Float operands truncate toward zero before the op runs.
  - NULL propagation through all five operators.
- 46 oracle tests in `tests/test_tier3_bitwise_operators.py` covering
  literal folding, column references, precedence (`& | << >>` bind
  looser than arithmetic, tighter than comparison), float coercion,
  NULL propagation, and use in WHERE clauses.

### Changed

- The `_comparison` adapter helper now descends through `bitwise` nodes
  rather than `additive` ones, matching the new grammar precedence
  layer.  Five call sites updated (cmp_op, BETWEEN, LIKE/ESCAPE, GLOB,
  IS DISTINCT FROM).
- `_unary` learned the `~` prefix, producing `UnaryExpr(BIT_NOT, ...)`
  at the same precedence as unary `-`.

## [1.76.0] - 2026-05-21

### Added

- **Optional alias for derived tables** (via ``sql-parser 0.30.0`` +
  ``sql-planner 0.35.0``).  ``SELECT * FROM (SELECT 1 AS x)`` and
  similar bare-derived-table forms are now accepted; the previous
  ``derived table requires an alias`` adapter error is gone.  Combined
  with PR #3817 (compound queries in derived tables), the
  derived-table feature now fully matches SQLite's syntax and
  semantics.

  14 oracle tests in ``test_tier3_derived_optional_alias.py`` covering:
  - No-alias variants with SELECT * / unqualified column / ORDER BY /
    outer aggregate / outer filter / compound inner query.
  - Aliased forms still work (with/without AS, qualified column refs).
  - JOIN positions with aliased sides and compound inner queries.
  - Real tables providing rows through an unaliased derived table.

## [1.75.0] - 2026-05-21

### Added

- **Compound queries as derived tables** (via ``sql-planner 0.34.0``).
  ``SELECT * FROM (SELECT 1 UNION SELECT 2) AS t``, INTERSECT, EXCEPT,
  and chained variants are now accepted both in FROM and JOIN positions.
  Previously the adapter raised ``derived table must be a plain SELECT,
  not a set operation``.  Alias is still required (optional-alias is a
  separate change).

  16 oracle tests in ``test_tier3_union_in_derived.py`` cover UNION /
  UNION ALL / INTERSECT / EXCEPT in derived tables, set-op chaining,
  JOIN with compound right sides, real-tables-providing-rows pipelines,
  and a regression guard for plain SELECT.

## [1.74.0] - 2026-05-20

### Fixed

- **``CAST(... AS INTEGER)`` saturates at signed 64-bit endpoints**
  (via ``sql-vm 1.48.0``).  Previously preserved Python bigints,
  diverging from SQLite's INTEGER affinity which is always a 64-bit
  signed integer.  16 oracle tests in ``test_tier3_cast_int64_clamp.py``
  cover numeric-literal, string, and float overflow at both
  endpoints; plus regressions for normal-range values.

## [1.73.0] - 2026-05-20

### Fixed

- **Arithmetic edge cases match SQLite byte-for-byte** (via
  ``sql-vm 1.47.0`` and ``sql-optimizer 0.14.0``):

  * ``x / 0`` returns NULL (was raising ``OperationalError``).
  * ``-7 % 3`` returns ``-1`` (was ``2`` — Python's divisor-sign mod).
  * ``7.5 % 2.0`` returns ``1.0`` (SQLite's ``%`` truncates floats to
    int first; was ``1.5`` from true fmod).
  * ``-7 / 2`` returns ``-3`` (was Python's ``-4`` floor-divide).

  21 oracle tests in ``test_tier3_arithmetic_edge_cases.py`` cover
  divide-by-zero / negative-dividend / negative-divisor / mixed
  int+float / column-driven arithmetic.  The ``mod()`` scalar function
  is also pinned (different from ``%`` — keeps true ``fmod``).

## [1.72.0] - 2026-05-20

### Fixed

- **``NOT integer_expr`` now coerces to truth** (via ``sql-vm 1.46.0``).
  Follow-up to 1.71.0: ``NOT 0`` was still raising ``DataError`` because
  the unary ``NOT`` operator had the same Python-bool-only check that
  ``AND`` / ``OR`` did before being fixed.  Now ``NOT 0 → 1``,
  ``NOT 5 → 0``, ``NOT NULL → NULL``, and ``WHERE NOT b`` filters
  integer columns correctly.

  14 oracle tests in ``test_tier3_not_truthiness.py`` covering integer/
  float literals, NULL handling, column refs, WHERE clauses, and
  compound expressions like ``NOT (1 AND 0)`` / ``NOT NOT NULL``.

## [1.71.0] - 2026-05-20

### Fixed

- **``SELECT 1 AND 0`` and similar integer-literal boolean expressions
  now return the correct value** (via ``sql-optimizer 0.13.0`` and
  ``sql-vm 1.45.0``).  Previously the optimizer folded every literal
  combination that wasn't already a Python ``bool`` to ``NULL``,
  including ``1 AND 0`` (should be ``0``), ``1 OR 0`` (should be ``1``),
  and ``NULL AND 0`` (should be ``0`` — FALSE dominates).

  Column-driven AND/OR (e.g. ``WHERE a AND b`` where both columns are
  ``INTEGER``) was also broken at the VM level because
  ``apply_binary(AND, 1, 0)`` raised ``TypeMismatch``.  Both code paths
  now use SQLite's numeric truthiness rule: zero is FALSE, any other
  numeric is TRUE, NULL is unknown.

  23 oracle tests in ``test_tier3_and_or_truthiness.py`` covering
  integer literals, 3-valued logic with NULL, negative / large /
  float numerics, column-driven AND/OR, and a regression guard for
  the boolean-comparison path that previously worked by accident.

## [1.70.0] - 2026-05-20

### Added

- **``TIMEDIFF(A, B)`` scalar function** (via ``sql-vm 1.44.0``) —
  SQLite 3.43+.  Returns ``±YYYY-MM-DD HH:MM:SS.sss`` calendar-aware
  difference ``A − B`` matching SQLite byte-for-byte.

  21 oracle tests in ``test_tier3_timediff.py`` covering basic
  positive/negative differences, fractional seconds, calendar-field
  borrowing across Feb's variable day count, year boundaries, NULL
  propagation, and unparseable inputs.

## [1.69.0] - 2026-05-20

### Fixed

- **``date(..., '+N year')`` Feb 29 rollover matches SQLite**
  (via ``sql-vm 1.43.0``).  Previously
  ``date('2024-02-29', '+1 year')`` returned ``'2025-02-28'`` (clamp);
  SQLite rolls forward to ``'2025-03-01'``.  The fix ports the
  existing month-rollover algorithm to the year branch.

  14 oracle tests in ``test_tier3_year_rollover.py`` covering forward/
  backward rollover, leap-to-leap preservation, and ordinary
  non-Feb-29 dates as regression guards.

## [1.68.0] - 2026-05-20

### Fixed

- **``strftime('%f', …)`` and ``strftime('%W', …)`` now match SQLite
  byte-for-byte** (via ``sql-vm 1.42.0``):

  * ``%f`` (millisecond fraction) was always emitting ``.000``
    because ``_parse_timevalue`` truncated the input before the
    fractional-seconds branch could run.
  * ``%W`` (week of year) was off by one: ``strftime('%W',
    '2024-01-15')`` returned ``'02'`` instead of ``'03'``.

  15 oracle tests in ``test_tier3_strftime_fixes.py``.

## [1.67.0] - 2026-05-20

### Fixed

- **``date()/datetime()/time()`` with ``'unixepoch'`` modifier now
  matches SQLite byte-for-byte** (via ``sql-vm 1.41.0``).  The
  modifier was previously a no-op:

  * ``date('2024-01-01', 'unixepoch')`` returned ``'2024-01-01'``
    (ignoring the modifier); SQLite returns NULL because the input
    is not a numeric Unix-epoch value.
  * ``date('1704067200', 'unixepoch')`` returned NULL (ISO parse
    failed); SQLite returns ``'2024-01-01'`` after parsing the
    numeric string as Unix-epoch seconds.

  Now strings are accepted *only* if the entire value (modulo
  whitespace) is a valid number — ISO dates and numeric prefixes
  followed by garbage both produce NULL.  20 oracle tests in
  ``test_tier3_date_unixepoch.py``.

## [1.66.0] - 2026-05-20

### Fixed

- **``CAST(text AS REAL/INTEGER)`` now matches SQLite byte-for-byte**
  (via ``sql-vm 1.40.0``).  Python's ``int()`` / ``float()`` reject
  any string with trailing non-numeric characters; SQLite takes the
  longest valid numeric prefix.  Two bug classes fixed:

  * ``CAST('inf' AS REAL)`` surfaced Python's infinity; SQLite returns
    ``0.0`` because the literal keyword has no numeric prefix.  Same
    for ``'nan'``, ``'infinity'``, etc.
  * ``CAST('1.5abc' AS REAL)`` returned ``0.0``; SQLite returns ``1.5``.
    ``CAST('123abc' AS INTEGER)`` returned ``0``; SQLite returns ``123``.

  Subtlety: the INTEGER cast extracts only the *integer* prefix, not
  the float prefix.  ``CAST('1.5abc' AS INTEGER)`` is ``1``, and
  ``CAST('1e5' AS INTEGER)`` is also ``1``.

  12 oracle tests in ``test_tier3_cast_numeric_prefix.py``.

## [1.65.0] - 2026-05-20

### Fixed

- **``REPLACE(x, "", y)`` no-op and ``printf('%#o', …)`` C-style octal
  prefix** (via ``sql-vm 1.39.0``).  Both were Python-vs-SQLite
  divergences uncovered by a scalar function gap audit:

  * ``replace('hello', '', 'X')`` returned ``'XhXeXlXlXoX'`` (Python
    behaviour); SQLite returns ``'hello'`` unchanged.
  * ``printf('%#o', 8)`` returned ``'0o10'`` (Python's modern octal
    prefix); SQLite returns ``'010'``.  Width/zero-pad interactions
    also fixed to match SQLite's column rules.

  9 oracle tests in ``test_tier3_replace_empty_and_octal.py``.

## [1.64.0] - 2026-05-20

### Fixed

- **``SUBSTR(x, y[, z])`` edge cases now match SQLite byte-for-byte**
  (via ``sql-vm 1.38.0``).  The previous implementation got three
  classes of inputs wrong:

  * ``y = 0`` — was treated as a sentinel for "start of string"; SQLite
    treats it as one position *before* the first character.
  * Negative ``z`` — was always returning empty; SQLite reads it as
    "``|z|`` characters preceding position ``y``".
  * Far-negative ``y`` (e.g. ``y = -100`` on a 5-char string) —
    arithmetic overflow returned bogus partial strings.

  21 oracle tests in ``test_tier3_substr_edge_cases.py`` compare each
  interesting combination against the reference ``sqlite3`` module.

## [1.63.0] - 2026-05-20

### Fixed

- **``printf('%q', x)`` no longer wraps in single quotes** (via
  ``sql-vm 1.37.0``).  Mini-sqlite was conflating ``%q`` with ``%Q``
  — both wrapped in single quotes.  In SQLite ``%q`` is the
  escape-only form (single quotes doubled, no wrapping); ``%Q`` is
  the complete-literal form (wrapped + NULL → ``"NULL"``).  The
  conflation made it impossible to interpolate the result into a
  larger string literal the caller was building.

  Also: ``printf('%q', NULL)`` now returns the literal text
  ``"(NULL)"`` instead of the empty string.

### Added

- **``printf('%w', x)``** — new SQL identifier escape (via
  ``sql-vm 1.37.0``).  Doubles internal double quotes.  Designed for
  building ``"…"`` quoted identifiers:
  ``printf('SELECT "%w" FROM t', col_name)``.

  15 oracle tests in ``test_tier3_printf_q_w.py`` compare every
  ``%q``/``%Q``/``%w`` case against reference ``sqlite3``
  byte-for-byte.

## [1.62.0] - 2026-05-20

### Fixed

- **``ROUND`` now matches SQLite byte-for-byte** (via ``sql-vm
  1.36.0``).  Mini-sqlite was delegating to Python's built-in
  ``round`` which uses banker's rounding (round half to even), so
  ``round(0.5)`` returned 0.0 instead of SQLite's 1.0 and ``round(2.5)``
  returned 2.0 instead of 3.0.  The single-arg form now uses
  half-away-from-zero ties; the two-arg form rounds half-up on the
  exact IEEE 754 value (so ``round(2.355, 2) == 2.35`` because the
  stored double is ≈ 2.3549…).  Negative ``n`` is now clamped to 0
  (matching SQLite), and ``round(x, NULL)`` correctly returns NULL.

  9 oracle tests in ``test_tier3_round_half_away_from_zero.py`` pair
  each interesting input against the reference ``sqlite3`` module.

## [1.61.0] - 2026-05-20

### Fixed

- **Reject ``DISTINCT`` on multi-argument aggregates** — SQLite
  enforces a parse-time rule that any ``DISTINCT`` aggregate must take
  exactly one argument; calling ``group_concat(DISTINCT col, sep)``,
  ``string_agg(DISTINCT col, sep)``, or
  ``json_group_object(DISTINCT key, val)`` raises
  ``OperationalError: DISTINCT aggregates must have exactly one
  argument`` in the reference engine.

  mini-sqlite previously accepted all three forms silently — the
  separator/value was simply ignored or, worse, the dedup happened on
  the wrong column.  Latent bug surfaced while writing the 1.60.0
  oracle tests: ``test_tier3_group_concat_distinct.py`` had to skip
  the explicit-separator DISTINCT form because there was no way to
  compare against the reference engine (which rejected the query).

  Now both engines raise the same diagnostic; the adapter validates
  ``distinct and len(args) > 1`` in the ``GROUP_CONCAT`` / ``STRING_AGG``
  and ``JSON_GROUP_OBJECT`` branches and surfaces SQLite's exact
  message text.

  10 tests in ``test_tier3_distinct_aggregate_arity.py`` cover the
  rejected forms (group_concat / string_agg with separator,
  json_group_object), exact-text diagnostic pinning, and legal-form
  regressions (single-arg DISTINCT, multi-arg without DISTINCT,
  COUNT/SUM DISTINCT).

## [1.60.0] - 2026-05-19

### Fixed

- **``GROUP_CONCAT(DISTINCT expr)`` deduplication** — the DISTINCT
  modifier was silently dropped by the adapter on its way from the AST
  to the planner IR.  The parser correctly captured ``DISTINCT``, and
  both codegen (``InitAgg(distinct=...)``) and the VM (``_AggState``
  ``seen`` set) honoured the flag, but
  ``mini_sqlite.adapter._function_call``'s ``GROUP_CONCAT`` branch
  forgot to propagate ``distinct`` into the returned ``AggregateExpr``
  — so ``group_concat(DISTINCT x)`` behaved identically to
  ``group_concat(x)``.

  Latent bug surfaced while reviewing the ``STRING_AGG`` alias landed
  in 1.59.0; the COUNT/SUM/MIN/MAX branch in the same function was
  already correct.  One-line fix: thread ``distinct=distinct`` into
  the GROUP_CONCAT ``AggregateExpr`` (mirrors the existing pattern for
  the other aggregates).

  10 oracle tests in ``test_tier3_group_concat_distinct.py`` cover
  integer/string/all-same dedup, GROUP BY per-group dedup, NULL
  skipping with DISTINCT, all-NULL groups, ``string_agg`` alias
  equivalence, and non-DISTINCT regression guards.

## [1.59.0] - 2026-05-19

### Added

- **``STRING_AGG(expr, sep)`` aggregate** — SQLite 3.44+ synonym for
  ``GROUP_CONCAT``.  Standard SQL spells string aggregation with this
  name; mini-sqlite previously raised ``unknown scalar function:
  'string_agg'`` because the alias wasn't wired into the adapter's
  aggregate dispatch.  Routed through the same code path as
  ``GROUP_CONCAT`` (planner emits the same ``AggregateExpr`` IR; VM
  handles both via ``AggFunc.GROUP_CONCAT``).

  10 oracle tests in ``test_tier3_string_agg.py`` cover separator
  variants, GROUP BY composition, NULL skipping, all-NULL groups, and
  GROUP_CONCAT regression guards.

## [1.58.0] - 2026-05-19

### Added

- **``ORDER BY ... NULLS FIRST | NULLS LAST``** (via ``sql-lexer
  0.22.0``, ``sql-parser 0.29.0``).  SQLite 3.30+ explicit NULL
  placement::

      SELECT * FROM t ORDER BY val NULLS LAST;
      SELECT * FROM t ORDER BY val ASC NULLS FIRST;
      SELECT * FROM t ORDER BY val DESC NULLS LAST;
      SELECT * FROM t ORDER BY a NULLS LAST, b DESC NULLS FIRST;

  When omitted, mini-sqlite continues to use SQLite's defaults
  (NULLs first for ASC, NULLs last for DESC).

### Changed

- ``test_tier16_json_total.py::test_total_all_null`` renamed its
  internal ``nulls`` table to ``all_nulls`` because ``NULLS`` is now
  a keyword and can no longer appear as a bare identifier.  Use
  quoted identifiers (``"nulls"``) for legacy schemas that need the
  exact name, or pick a different name.  ``FIRST`` and ``LAST``
  remain valid identifiers (not keywords).

12 new oracle tests in ``test_tier3_order_by_nulls_first_last.py``
covering each direction × placement combination, multi-key ORDER BY
with per-key placement, positional ORDER BY + NULLS placement,
text-column null placement, and the default-placement regression
guards.

## [1.57.0] - 2026-05-19

### Added

Three SQLite 3.44+ string-family scalar functions now available (via
``sql-vm 1.35.0``):

- **``concat(...)``** — variadic concat; NULL args treated as empty.
- **``concat_ws(sep, ...)``** — concat with separator; NULL sep
  returns NULL but NULL values are skipped.
- **``octet_length(x)``** — byte length of UTF-8-encoded text or BLOB.

Application SQL relying on these (especially common in modern SQLite
code) previously raised ``InternalError: unknown scalar function``.

22 new oracle tests in ``test_tier3_concat_octet_length.py``:
two-/three-arg concat, NULL-as-empty semantics, numeric coercion,
single-arg form, in-WHERE-clause usage, concat_ws NULL-skipping vs
NULL-separator-propagation, multi-char separators, empty separator,
all-NULL inputs, ASCII vs non-ASCII byte counts, length vs
octet_length distinction for unicode, integer/negative coercion.

## [1.56.0] - 2026-05-19

### Fixed

- **``SELECT * ORDER BY N`` for N > 1** (via ``sql-planner 0.33.0``).
  Previously ``SELECT * FROM t ORDER BY 2`` raised
  ``InternalError: ValueError: tuple.index(x): x not in tuple``
  because the planner's positional-index validator treated the
  Wildcard as a single SELECT item.  Now the planner accepts any
  ordinal ≥ 1 when a wildcard is present and sets the position-
  based index directly.

  12 new oracle tests in
  ``test_tier3_order_by_positional_select_star.py`` cover the fix
  across single-table SELECT *, cross-joins, inner joins, multiple
  sort keys, mixed direction, and the regression-guard case of
  explicit projections.

## [1.55.0] - 2026-05-19

### Fixed

- **``SELECT *`` over RIGHT JOIN now emits columns in original FROM
  order** (via ``sql-codegen 1.32.0``).  Completes the SELECT-star /
  outer-join saga across the recent PRs:

  - #3600: derived table implicit AS
  - #3605: SELECT * across cross-join sources
  - #3612: LEFT JOIN unmatched-row NULL-padding
  - This PR: RIGHT JOIN column order

  Before::

      SELECT * FROM a RIGHT JOIN b ON a.id = b.id
      -- mini-sqlite: (b.id, b.y, a.id, a.x)    -- wrong order
      -- real SQLite: (a.id, a.x, b.id, b.y)

  12 new oracle tests in ``test_tier3_right_join.py`` cover matched,
  partial-match, no-match, empty-left, explicit-projection sanity,
  LEFT/FULL/INNER regression guards, and the derived-table /
  CTE-on-the-left variants.

## [1.54.0] - 2026-05-19

### Fixed

- **LEFT JOIN + ``SELECT *`` now NULL-pads unmatched rows correctly**
  (via ``sql-vm 1.34.0``).  Follow-up to PR #3605 which fixed the
  matched-row column count but left unmatched rows truncated.

  Before::

      SELECT * FROM a LEFT JOIN b ON a.id = b.id
      -- mini-sqlite (matched row):   (1, 'x', 1, 100)   ✓
      -- mini-sqlite (unmatched row): (2, 'y')           ✗ missing right cols
      -- real SQLite:                 (2, 'y', None, None)

  Now mini-sqlite returns the same NULL-padded row width as SQLite for
  every LEFT JOIN form: matched rows, unmatched rows, derived-table
  right sides, CTE right sides, and the cross-join + LEFT JOIN
  composition.

  Implementation: the VM now keeps a per-cursor column schema (cached
  at OpenScan time when the backend exposes ``columns()``, and lazily
  on first row otherwise).  When ``ScanAllColumns`` encounters a
  cursor with no current row, it emits ``None`` per cached column
  instead of bailing out.

  7 new oracle tests in ``test_tier3_left_join_null_pad.py`` cover
  partial matches, all-unmatched, wider right schemas, derived-table
  right sides, CTE right sides, and the cross-join composition.

## [1.53.0] - 2026-05-19

### Fixed

- **``SELECT *`` now returns columns from every FROM source**, not
  just the first one (via ``sql-codegen 1.31.0``).  The bug was in
  the Wildcard branch of ``_compile_project_body``, which called
  ``_primary_cursor`` and emitted ``ScanAllColumns`` for only the
  first opened cursor.  Cross-join queries silently dropped columns
  from later sources.

  Example::

      mini-sqlite (before): [(1,)]      -- only first table's columns
      mini-sqlite (now):    [(1, 2)]    -- matches real SQLite
      real SQLite:          [(1, 2)]

  Affected forms (all now correct):

      SELECT * FROM a, b
      SELECT * FROM (SELECT 1 AS x) t1, (SELECT 2 AS y) t2
      WITH x AS (...), y AS (...) SELECT * FROM x, y
      SELECT * FROM orders, customers WHERE ...

  11 new oracle tests in ``test_tier3_select_star_cross_join.py``
  cover the cross-join cases for plain tables, derived tables, CTEs,
  and the explicit JOIN-ON forms (regression guard).

  Known follow-up (separate PR): the LEFT JOIN unmatched-row case
  still needs NULL-padding work in the VM.  Matched rows are now
  fully correct; unmatched left rows return fewer columns than
  expected.

## [1.52.0] - 2026-05-19

### Fixed

- **Derived tables can now omit the ``AS`` keyword** (via ``sql-parser
  0.28.0``).  Both forms are accepted, matching SQLite::

      SELECT t.x FROM (SELECT 1 AS x) AS t   -- always worked
      SELECT t.x FROM (SELECT 1 AS x) t      -- now also works

  This is important for portability: a lot of application SQL written
  for real SQLite uses the implicit-AS form, especially in cross-join
  contexts like ``FROM (subq1) t1, (subq2) t2``.

  The fix is two-fold:

  - ``sql.grammar``: ``table_ref`` rule's derived-table branch now
    accepts an optional ``[ "AS" ]`` between ``)`` and the alias NAME.
  - ``adapter._table_ref``: scans for the first NAME token after the
    closing ``)`` (skipping any optional AS keyword) rather than
    requiring AS to immediately precede the alias.

  The alias itself is still required — bare ``FROM (SELECT 1)`` (no
  alias) raises ``ProgrammingError`` as before.

  8 new oracle tests in ``test_tier3_derived_table_implicit_as.py``
  cover single derived tables, comma-cross-join with derived tables on
  both sides (with and without AS, including mixed-style queries with
  three sources), and the nested-IN-subquery case that initially
  surfaced the bug.

## [1.51.0] - 2026-05-19

### Added

- **CTE ``MATERIALIZED`` / ``NOT MATERIALIZED`` hint** parser support
  (via ``sql-lexer 0.21.0``, ``sql-parser 0.27.0``).  SQLite 3.35+
  optimizer hint::

      WITH cte AS MATERIALIZED (SELECT 1 AS x) SELECT x FROM cte;
      WITH cte AS NOT MATERIALIZED (SELECT 1 AS x) SELECT x FROM cte;
      WITH RECURSIVE n(i) AS MATERIALIZED
          (SELECT 1 UNION ALL SELECT i+1 FROM n WHERE i<5)
          SELECT * FROM n;

  Mini-sqlite has no cost-based optimizer, so the hint is silently
  ignored at the adapter level (no code changes needed — the adapter
  already only pulls NAME, optional column aliases, and ``query_stmt``
  from the CTE node).

9 new oracle tests in ``test_tier3_cte_materialized.py`` pin
end-to-end equivalence with real ``sqlite3`` for the hint applied to
simple CTEs, CTEs with column aliases, recursive CTEs, and multi-CTE
queries with mixed hints.

## [1.50.0] - 2026-05-19

### Added

Six additional PRAGMAs that application code commonly probes when
running against real SQLite — all are oracle-verified round-trip
storage with no semantic effect on execution (mini-sqlite has no
on-disk file, no WAL, no thread pool):

- **`reverse_unordered_selects`** — bool, default 0
- **`cell_size_check`** — bool, default 0
- **`fullfsync`** — bool, default 0
- **`wal_autocheckpoint`** — int, default 1000
- **`journal_size_limit`** — int, default -1 (no limit)
- **`threads`** — int, default 0

The three integer-valued PRAGMAs (`wal_autocheckpoint`,
`journal_size_limit`, `threads`) echo their new value back on set —
a SQLite quirk where most `PRAGMA name = X` forms return an empty
result but these three return a one-row scalar.  Mini-sqlite now
matches that distinction byte-for-byte.

18 oracle tests in `test_tier3_pragma_audit_additions.py` pin both
default values and round-trip behaviour against `sqlite3`.

## [1.49.0] - 2026-05-19

### Added

Datetime modifier and strftime improvements surfaced end-to-end via
``sql-vm 1.33.0``:

- **Timezone offset modifiers** — ``+HH:MM``, ``-HH:MM``, ``+HH:MM:SS``
  shift the underlying datetime by the given offset.  Common pattern
  for converting UTC to a fixed display timezone::

      SELECT datetime('2024-03-15 14:30:00', '+02:00');
      -- → '2024-03-15 16:30:00'

- **``auto`` modifier** is now accepted as a no-op (SQLite 3.46+).
  Previously caused NULL propagation as an unrecognised modifier.

### Fixed

- **``%P`` strftime specifier** now returns ``'am'``/``'pm'`` on macOS.
  Python's macOS libc returns the literal ``'P'`` for ``%P``, which
  caused mini-sqlite to diverge from real ``sqlite3`` on macOS CI
  runners.  Pre-processing ``%P`` in ``sql-vm`` fixes the divergence.

17 new oracle tests in ``test_tier3_datetime_modifier_additions.py``
pin all three behaviours byte-for-byte against ``sqlite3``.

## [1.48.0] - 2026-05-19

### Added

Eleven additional scalar functions surfaced end-to-end (via
``sql-vm 1.32.0``), each pinned with oracle tests in
``test_tier3_scalar_fn_additions.py``:

- **Hyperbolic trig** — ``sinh``, ``cosh``, ``tanh``, ``asinh``,
  ``acosh``, ``atanh``.  Out-of-domain inputs return NULL.
- **``trunc(X)``** — truncate toward zero; distinct from ``floor`` for
  negative inputs.
- **Optimizer hints** — ``likely(X)``, ``unlikely(X)``,
  ``likelihood(X, Y)``.  All pass *X* through unchanged (no-op hints).
- **Compile-option probes** — ``sqlite_compileoption_used(name) → 0``
  and ``sqlite_compileoption_get(N) → NULL``, since mini-sqlite is
  not a compiled SQLite binary.

These close common gaps that application SQL written for real SQLite
relies on (math libraries that use hyperbolic trig, optimizer-hint
sprinkling, feature-detection probes).

## [1.47.0] - 2026-05-19

### Added

- **SQLite conditional-upsert (`ON CONFLICT … DO UPDATE SET … WHERE`)**
  fully supported end-to-end (via `sql-parser 0.26.0`,
  `sql-planner 0.32.0`, `sql-codegen 1.30.0`, `sql-vm 1.31.0`).  When the
  WHERE predicate evaluates false (or NULL) the row is left untouched —
  semantically equivalent to `DO NOTHING` for that single conflicting
  row.  Predicates may freely reference `EXCLUDED.col`, bare existing-row
  column names, and arbitrary compound boolean expressions::

      INSERT INTO inventory(id, qty) VALUES (1, 5)
      ON CONFLICT(id) DO UPDATE SET qty = excluded.qty
      WHERE excluded.qty > qty

  Ten oracle-verified tests in `test_tier10_upsert.py::TestUpsertConditionalWhere`
  cover both branches (predicate true / false / null), single- and
  multi-row scenarios, EXCLUDED-only refs, existing-only refs, compound
  predicates, NULL-as-false, and accumulators with a cap.

### Fixed

- **`EXCLUDED` pseudo-table name is now case-insensitive**
  (`adapter._rewrite_excluded`).  ``excluded.v`` and ``Excluded.v`` now
  rewrite to ``ExcludedColumn`` the same as ``EXCLUDED.v``, matching
  SQLite's case-insensitive identifier semantics.  The helper also
  descends through `UnaryExpr` so EXCLUDED references inside `NOT …` or
  unary-minus expressions in WHERE predicates are rewritten correctly.

## [1.46.0] - 2026-05-18

### Added

- **`STRICT` and `WITHOUT ROWID` table options** in CREATE TABLE (via
  `sql-lexer 0.20.0`, `sql-parser 0.25.0`).  Both syntactically accepted
  and silently ignored — mini-sqlite uses lenient type affinity regardless
  of STRICT, and its single storage model regardless of WITHOUT ROWID.

      CREATE TABLE t (id INTEGER) STRICT
      CREATE TABLE t (id INTEGER PRIMARY KEY) WITHOUT ROWID
      CREATE TABLE t (id INTEGER PRIMARY KEY) STRICT, WITHOUT ROWID

### Tests

- 12 new tests in `test_tier3_strict_without_rowid.py` cover all syntactic
  variants plus a regression test confirming that `rowid` remains a valid
  column reference in SELECT statements.

## [1.45.0] - 2026-05-18

### Added

- **`ATTACH DATABASE` / `DETACH DATABASE`** (`sql-lexer 0.19.0`,
  `sql-parser 0.24.0`, `engine.py`) — accept-and-no-op support for these
  SQLite statements.  Real SQLite uses ATTACH to mount additional
  databases under a schema name; mini-sqlite is a single-database engine
  and silently succeeds without actually attaching anything.  This unlocks
  ORM/migration code that issues these statements during connection setup
  or schema verification.

  Limitations (documented):

  - Multi-database queries (`SELECT * FROM aux.t`) are not supported.
  - `PRAGMA database_list` still reports only the `main` schema.

### Tests

- 9 new tests in `test_tier3_attach_detach.py` verify that both engines
  accept all four syntactic forms (`ATTACH DATABASE`/`ATTACH`,
  `DETACH DATABASE`/`DETACH`) and that round-trip patterns common in
  ORM code don't raise.

## [1.44.0] - 2026-05-18

### Added

- **Indexed expressions** (`sql-lexer 0.18.0`, `sql-parser 0.23.0`,
  `adapter.py`, `engine.py`) — `CREATE INDEX` now accepts arbitrary
  expressions in the column list, plus `COLLATE …` and the optional
  `WHERE` predicate of partial indexes:

      CREATE INDEX idx_lower    ON t(LOWER(name))
      CREATE INDEX idx_collate  ON t(name COLLATE NOCASE)
      CREATE INDEX idx_compound ON t(LOWER(name), id)

  Implementation:

  - The adapter's `_create_index` walks the `index_col`'s expression tree
    via the new `_extract_bare_column_name` helper.  Bare-column index
    columns create a real index (PRAGMA index_list registers it).
    Expression-based index columns are assigned a synthetic
    `__expr_<N>` placeholder name.
  - The engine intercepts `CreateIndexStmt` and, if any column name starts
    with `__expr_`, silently no-ops the index creation.  The SQL parses
    and the DDL succeeds; the lookup falls back to a table scan.

  COLLATE clauses are discarded (only BINARY is implemented).  ASC/DESC
  hints are discarded (B-tree indexes are bidirectional).

  This unlocks SQLAlchemy / Alembic / Django migration code that creates
  indexed expressions for case-insensitive search.  Lookups against the
  expression don't benefit from the index — but they remain correct.

### Tests

- 17 new oracle tests in `test_tier3_indexed_expressions.py` byte-compare
  against the real `sqlite3` module across bare columns, function-call
  expressions, COLLATE clauses, compound indexes, `IF NOT EXISTS`,
  `UNIQUE`, and direction hints.

## [1.43.0] - 2026-05-18

### Added

Three maintenance PRAGMAs that ORMs and migration tools commonly call:

- **`PRAGMA optimize`** / **`PRAGMA optimize(N)`** — silently succeeds with
  an empty result.  Real SQLite analyses statistics and may rebuild indexes;
  mini-sqlite has nothing to do (everything is in-memory).
- **`PRAGMA integrity_check`** / **`PRAGMA integrity_check(N)`** /
  **`PRAGMA integrity_check('table')`** — returns `[('ok',)]`, matching the
  result every healthy real SQLite database returns.  Mini-sqlite's
  in-memory storage cannot suffer the corruption modes SQLite checks for.
- **`PRAGMA quick_check`** / **`PRAGMA quick_check(N)`** — same as
  `integrity_check`, returns `[('ok',)]`.

### Changed

- `_PRAGMA_RE` now accepts numeric arguments in the parenthesised form
  (e.g. `PRAGMA optimize(0)`, `PRAGMA integrity_check(10)`), in addition
  to identifier arguments (e.g. `PRAGMA table_info(users)`).  The argument
  regex `[A-Za-z_0-9][A-Za-z0-9_]*` allows both styles.

### Tests

- 12 new oracle tests in `test_tier3_pragma_maintenance.py` byte-compare
  against the real `sqlite3` module across all six pragma forms.

## [1.42.0] - 2026-05-18

### Added

- **Connection-state scalar functions** (via `sql-vm 1.30.0`) — five new
  SQLite-compatible functions:
  `changes()`, `total_changes()`, `last_insert_rowid()`, `sqlite_version()`,
  `sqlite_source_id()`.
- **Engine wiring** (`engine.py`) — after every successful statement the
  engine calls `set_connection_state(...)` to propagate `rows_affected`
  and `last_inserted_rowid` from the VM result into the global state that
  backs the scalar functions.  Only DML statements bump the counters; SELECT
  leaves them unchanged.
- **`connect()` resets state** (`connection.py`) — opening a new
  `Connection` clears the connection-state globals so that fresh
  connections start with `changes() == 0` and friends, matching SQLite.

### Tests

- 12 new oracle tests in `test_tier3_connection_state_fns.py` byte-compare
  against the real `sqlite3` module across INSERT, multi-value INSERT,
  UPDATE, DELETE, repeated INSERT, integer-PK rowid pickup, and the
  zero-before-any-insert default.

## [1.41.0] - 2026-05-17

### Added

- **JSON path-shortcut operators `->` and `->>`** (SQLite 3.38+) end-to-end
  (via `sql-lexer 0.17.0`, `sql-parser 0.22.0`, `sql-vm 1.29.0`).  The
  adapter rewrites the operators in `_additive` into calls to two new
  internal scalar helpers: `j -> p` → `__json_arrow(j, p)` (JSON-typed
  result), `j ->> p` → `__json_arrow_text(j, p)` (SQL-typed result).
  Chained access (`j -> 'a' -> 'b'`) and explicit JSON paths
  (`j -> '$.a.b'`) both work.  Locked in by 23 new oracle tests in
  `test_tier3_json_arrow_ops.py`.

## [1.40.0] - 2026-05-17

### Added

Significant PRAGMA coverage expansion in `engine.py` `_run_pragma`:

**Read-only metadata pragmas (new):**

- `PRAGMA database_list` — returns `(0, 'main', '')`.  Mini-sqlite has no
  ATTACH support; only the main database exists.
- `PRAGMA collation_list` — returns the three standard SQLite collations
  (RTRIM, NOCASE, BINARY).  Only BINARY is actually implemented; the others
  are reported for introspection compatibility.
- `PRAGMA compile_options` — returns a representative list of compile-time
  options (ENABLE_JSON1, ENABLE_FTS5, ENABLE_RTREE, THREADSAFE=0).
- `PRAGMA function_list` — enumerates registered scalar functions and the
  built-in aggregates with the SQLite 6-column shape
  `(name, builtin, type, enc, narg, flags)`.
- `PRAGMA module_list` — returns empty (no virtual-table modules).

**Settable boolean pragmas (new):**

- `PRAGMA foreign_keys`, `recursive_triggers`, `legacy_alter_table`,
  `defer_foreign_keys`, `secure_delete` — all accept the full SQLite value
  space on write (`ON / OFF / 1 / 0 / TRUE / FALSE / YES / NO`,
  case-insensitive) and always return `0` or `1` on read.

**Settable integer pragmas (new):**

- `PRAGMA temp_store`, `synchronous`, `cache_size`, `auto_vacuum`,
  `application_id` — all accept signed integer literals on write
  (including the negative-value kibibyte convention for `cache_size`).

**Read-only integer pragmas (new):**

- `PRAGMA page_size`, `page_count`, `freelist_count` — return SQLite-default
  values; assignments are silently ignored (matching SQLite's behaviour for
  read-only DB-creation-time settings).

**Settable text pragmas (new):**

- `PRAGMA encoding`, `journal_mode`, `locking_mode` — defaults match
  SQLite (`UTF-8` / `memory` / `normal`).  `journal_mode` is locked to
  `memory` for in-memory databases (silently rejects WAL, DELETE, …),
  matching SQLite's actual behaviour on `:memory:`.

**Special:**

- `PRAGMA case_sensitive_like = ON|OFF` — write-only.  Reads always return
  empty (SQLite-compatible).  The flag does not yet affect LIKE evaluation;
  this is purely a parser/round-trip compatibility add.

**Write-form parsing:**

- `_PRAGMA_RE` now accepts bare-identifier values on the right of `=`
  (e.g. `PRAGMA journal_mode = WAL`), in addition to integer literals.

**Per-connection state:**

- Settable pragmas store their value in a process-level dict keyed by
  `id(backend)`, so each connection has its own independent value.
  29 new oracle tests in `test_tier3_pragma_additions.py` lock the
  byte-for-byte behaviour against the real `sqlite3` module.

## [1.39.0] - 2026-05-17

### Added

- **`LIKE … ESCAPE 'c'` clause** end-to-end (via `sql-lexer 0.16.0`,
  `sql-parser 0.21.0`, `sql-planner 0.31.0`, `sql-optimizer 0.12.0`,
  `sql-codegen 1.29.0`, `sql-vm 1.28.0`).  A new test file
  `test_tier3_like_escape.py` adds 16 oracle-comparison tests covering
  underscore/percent escapes, mixed escaped+unescaped wildcards, NOT LIKE
  with escape, WHERE-clause usage, and edge cases (self-escape, escape
  with no special chars).

### Changed

- **Adapter `_unquote_string`** (`adapter.py`) — removed backslash-escape
  processing.  SQLite treats backslashes inside string literals as literal
  characters; only `''` (doubled apostrophe) is an escape.  This restores
  byte-for-byte parity with the real `sqlite3` module and is required for
  `LIKE 'a\\_b' ESCAPE '\\'` to work (the backslash must survive parsing).

- **Binding `_repr_sql`** (`binding.py`) — `repr` of a Python `str` now
  uses doubled-quote escaping (`'O''Brien'`) instead of backslash
  escaping (`'O\\'Brien'`).  Updated the `substitute()` string-literal
  scanner accordingly.

## [1.38.0] - 2026-05-17

### Fixed

- **Four SQLite-compatibility scalar-function fixes** (via `sql-vm 1.27.0`):
  `time()` now parses bare time strings, `date(t, 'weekday N')` works,
  `log()` is base-10 (not natural), and `hex(N)` uses decimal-string bytes
  (matching `HEX("123")`).  New `test_tier3_scalar_fn_fixes.py` adds 22
  oracle-comparison tests against the real `sqlite3` module.

## [1.37.0] - 2026-05-17

### Fixed

- **SQLite-compatible NULL ordering** (`sql-codegen 1.28.0`, `sql-vm 1.26.0`) —
  `ORDER BY x` previously placed NULL rows at the end of the result; SQLite
  places them at the start (treating NULL as smaller than any non-NULL value).
  `ORDER BY x DESC` now correctly puts NULLs at the *end* (previously, due to
  a separate VM bug, they ended up at the start).

  10 new oracle tests in `test_tier3_null_ordering.py` lock the byte-for-byte
  match against the real `sqlite3` module across ASC, DESC, multi-key sorts,
  TEXT columns, and LIMIT interactions.

## [1.36.0] - 2026-05-16

### Added

- **`CREATE TEMP TABLE` / `CREATE TEMPORARY TABLE` support** — SQLite scripts
  commonly create temporary tables with the `TEMP` or `TEMPORARY` modifier.
  The engine now normalises `CREATE TEMP(ORARY)? TABLE` to `CREATE TABLE`
  and `CREATE TEMP(ORARY)? VIEW` to `CREATE VIEW` with a single regex
  substitution before the parser sees the SQL, so the grammar stays clean and
  `temp` remains a valid table/column name everywhere else.  Tested with
  17 new tests covering lower-case, mixed-case, `IF NOT EXISTS`, DROP, and
  the no-conflict property (table named `temp`, `DELETE FROM temp`, etc.).

- **Double-quoted identifier support** (`sql-lexer 0.15.0`) — `"colname"` and
  `"my column"` are now lexed as `NAME` tokens with the quotes stripped,
  matching SQLite's ANSI SQL identifier quoting.  Embedded `""` escape
  sequences are un-escaped correctly: `"it""s"` → `NAME("it's")`.

## [1.35.0] - 2026-05-15

### Fixed

- **`VARCHAR(N)` and parameterised column types** — `CREATE TABLE t(x VARCHAR(30))`
  previously failed with `Parse error at 1:1: Expected program, got 'CREATE'`.
  The SQL grammar now defines `col_type = NAME [ "(" NUMBER {"," NUMBER} ")" ]`
  and the adapter extracts the base type name from the `col_type` child node.

- **`IN ()` and `NOT IN ()` empty list** — `x IN ()` now parses and executes
  correctly, always returning `FALSE` (resp. `TRUE` for `NOT IN`).  The grammar
  makes `in_expr` optional; the adapter returns an `In`/`NotIn` with
  `values=()`; the VM's `_do_in_list` short-circuits to `False` for `n=0`.

- **`FROM t1, t2` implicit cross-join** — Comma-separated `FROM` clauses are now
  recognised.  Each comma-joined table is treated as a `CROSS JOIN` and produces
  the Cartesian product, matching standard SQL behaviour.

- **`CREATE INDEX` with `ASC`/`DESC` per column** — `CREATE INDEX idx ON t(a DESC)`
  previously raised a parse error.  The new `index_col` grammar rule accepts an
  optional `ASC` or `DESC` after each column name.

- **`ORDER BY alias` for computed expressions** — `SELECT a+b AS v4 FROM t ORDER
  BY v4` previously crashed with `InternalError: ValueError`.  The planner now
  treats bare alias references in `ORDER BY` as positional references (recording
  the 0-based index of the aliased SELECT item), avoiding the fallback display
  name `"?"` that caused the VM's sort to fail.

### Tests

- **`run_sqllogictest.py`** — New standalone SQLLogicTest runner that executes
  the full sqlite SQLLogicTest suite (`select1.test` through `select5.test`)
  against mini-sqlite.  Select tests pass at 100% with all fixes applied.

## [1.34.0] - 2026-05-15

### Fixed

- **`ORDER BY` column not in `SELECT` list** — Queries such as
  `SELECT name FROM employees ORDER BY salary` previously crashed with
  `InternalError: ValueError: tuple.index(salary): salary not in tuple`.

  The fix is entirely at the codegen layer: `_compile_read` now detects sort
  keys absent from the `Project`'s output, appends them as hidden trailing
  `ProjectionItem` entries, inserts a `StripTrailingColumns` instruction
  immediately after `SortResult`, and prefixes a corrected `SetResultSchema` so
  the VM's column list matches the extended row width during the sort phase.

  `SELECT *` projections are exempt (all table columns are present at runtime,
  and column names cannot be determined at compile time).

### Added

- **`WITH [RECURSIVE] cte_name(col, …) AS (…)` column alias list** — The SQL
  parser now accepts an explicit column-alias list after the CTE name:

  ```sql
  WITH RECURSIVE cnt(n) AS (
      SELECT 1
      UNION ALL
      SELECT n + 1 FROM cnt WHERE n < 5
  )
  SELECT n FROM cnt;
  ```

  Previously this produced `Parse error at 1:1: Expected program, got 'WITH'`
  when the column list was present (parser rejected `(n)` after the CTE name).

  Changes:
  - **Grammar** (`sql.grammar`): `cte_def` extended with optional
    `[ "(" NAME { "," NAME } ")" ]`.
  - **Parser** (`_grammar.py`): regenerated from the updated grammar.
  - **Adapter** (`adapter.py`): two new helpers — `_cte_col_aliases` extracts
    the column list from the `cte_def` AST node; `_apply_cte_col_aliases`
    adds `alias=` to the anchor's `SelectItem` objects so the planner derives
    the declared column names.  Both recursive and non-recursive CTEs benefit.

  Oracle-verified: 17 tests compare results against the real `sqlite3` module.

## [1.33.0] - 2026-05-14

### Added

- **`RETURNING` on `INSERT … SELECT`** — The RETURNING clause is now fully
  supported when the insert source is a sub-query rather than literal VALUES.
  Results are oracle-verified against real sqlite3:

  ```sql
  INSERT INTO log SELECT event, ts FROM events WHERE ts > ? RETURNING event, ts;
  INSERT OR IGNORE INTO t SELECT * FROM src RETURNING id, v;  -- skipped rows omitted
  INSERT INTO dst (a, b) SELECT x, y FROM src RETURNING a, b;
  ```

  One RETURNING row is emitted per successfully inserted row in insertion
  order.  Rows skipped by ON CONFLICT IGNORE do not appear in RETURNING.
  `cursor.description` is populated with the RETURNING column names.

  Implementation: `InsertFromResult` (sql-codegen IR) gains a
  `returning_columns` field; the VM's `_do_insert_from_result` snapshots
  source rows then repopulates `st.result` with RETURNING output after
  draining.  No new VM instructions required.

  17 new integration tests across `TestBasicReturning`, `TestDescription`,
  `TestOrderAndCardinality`, `TestOnConflictReturning`, `TestExplicitColumnList`.

## [1.32.0] - 2026-05-14

### Added

- **`FILTER (WHERE …)` on aggregate functions** — SQLite/SQL:2003 per-aggregate
  row predicates are now fully supported end-to-end across the pipeline:

  ```sql
  SELECT COUNT(*) FILTER (WHERE active = 1) FROM emp;
  SELECT dept, SUM(salary) FILTER (WHERE active = 1) FROM emp GROUP BY dept;
  SELECT
      COUNT(*) FILTER (WHERE dept = 'eng'),
      COUNT(*) FILTER (WHERE dept = 'sales')
  FROM emp;
  ```

  Supported on all aggregate functions: `COUNT(*)`, `COUNT(col)`, `SUM`, `AVG`,
  `MIN`, `MAX`, `GROUP_CONCAT`, `JSON_GROUP_ARRAY`, `JSON_GROUP_OBJECT`.
  Rows where the `FILTER` predicate is `FALSE` or `NULL` are silently skipped
  (matching SQLite semantics).  Multiple aggregates with different filter
  predicates in the same `SELECT` are independently accumulated.  Works with
  `GROUP BY`, outer `WHERE`, and `HAVING` clauses simultaneously.

  Implementation spans four packages:
  - `sql-parser 0.18.0`: new `filter_clause` grammar rule
  - `sql-planner 0.29.0`: `filter_expr` field on `AggregateExpr`/`AggregateItem`
  - `sql-codegen 1.24.0`: conditional `JumpIfFalse` skip block in update loop
  - `mini-sqlite/adapter.py`: extracts and converts the `filter_clause` AST node

## [1.31.0] - 2026-05-14

### Added

- **`json_group_array(val)` aggregate** — Fixed silent-bug where
  `json_group_array` was dispatched as a scalar (returning only the current
  row's value).  It is now a proper aggregate that accumulates non-NULL values
  into a JSON array, consistent with SQLite.  Returns `'[]'` for an empty group.
- **`json_group_object(key, val)` aggregate** — New SQLite-compatible aggregate
  that builds a JSON object from key-value pairs across a GROUP BY group.  Rows
  with NULL key or NULL value are silently skipped.  Duplicate keys: last writer
  wins.  Returns `'{}'` for an empty group.
- **`VACUUM`, `ANALYZE`, `REINDEX`, `EXPLAIN` stubs** (`engine.py`) — These
  statements previously raised `ProgrammingError` because the SQL parser does
  not recognise them.  They are now intercepted by a regex match in `engine.py`
  (like `PRAGMA`) and silently return `QueryResult(rows_affected=0)`.  This
  matches the expectations of migration tools and ORM setup routines that call
  these statements unconditionally.

## [1.30.0] - 2026-05-13

### Added

- **Tier 16 — JSON1 scalar functions** — 14 SQLite-compatible JSON functions
  are now recognised by the mini-sqlite pipeline end-to-end:

  | Function | Description |
  |---|---|
  | `json(x)` | Canonical (minified) JSON |
  | `json_valid(x)` | 1 / 0 / NULL validation |
  | `json_quote(x)` | SQL value → JSON text |
  | `json_array(v…)` | Build JSON array |
  | `json_object(k, v…)` | Build JSON object |
  | `json_extract(json, path…)` | Extract one or more paths |
  | `json_type(json [, path])` | Type name at path |
  | `json_array_length(json [, path])` | Array length (0 for non-arrays) |
  | `json_keys(json [, path])` | Object keys as JSON array |
  | `json_patch(target, patch)` | RFC 7396 merge patch |
  | `json_remove(json, path…)` | Remove paths |
  | `json_set(json, path, val…)` | Insert or replace paths |
  | `json_insert(json, path, val…)` | Insert only (no overwrite) |
  | `json_replace(json, path, val…)` | Replace only (no insert) |

  Implemented in `sql_vm.scalar_functions`; no parser/planner/codegen
  changes required — JSON functions are dispatched via `CallScalar`.

  82 oracle-verified tests in `tests/test_tier16_json_total.py`.

- **`TOTAL()` aggregate** — SQLite-specific aggregate function that returns
  `0.0` (float) instead of NULL for empty groups or all-NULL input.  Follows
  the SQLite documentation: "If there are no non-NULL input rows then
  `TOTAL()` returns 0.0."  Added `AggFunc.TOTAL` to the planner (`expr.py`),
  codegen IR (`ir.py`), and VM (`vm.py`); registered in the adapter's
  `agg_map` so `SELECT TOTAL(col) FROM t ...` queries compile correctly.

  8 oracle-verified tests cover normal summation, empty-table, all-NULL, and
  `GROUP BY` with TOTAL, plus a contrast test showing SUM returns NULL where
  TOTAL returns 0.0.

## [1.29.0] - 2026-05-13

### Added

- **Tier 15 — Window frame clause (`ROWS BETWEEN … AND …`)** — SQL-standard
  window frame bounds are now parsed, planned, threaded through the IR, and
  evaluated in the VM.  This fixes correctness of running aggregates (SUM, COUNT,
  AVG, MIN, MAX) and the value functions FIRST_VALUE, LAST_VALUE, NTH_VALUE when
  an explicit `ROWS BETWEEN … AND …` or the implicit SQL-standard defaults apply.

  **Grammar changes** (`sql.tokens`, `sql.grammar`):
  - Seven new keywords: `ROWS`, `RANGE`, `GROUPS`, `PRECEDING`, `FOLLOWING`,
    `UNBOUNDED`, `CURRENT` — these tokenise as `KEYWORD` so the parser can match
    them case-insensitively.
  - New grammar rules: `frame_clause`, `frame_unit`, `frame_bound` attached to
    `window_spec`.

  **sql-planner 0.26.0** — `FrameBound` and `WinFrame` dataclasses carry frame
  semantics from the adapter through to `WindowFuncSpec.frame` and
  `WindowFuncExpr.frame`.

  **sql-codegen 1.21.0** — `WinFuncSpec.frame` re-exports `WinFrame`; compiler
  copies the field through verbatim.

  **sql-vm 1.20.0** — `_frame_slice(partition, i, spec)` helper maps frame bounds
  to Python slice indices; all running aggregates and value functions call it
  per-row instead of using a fixed slice.

  **mini-sqlite adapter** — `_frame_clause(node)` walks the AST frame_clause node,
  using recursive `_find_number` to locate offset tokens deep inside the expression
  grammar tower, and returns a `WinFrame`.

### Fixed

- **Running SUM / COUNT / AVG / MIN / MAX with `ORDER BY`** — previously all
  aggregates used the full partition regardless of ORDER BY.  They now correctly
  apply the SQL-standard cumulative default (equivalent to `ROWS BETWEEN
  UNBOUNDED PRECEDING AND CURRENT ROW`) when an ORDER BY clause is present.

- **LAST_VALUE default frame** — `LAST_VALUE` previously always returned the last
  value in the full partition.  It now returns the last value in the current frame
  window (cumulative by default with ORDER BY), matching SQLite behaviour.

- **NTH_VALUE with frame offset** — the offset integer was not being extracted
  from deeply nested AST expression nodes.  Fixed with a recursive `_find_number`
  tree walk in the adapter.

### Tests

- **`test_tier15_window_frames.py`** (34 tests, 10 classes) — comprehensive
  oracle-verified tests covering: running SUM/COUNT/AVG/MIN/MAX with and without
  ORDER BY, explicit `ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW`,
  `ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING`, sliding windows
  (`N PRECEDING`), RANGE BETWEEN syntax, and all ranking functions verified
  unaffected.

## [1.28.0] - 2026-05-13

### Fixed

- **`IS DISTINCT FROM` / `IS NOT DISTINCT FROM` operators** — SQL:1999 NULL-safe
  equality comparisons now work end-to-end.  These operators never return NULL:
  `NULL IS DISTINCT FROM NULL` → `FALSE`, `NULL IS DISTINCT FROM 1` → `TRUE`,
  `1 IS NOT DISTINCT FROM 1` → `TRUE`, etc.

  Four-layer fix:
  1. `sql-parser 0.17.0` — grammar extended with two new comparison suffix
     alternatives; adapter emits `BinaryExpr(op=BinaryOp.IS_DISTINCT_FROM, …)`.
  2. `sql-planner 0.25.0` — two new `BinaryOp` enum values pass through unchanged.
  3. `sql-codegen 1.20.0` — two new `BinaryOpCode` enum values added; both mapped
     in `_BINOP_MAP` so the generic `_compile_expr` path handles them.
  4. `sql-vm 1.19.0` — `apply_binary` handles both opcodes *before* the general
     `if left is None or right is None: return None` short-circuit, which is
     critical because these operators must see NULL operands directly.
  5. `sql-optimizer 0.10.0` — constant folding handles `IS DISTINCT FROM` before
     the generic NULL propagation guard to avoid incorrectly folding
     `NULL IS DISTINCT FROM 1` to `NULL`.

- **Aggregate SELECT aliases now resolve in HAVING and ORDER BY** — queries like
  `SELECT SUM(amount) AS total FROM t HAVING total > 100 ORDER BY total` previously
  raised `ColumnNotFound: total`.  `sql-planner 0.25.0` introduces
  `_substitute_aliases` which rewrites alias references in HAVING / ORDER BY
  expressions to their source aggregate expressions before column resolution.

- **`MAX()` / `MIN()` scalar functions propagate NULL** — `SELECT MAX(NULL, 1)`
  now returns `NULL`, matching SQLite.  Previously the NULL argument was silently
  dropped.

- **`ABS()` on non-numeric text returns `0.0`** — `SELECT ABS('hello')` now
  returns `0.0` to match SQLite's coercion behaviour.  Previously passed the string
  through unchanged.

- **`HEX(NULL)` returns `''`** — `SELECT HEX(NULL)` now returns an empty string
  instead of `None`, matching SQLite.

- **`date()` month arithmetic overflow** — adding months that push the day past the
  last day of the resulting month (e.g. `date('2024-01-31', '+1 month')`) now
  overflows into the next month (`2024-03-02`) rather than clamping to `2024-02-29`.
  This matches SQLite's behaviour.

### Tests

- Added `tests/test_tier14_convergence.py` — 33 oracle-grade integration tests
  comparing mini-sqlite against real `sqlite3` for all six fixed patterns:
  - `TestIsDistinctFrom` (8 tests)
  - `TestIsNotDistinctFrom` (8 tests)
  - `TestAggregateAliasInHavingAndOrderBy` (5 tests)
  - `TestScalarMaxMinNull` (4 tests)
  - `TestAbsNonNumeric` (3 tests)
  - `TestHexNull` (3 tests)
  - `TestDateMonthOverflow` (2 tests)

## [1.27.0] - 2026-05-13

### Fixed

- **`x % 0` returns NULL** — `SELECT 5 % 0` now returns `NULL` instead of
  crashing.  Fix is in `sql-vm 1.18.0`.

- **Doubled-quote `''` escape in string literals** — SQL strings containing
  `''` (the ANSI standard way to embed a single quote) now parse and store
  correctly.  `INSERT INTO t VALUES (1, 'O''Brien')` stores `O'Brien`.

  Two-layer fix:
  1. `sql-parser 0.16.0` — updated `STRING_SQ` token regex to `'(''|[^'\\]|\\.)*'`
  2. `mini_sqlite.adapter._unquote_string` — updated to process `''` → `'` on the
     already-quote-stripped token body received from the sql-lexer.

- **`COUNT(DISTINCT col)` / `SUM(DISTINCT col)` deduplication** — aggregate
  functions with `DISTINCT` now correctly deduplicate values before accumulating.
  Previously the `DISTINCT` flag was parsed and propagated through the plan but
  ignored in the VM, causing `COUNT(DISTINCT col)` to behave like `COUNT(col)`.

  Fix is in `sql-codegen 1.19.0` (new `InitAgg.distinct` field) and
  `sql-vm 1.18.0` (`_AggState.seen` deduplication set).

- **`REPLACE(str, from, to)` scalar function** — `REPLACE` is a SQL keyword used
  for `REPLACE INTO` DML.  The grammar previously rejected `REPLACE(...)` as a
  function call.  `sql-parser 0.16.0` extends `function_call` to allow `REPLACE`
  as a function name alongside the existing `NAME` alternative.

  The adapter's `_function_call` is updated to recognise `KEYWORD` tokens with
  value `REPLACE` as the function name.

### Tests

- Added `tests/test_tier13_convergence.py` — 30 oracle-grade integration tests
  comparing mini-sqlite against real `sqlite3` for all four fixed patterns:
  - `TestModuloByZero` (6 tests)
  - `TestDoubledQuoteEscape` (7 tests)
  - `TestCountDistinct` + `TestSumDistinct` + `TestCountDistinctWithGroupBy` (12 tests)
  - `TestReplaceFunction` (8 tests — including regression checks for DML syntax)

## [1.26.0] - 2026-05-12

### Fixed

- **GROUP BY + HAVING duplicate aggregate column bug** — queries of the form
  `SELECT cat, SUM(val) FROM t GROUP BY cat HAVING SUM(val) > N` previously
  returned an extra spurious column (e.g. `('A', 3, 3)` instead of `('A', 3)`).

  Root cause: `_collect_aggregates` in `sql_planner` created a separate
  `AggregateItem` slot for each textual occurrence of an aggregate expression,
  so `SUM(val)` in the SELECT list and `SUM(val)` in the HAVING predicate each
  got their own `_agg_N` alias.  Codegen then emitted two `EmitColumn`
  instructions for the row — one per slot — doubling the value.

  Fix: deduplicate aggregate expressions by a `(func, arg, distinct, separator)`
  key inside `_collect_aggregates`.  Duplicate occurrences reuse the slot created
  at first encounter.  The fix is in `sql-planner 0.24.0`.

### Tests

- Added `tests/test_tier12_aggregate_convergence.py` — oracle-grade integration
  tests that compare mini-sqlite results against real `sqlite3` for all affected
  query patterns:
  - `SUM`, `COUNT(*)`, `MAX`, `AVG` shared between SELECT and HAVING
  - Aggregate only in HAVING (not in SELECT)
  - Two different aggregates with one referenced by HAVING
  - HAVING that matches no groups (empty result)
  - HAVING + ORDER BY on the same aggregate

## [1.25.0] - 2026-05-12

### Added — ROWID pseudo-column (`rowid` / `_rowid_` / `oid`)

Full end-to-end rowid support matching SQLite's behaviour.

**SQL supported:**

```sql
SELECT rowid FROM t                    -- basic pseudo-column in SELECT list
SELECT _rowid_ FROM t                  -- alias synonym
SELECT oid FROM t                      -- alias synonym
SELECT t.rowid FROM t                  -- table-qualified reference
SELECT rowid, name FROM items          -- rowid alongside real columns
SELECT * FROM t                        -- rowid NOT included (implicit only)
SELECT rowid, val FROM t WHERE rowid = 2     -- filter by rowid
SELECT rowid, msg FROM log WHERE rowid > 2   -- range filter / pagination
SELECT rowid, val FROM t WHERE rowid BETWEEN 2 AND 4
SELECT val FROM t WHERE oid = 1        -- oid alias in WHERE
DELETE FROM t WHERE rowid = 2          -- delete by rowid
DELETE FROM t WHERE rowid > 2          -- range delete
SELECT rowid, val FROM t ORDER BY rowid ASC
SELECT rowid, val FROM t ORDER BY rowid DESC
```

**Semantics:**

- Rowids are 1-based, stable integers assigned at insert time and never reused
  (surviving rows keep their original rowid after DELETE — matches SQLite).
- `SELECT *` does not include the implicit rowid; `SELECT rowid, *` requires
  an explicit `rowid` reference (parser limitation noted; test skipped).
- `WHERE rowid = N` with no matching row returns an empty result set, not an
  error.

**Implementation layers touched:**

| Layer | Change |
|-------|--------|
| `sql-backend` 0.12.0 | Stable `"\x00rowid"` hidden field stamped at insert; `ListRowIterator.rowid()` / `ListCursor.rowid()` |
| `sql-planner` 0.23.0 | `RowIdRef` expr node; rowid alias resolution in `_resolve_column` |
| `sql-optimizer` 0.9.0 | Predicate pushdown recognises `RowIdRef` |
| `sql-codegen` 1.18.0 | `LoadRowId` IR instruction; `RowIdRef` → `LoadRowId` compilation |
| `sql-vm` 1.17.0 | `LoadRowId` dispatch; hidden-key filtering in `SELECT *` |

**Tests added:**

- `tests/test_tier11_rowid.py` — 19 oracle-grade integration tests (1 skipped)
  comparing mini-sqlite output against real `sqlite3` module for every rowid
  pattern listed above.

## [1.24.0] - 2026-05-05

### Added — UPSERT (`ON CONFLICT DO UPDATE / DO NOTHING`)

Full end-to-end support for modern SQLite 3.24+ upsert syntax.

**SQL syntax supported:**

```sql
-- Silently skip conflicting rows
INSERT INTO t VALUES (1, 'x') ON CONFLICT DO NOTHING;
INSERT INTO t (id, val) VALUES (1, 'x') ON CONFLICT (id) DO NOTHING;

-- Update the conflicting row in-place
INSERT INTO t VALUES (1, 'new')
  ON CONFLICT (id) DO UPDATE SET val = EXCLUDED.val;

-- Arithmetic accumulation using both existing and excluded values
INSERT INTO inventory VALUES (1, 5)
  ON CONFLICT (id) DO UPDATE SET qty = qty + EXCLUDED.qty;

-- Multiple SET assignments
INSERT INTO t VALUES (1, 'a2', 'b2')
  ON CONFLICT (id) DO UPDATE SET a = EXCLUDED.a, b = EXCLUDED.b;
```

**Pipeline changes:**

- **Grammar / tokens** (`sql.grammar`, `sql.tokens`) — new `CONFLICT`, `DO`,
  `NOTHING` keywords; new `upsert_clause` rule attached to `insert_stmt`.  Parser
  regenerated (`_grammar.py`).

- **`adapter.py`** (`_upsert_clause`, `_rewrite_excluded`) — parses the
  `upsert_clause` AST node, rewrites `Column(table="EXCLUDED", col=c)` references
  to `ExcludedColumn(col=c)`, and returns a `BackendUpsertClause` for the planner.
  Integrated into `_insert()`.

- **`sql-planner`** — `ExcludedColumn` expression node; `UpsertClause` and
  `UpsertAssignment` AST nodes; `UpsertAction` and `UpsertAssignment` plan nodes;
  `_resolve_upsert()` / `_resolve_upsert_expr()` in the planner.

- **`sql-optimizer`** — `_fold_upsert()` helper preserves `UpsertAction` through
  constant folding.

- **`sql-codegen`** — `UpsertSpec`, `UpsertAssignment`, `LoadExcludedColumn` IR
  nodes; `_compile_upsert()` + `ExcludedColumn` case in `_compile_expr()`.

- **`sql-vm`** — `excluded_row` state; `LoadExcludedColumn` dispatch; `_do_upsert()`
  (DO NOTHING fast path + DO UPDATE cursor scan); `_upsert_apply()` (evaluates SET
  expressions, temporarily parks the existing row for bare column refs, calls
  `backend.update()`).

**Tests (`tests/test_tier10_upsert.py`)** — 15 oracle-verified tests comparing
mini-sqlite against real `sqlite3` across DO NOTHING (with/without conflict,
UNIQUE columns, selective skipping) and DO UPDATE (EXCLUDED.col, arithmetic,
multiple columns, literal SET, counter accumulation, no-conflict plain inserts).

## [1.23.0] - 2026-05-05

### Added — DEFAULT column values (end-to-end)

Full end-to-end support for `DEFAULT <literal>` column constraints.  When a
column is declared with `DEFAULT <value>` and an INSERT omits that column,
the backend fills the row with the declared default instead of `NULL`.

**Pipeline changes:**

- **`mini-sqlite/adapter.py`** (`_col_def`) — after parsing column
  constraints, detects `DEFAULT primary` and calls `_primary()` to extract
  the literal value.  `Literal` results (integer, float, string, `None`) are
  stored directly; any non-literal expression (function call, parenthesised
  expression, etc.) is silently ignored and falls back to `NO_DEFAULT`.  The
  resulting `BackendColumnDef` now includes `default=col_default`.

- **`sql-codegen/ir.py`** — added `NO_COLUMN_DEFAULT` sentinel (`Final`
  singleton) and `default: object = NO_COLUMN_DEFAULT` field on `ColumnDef`.
  The sentinel is distinct from `sql_backend.schema.NO_DEFAULT` to keep the
  IR layer free from backend imports.

- **`sql-codegen/compiler.py`** (`_to_ir_col`) — converts the backend
  `NO_DEFAULT` sentinel to the IR `NO_COLUMN_DEFAULT`, passes all other
  values through verbatim.

- **`sql-vm/vm.py`** (`_do_create_table`) — converts `NO_COLUMN_DEFAULT` back
  to `NO_DEFAULT` when building `BackendColumnDef`, passes real default values
  through.  `InMemoryBackend._apply_defaults()` uses these values to fill
  omitted columns at INSERT time.

**Supported DEFAULT literal forms:**
  - Integer: `DEFAULT 0`, `DEFAULT 42`, `DEFAULT 1`
  - Real: `DEFAULT 3.14`
  - Text: `DEFAULT 'active'`
  - Null: `DEFAULT NULL`

**Not yet supported** (planned follow-on): `DEFAULT -1` (bare negative integer
requires grammar/adapter support for unary-minus signed literals; use
`DEFAULT (-1)` as a workaround, though this currently also falls back to
`NO_DEFAULT` since the adapter only materialises `Literal` nodes).

**Tests:** `tests/test_tier9_column_defaults.py` — 27 oracle-verified tests
across 7 test classes covering integer/null/text defaults, NOT NULL+DEFAULT,
SELECT *, UNIQUE+DEFAULT, and edge cases.  Coverage remains ≥ 91%.

## [1.22.0] - 2026-05-04

### Added — INSERT OR REPLACE, INSERT OR IGNORE, REPLACE INTO

Full end-to-end support for SQLite's conflict-resolution INSERT syntax.  Every
layer of the pipeline was extended: grammar → lexer → parser → adapter →
planner → optimizer → codegen → VM.

- **`INSERT OR REPLACE INTO t VALUES …`** — if the new row conflicts on any
  UNIQUE or PRIMARY KEY column, all conflicting existing rows are deleted and
  the new row is inserted.  Exactly matches SQLite's `INSERT OR REPLACE`
  semantics.

- **`REPLACE INTO t VALUES …`** — syntactic sugar for `INSERT OR REPLACE INTO`.
  Parsed by the new `replace_stmt` grammar rule; the adapter maps it to
  `on_conflict="REPLACE"`.

- **`INSERT OR IGNORE INTO t VALUES …`** — if the new row would violate a
  UNIQUE or PRIMARY KEY constraint, the row is silently skipped.  Rows with no
  conflict are inserted normally.

- **`INSERT OR ABORT INTO t VALUES …`** — explicit form of the default
  behaviour: raises `IntegrityError` on constraint violation.

- **`INSERT OR REPLACE / IGNORE … SELECT …`** — conflict resolution also works
  for `INSERT … SELECT` forms.

- **UNIQUE column constraints now enforced for plain `INSERT`** — a latent bug
  where `col TEXT UNIQUE` constraints were silently ignored by the in-memory
  backend (and therefore by `mini_sqlite.connect(":memory:")`) has been fixed.
  The UNIQUE flag now flows correctly through: `sql_backend.schema.ColumnDef`
  → IR `ColumnDef` (new `unique` field) → `BackendColumnDef` created by the VM
  `CreateTable` handler.

### Tests

- 17 oracle-verified tests in `tests/test_tier8_insert_conflict.py` run the
  same SQL on both mini-sqlite and the real `sqlite3` module and assert the
  results are identical.  Covers: single-key REPLACE, multi-REPLACE,
  non-key-column REPLACE, UNIQUE column REPLACE/IGNORE, mixed rows, REPLACE
  INTO shorthand, `INSERT … SELECT` forms, and ABORT (default) behaviour.

## [1.21.0] - 2026-05-04

### Added — String concatenation, JOIN USING, NATURAL JOIN

- **`||` string concatenation** — SQL's standard string-concat operator is now
  fully supported end-to-end: grammar (`sql.tokens` / `sql.grammar` via
  `CONCAT_OP = "||"`) → lexer → parser → adapter (`_additive` maps
  `CONCAT_OP → BinaryOp.CONCAT`) → planner → optimizer (constant-folds
  `'hello' || 'world' → 'helloworld'`) → codegen → VM.  NULL propagates:
  `NULL || 'x'` → NULL.

- **`JOIN … USING (col, …)`** — USING syntax is now parsed and correctly
  desugared for two-table and chained three-table join cases.  The adapter
  emits `JoinClause(using=(...))` (instead of a pre-built ON expression), and
  the planner's `_build_from_tree` resolves each USING column against the full
  accumulated scope.  This is essential for three-table chains like
  `a JOIN b USING (x) JOIN c USING (y)` where `y` may live in `a`, not `b`.
  Supports INNER, LEFT, and all other join kinds.

- **`NATURAL JOIN`** — automatically equates all shared column names between
  the left scope and the right table.  Resolved in the planner where schema
  access is available.  Falls back to CROSS JOIN when no shared columns exist
  (matching SQLite semantics).  Grammar adds `NATURAL` keyword and
  `join_type` alternative; adapter emits `JoinKind.NATURAL`.

### Fixed

- **`ConstantFolding` silent NULL for `||`** — `constant_folding.py`'s
  `_apply_binary` had no case for `BinaryOp.CONCAT`, causing Python's pattern
  matching to silently return `None` and fold `'hello' || 'world'` to
  `Literal(None)`.  Now fixed.

### Tests

- `tests/test_tier7_string_and_joins.py` — 25 new oracle-verified tests
  covering `||` (10 cases: literals, columns, NULL, WHERE, alias, constant
  folding, nullable columns), `JOIN USING` (6 cases: single-column, no
  matches, multi-column, WHERE filter, LEFT JOIN, three-table chain), and
  `NATURAL JOIN` (7 cases: single shared column, no unmatched rows, multiple
  shared columns, empty right table, no shared columns → CROSS, WHERE filter,
  aliased table), plus 2 cross-feature tests combining `||` with JOIN.

## [1.20.0] - 2026-05-04

### Added — SQLite convergence (parser + runtime)

This release closes four parser-level gaps between mini-sqlite and real SQLite,
plus two correctness fixes in the shared VM runtime.

**SELECT without FROM** (`sql.grammar`, `sql-planner`, `sql-codegen`, `adapter.py`):
- The FROM clause is now optional in the grammar (`select_stmt`).
- The planner emits `SingleRow()` when `from_` is `None`; the codegen runs
  the body exactly once with no cursor loop, no AdvanceCursor, no CloseScan.
- `SELECT 1`, `SELECT UPPER('hello')`, `SELECT 1 + 1 WHERE 1 = 1` all work.

**CAST(expr AS type)** (`sql.grammar`, `sql.tokens`, `adapter.py`):
- `CAST` is now a grammar keyword with its own `cast_expr` rule so the `AS`
  inside it is never confused with a column alias.
- Adapter maps `cast_expr` to the existing `cast` scalar function
  (`FunctionCall(name='cast', args=[expr, Literal(type_name)])`).

**Table alias without AS** (`sql.grammar`, `adapter.py`):
- `FROM employees e` now accepted in addition to `FROM employees AS e`.
- Bare-NAME alias detection uses a `saw_table_name` flag to avoid eating
  SQL keywords (WHERE, JOIN, ON …) as alias names.

**GLOB operator** (`sql.grammar`, `sql.tokens`, `adapter.py`, `sql-vm`):
- `name GLOB '*.py'` and `name NOT GLOB '*.py'` are now supported.
- Compiles to `FunctionCall(name='glob', args=[pattern, string])` in the
  `glob(pattern, string)` argument order matching SQLite's C API.
- New `glob` scalar function in `sql-vm` using `fnmatch.fnmatchcase` for
  case-sensitive Unix-style pattern matching.

**Plain JOIN (= INNER JOIN)** (`sql.grammar`, `adapter.py`):
- `join_type` is now optional in `join_clause`; a bare `JOIN` keyword
  defaults to `JoinKind.INNER`.

### Fixed

- **LIKE is now case-insensitive** (`sql-vm`) — ANSI SQL and SQLite both
  treat LIKE as case-insensitive by default for ASCII. `like_match` now
  folds both value and pattern to lowercase before the DP comparison.
- **`JumpIfFalse`/`JumpIfTrue` use SQL truthiness** (`sql-vm`) — previously
  only Python `False` was treated as falsy; now `0`, `0.0`, and `None` are
  also falsy, fixing GLOB (which returns int 0/1) in WHERE clauses.

## [1.19.0] - 2026-05-04

### Added

- **`GROUP_CONCAT` end-to-end support** (`adapter.py`) — the SQL adapter
  now recognises `GROUP_CONCAT(col)` and `GROUP_CONCAT(col, separator)`,
  emitting `AggregateExpr(func=AggFunc.GROUP_CONCAT, separator=…)`.
  - Zero or 3+ arguments raise `ProgrammingError` at parse time.
  - The separator must be a string literal; non-literal separators raise
    `ProgrammingError`.
- **15 new GROUP_CONCAT tests** (`tests/test_tier5_group_concat.py`) —
  covering default and custom separators, per-group concatenation, numeric
  column values, NULL handling (skip / all-NULL → NULL / empty table → NULL),
  oracle comparison against the real `sqlite3` module, and error cases.

## [1.18.0] - 2026-05-04

### Added

- **LAG / LEAD window functions** — `LAG(col [, offset [, default]])` and
  `LEAD(col [, offset [, default]])` are now fully supported end-to-end.
  The adapter (`adapter.py`) extracts `exprs[1:]` from the `value_list`
  grammar node into `WindowFuncExpr.extra_args`; codegen normalises these
  to an `(offset, default)` pair; the VM evaluates the offset-lookback or
  lookahead within each ordered partition.
- **NTILE(n) window function** — `NTILE(n)` divides each partition into `n`
  numbered buckets (1..n) using the standard `divmod` distribution rule.
  The integer literal `n` is parsed as the sole argument to NTILE.
- **PERCENT_RANK() window function** — `PERCENT_RANK()` computes
  `(rank − 1) / (N − 1)`.  Argument-free; only `OVER (ORDER BY ...)` is
  required.  Returns `0.0` for single-row partitions.
- **CUME_DIST() window function** — `CUME_DIST()` computes the cumulative
  distribution fraction for each row's peer group.  Also argument-free.
- **NTH_VALUE(col, n) window function** — `NTH_VALUE(col, n)` returns the
  value of `col` at the n-th row (1-indexed) of the partition.  Returns
  `NULL` when the partition has fewer than n rows.
- **Negated literal folding in window extra args** (codegen) — SQL expressions
  like `LAG(col, 1, -1)` where `-1` is parsed as `UnaryExpr(NEG, Literal(1))`
  are now constant-folded to `-1` by the codegen `_literal_val` helper,
  making negative default values work correctly.

## [1.17.0] - 2026-05-04

### Added

- **RETURNING clause** — `INSERT`, `UPDATE`, and `DELETE` statements now
  support a trailing `RETURNING col1, col2, ...` clause that returns the
  affected rows as a result set, exactly like SQLite's `RETURNING` extension.
  - **INSERT RETURNING** — returns the inserted row(s); `cursor.description`
    is set, `cursor.fetchall()` / `cursor.fetchone()` work as with SELECT.
  - **UPDATE RETURNING** — returns the post-update row values for each
    matched row.
  - **DELETE RETURNING** — captures row values *before* deletion; the rows
    are gone from the table by the time the cursor is consumed.
  - The adapter (`adapter.py`) extracts the `returning_clause` AST child and
    passes a `returning=(expr, ...)` tuple to the statement constructors.
  - 17 integration tests in `tests/test_tier4_returning.py` covering single-
    row, multi-row, single- and multi-column, description header, rowcount,
    value-persistence, and empty-result cases for all three DML statements.

## [1.16.0] - 2026-05-04

### Added

- **Correlated subquery execution** — end-to-end support for subqueries
  whose WHERE clause references columns from the enclosing query (correlated
  subqueries).  The adapter, planner, codegen, and VM cooperate to re-execute
  the inner program for each outer row with the outer cursor's current snapshot.
  Supported forms:
  - `WHERE e.col IN (SELECT ... FROM t WHERE t.x = e.col)` — correlated IN
  - `WHERE e.col NOT IN (SELECT ... FROM t WHERE t.x = e.col)` — correlated NOT IN
  - `WHERE EXISTS (SELECT 1 FROM t WHERE t.x = e.col)` — correlated EXISTS
  - `WHERE NOT EXISTS (SELECT 1 FROM t WHERE t.x = e.col)` — correlated NOT EXISTS
  - `SELECT (SELECT t.col FROM t WHERE t.x = e.col) ...` — scalar subquery
    in SELECT list (returns `NULL` when inner query yields no rows)
- **14 new integration tests** in `tests/test_tier4_correlated_subquery.py`
  covering: basic correlated IN/NOT IN/EXISTS/NOT EXISTS, no-match /
  all-match variants, scalar NULL semantics, per-row re-execution
  verification, and correlated subqueries combined with outer WHERE filters.

## [1.15.0] - 2026-05-04

### Added

- **`IN (subquery)` / `NOT IN (subquery)` execution** — the adapter
  now converts the subquery form of `IN` / `NOT IN` (previously
  `ProgrammingError("subquery in IN clause is not yet supported")`)
  to `InSubquery` / `NotInSubquery` plan-expression nodes, which flow
  through the planner, codegen, and VM.  Full SQL three-valued NULL
  logic is preserved end-to-end.
- **13 new integration tests** in `tests/test_tier3_in_subquery.py`
  covering: basic `IN` / `NOT IN`, no-match / all-match / partial-match
  sets, `NULL` test-value exclusion, `NULL` in subquery set making
  `NOT IN` return `UNKNOWN`, aggregate subqueries (`GROUP BY` / `HAVING`
  inside the inner query), combined `AND` predicates, and `HAVING`-
  level `IN` filtering.

## [1.14.0] - 2026-05-04

### Added

- **FULL [OUTER] JOIN end-to-end** — `FULL JOIN` and `FULL OUTER JOIN`
  now execute correctly through the full mini-sqlite pipeline.  All rows
  from both tables appear: matched rows carry values from both sides,
  unmatched left rows carry `NULL` for right columns, and unmatched right
  rows carry `NULL` for left columns.
- **7 new integration tests** in `test_outer_join.py`:
  `test_full_outer_join_basic`, `test_full_join_keyword_alone`,
  `test_full_outer_join_no_orphans`, `test_full_outer_join_left_empty`,
  `test_full_outer_join_right_empty`, `test_full_outer_join_where_null_right`,
  `test_full_outer_join_where_null_left`.

## [1.13.0] - 2026-05-04

### Added

- **RIGHT [OUTER] JOIN end-to-end** — `RIGHT JOIN` and `RIGHT OUTER JOIN`
  now execute correctly. Unmatched right rows appear with `NULL` for all
  left-side columns. Implemented by swapping `lft`/`rgt` in the codegen
  and reusing LEFT JOIN machinery.
- **4 new integration tests** in `test_outer_join.py`:
  `test_right_outer_join_basic`, `test_right_join_keyword_alone`,
  `test_right_outer_join_left_empty`, `test_right_outer_join_where_null_left`

## [1.12.0] - 2026-05-04

### Added

- **LEFT [OUTER] JOIN end-to-end** — `LEFT JOIN` and `LEFT OUTER JOIN`
  now execute correctly through the full mini-sqlite pipeline. Unmatched
  left rows appear with `NULL` for all right-side columns.
- **Three-way chained LEFT JOIN** — `A LEFT JOIN B LEFT JOIN C` works via
  `join_match_stack` nesting in the VM; each join level tracks its own
  match state independently.
- **GROUP BY + COUNT with LEFT JOIN** — `COUNT(right_col)` correctly
  counts zero for left rows with no right match, since `COUNT` ignores
  NULLs.
- **WHERE on join result** — predicates like `WHERE right_col IS NULL`
  (anti-join pattern) and `WHERE left_col = 'x'` apply correctly after
  LEFT JOIN.

### Fixed

- **`PredicatePushdown` outer-join safety** — the optimizer no longer
  pushes right-side WHERE predicates inside a `LEFT OUTER JOIN`. Doing
  so would filter the right scan *before* the join, destroying the
  null-padding that makes the outer join semantics correct. The fix adds
  a `JoinKind`-aware guard in `_distribute_conjuncts`:
  - `LEFT JOIN`: left-side predicates may be pushed; right-side predicates
    stay above the join.
  - `RIGHT JOIN`: right-side predicates may be pushed; left-side stay above.
  - `FULL JOIN`: no predicates pushed to either side.
  - `INNER`/`CROSS`: both sides safe to push (unchanged).

## [1.11.0] - 2026-04-29

### Added

**Numeric parameter binding (`:N` style)**

`Cursor.execute` and `Connection.execute` now accept the third PEP 249
positional paramstyle: numeric `:N` placeholders bound from a `Sequence`.
This completes the trio (`?`, `:N`, `:name`) supported by the stdlib
`sqlite3` module.

```python
conn.execute(
    "SELECT * FROM employees WHERE dept = :1 OR dept = :2",
    ("eng", "sales"),
)
```

- **`binding.substitute`** — recognises `:` followed by digits as a
  numeric placeholder.  `N` is 1-indexed: `:1` → `parameters[0]`,
  `:2` → `parameters[1]`, etc.
- **Mutual exclusion** — qmark, numeric, and named styles cannot be
  mixed in a single statement.  The error message now lists all three:
  `"cannot mix '?', ':N', and ':name' parameter styles in one statement"`.
- **Error cases** — `:0` raises `ProgrammingError("1-indexed")`;
  `:N` with `N > len(parameters)` raises `out of range`; `:N` with a
  mapping raises `numeric` (must be a sequence).
- **Repeated indices** — `:1` may appear multiple times, all binding to
  the same value.  Trailing unused values in the sequence are silently
  ignored (matching `sqlite3`).
- **`paramstyle`** docstring extended to mention all three runtime
  styles; the declared value remains `"qmark"`.

### Tests added

- `tests/test_binding.py::TestNumericParameters` — 13 unit tests:
  single/multi binding, repeated index, extra-value tolerance,
  out-of-range, zero-index, scanner safe inside literals/comments,
  multi-digit index, paramstyle exclusivity (mixing, mapping rejection),
  value type rendering.
- `tests/test_cursor.py` — 3 end-to-end tests via `Connection.execute`:
  numeric SELECT, numeric INSERT, repeated index.

## [1.10.0] - 2026-04-29

### Added

**`PRAGMA user_version` (read/write) and `PRAGMA schema_version` (read)**

Two new PRAGMAs matching real SQLite's behaviour for header-field access:

```sql
PRAGMA user_version;             -- read: returns one row (user_version,)
PRAGMA user_version = 7;         -- write: stores 7 in the header
PRAGMA schema_version;           -- read: returns one row (schema_version,)
```

- **`PRAGMA user_version`** — application-defined `u32` (0 ≤ v ≤ 2³² − 1).
  Read returns a one-row, one-column result `(user_version,)`.  Write
  validates the range and stages the change on the backend; persistent
  backends (`SqliteFileBackend`) flush via the next `commit`.  An
  out-of-range value raises `ProgrammingError`.
- **`PRAGMA schema_version`** — read-only.  Returns the schema cookie,
  which is bumped automatically on every DDL operation (`CREATE TABLE`,
  `DROP TABLE`, `CREATE INDEX`, `DROP INDEX`, …).  DML statements
  (INSERT/UPDATE/DELETE) do *not* bump it.
- **`_PRAGMA_RE` extension** — the engine's PRAGMA regex now also matches
  the assignment form `PRAGMA name = <int>` (signed integer), with the
  value captured into a new `set_value` named group.
- Backend support: relies on
  `Backend.get_user_version` / `set_user_version` /
  `get_schema_version` (added in `sql-backend` 0.11.0 and
  `storage-sqlite` 0.18.0).  Backends without these methods cause
  `PRAGMA user_version` writes to raise
  `ProgrammingError("backend does not support …")` rather than
  AttributeError.

### Tests added

- `tests/test_tier3_pragma.py::TestUserVersion` — 8 tests: default,
  description, set+read, overwrite, zero-is-valid, max u32, negative
  rejected, overflow rejected.
- `tests/test_tier3_pragma.py::TestSchemaVersion` — 6 tests: default,
  description, CREATE TABLE bumps, DROP TABLE bumps, CREATE INDEX
  bumps, DML does not bump.

## [1.9.0] - 2026-04-28

### Added

**Bytes (BLOB) parameter binding**

`bytes`, `bytearray`, and `memoryview` parameters can now be bound to `?`
placeholders.  They render as the SQLite blob-literal form `X'<hex>'`,
which round-trips through the SQL lexer (it already accepts `X'...'`
since the BLOB-type work in 1.7.0).

```python
conn.execute("INSERT INTO blobs (data) VALUES (?)", (b"\xde\xad\xbe\xef",))
```

- **`binding._to_sql_literal`** — the previous `NotSupportedError` for
  byte parameters is replaced with `f"X'{bytes(value).hex()}'"`.  The
  explicit `bytes(value)` coercion materialises a fresh object so a
  hostile `bytes` subclass overriding `.hex()` cannot inject SQL.
- **`bytearray` / `memoryview`** are coerced via `bytes(...)` and render
  identically to `bytes`.
- **Empty bytes** render as `X''` (parses as a zero-length blob).

### Tests added

- `tests/test_binding.py` — 5 new tests: bytes round-trip, empty bytes,
  bytearray, memoryview, and a hostile-subclass injection-defense test.
- `tests/test_cursor.py::test_bytes_param_round_trip` — end-to-end
  insert + select of binary data through `Connection.execute`.

### Removed

- `test_bytes_not_supported` — replaced by the round-trip tests above.

## [1.8.0] - 2026-04-28

### Added

**Named parameter binding (`:name` style)**

`Cursor.execute` and `Connection.execute` now accept a `Mapping` (e.g. `dict`)
as the *parameters* argument, in addition to the existing `Sequence` form.
When a mapping is passed, every `:identifier` placeholder in the SQL is
replaced by `parameters[identifier]` — matching the stdlib `sqlite3`
behaviour and PEP 249's `"named"` paramstyle.

```python
conn.execute(
    "SELECT name FROM employees WHERE dept = :d AND active = :active",
    {"d": "eng", "active": True},
)
```

- **`binding.substitute(sql, parameters)`** — parameter type now
  `Sequence | Mapping`.  Sequence → qmark style (`?`); mapping → named
  style (`:name`).  Mixing the two styles in one statement raises
  `ProgrammingError`.
- **Identifier rules** — `:identifier` matches `[A-Za-z_][A-Za-z0-9_]*`.
  Postgres-style casts like `a::INT` are NOT recognised as placeholders
  (the `:` is followed by another `:`, not an identifier-start
  character).  Numeric placeholders like `:1` are also NOT recognised
  (PEP 249 calls those `"numeric"` style; not yet supported).
- **NULL-safe placeholders inside literals/comments** — `:foo` inside
  `'...'`, `--...`, or `/* ... */` is left untouched, matching the
  existing `?` scanner behaviour.
- **Extra dict keys are ignored** — only keys referenced by the SQL are
  consumed; unused keys do not raise (matches `sqlite3`).
- **`Connection.execute` / `Cursor.execute`** — type signature widened
  to `Sequence[Any] | Mapping[str, Any] = ()`.
- **`engine.run`** — same signature widening; forwards the mapping
  through to `substitute`.
- **`paramstyle`** docstring clarified — the module still declares
  `"qmark"` (matching stdlib `sqlite3`) but accepts both styles at
  runtime.

### Tests added

- `tests/test_binding.py::TestNamedParameters` — 17 unit tests
  covering single/multi-named binding, repeated keys, extra-key
  tolerance, missing-key error, scanner robustness inside literals
  and comments, double-colon non-recognition, identifier rules,
  paramstyle exclusivity (mixing, wrong container types), and value
  type rendering.
- `tests/test_cursor.py` — 4 end-to-end tests via `Connection.execute`:
  named SELECT, named INSERT, missing-key error, repeated key.

## [1.7.0] - 2026-04-28

### Added — SQL Extras: Scalar Subqueries, BLOB, PRAGMA, UDFs

- **Scalar subqueries** — `(SELECT expr FROM ...)` expressions now work in
  SELECT list, WHERE, and other expression positions. Returns NULL when
  the subquery finds no rows; raises `CardinalityError` when it returns
  more than one row.
- **BLOB type** — binary data via `x'DEADBEEF'` / `X'...'` hex literal
  syntax. `SqlValue` extended to include `bytes`; `sql_type_name()` returns
  `"BLOB"` for byte values.
- **PRAGMA statements** — engine-level interception for:
  - `PRAGMA table_info(t)` — column metadata (cid, name, type, notnull,
    dflt_value, pk)
  - `PRAGMA index_list(t)` — index names and uniqueness flags
  - `PRAGMA foreign_key_list(t)` — FK constraints from the live fk_child
    registry
  - `PRAGMA table_list` — all table names in the schema
- **User-defined functions (UDFs)** — `conn.create_function(name, nargs, fn)`
  registers a Python callable; nargs=-1 for variadic. UDFs take precedence
  over built-ins.

### Fixed

- **`primary_key` now flows through to backend** — `CREATE TABLE ... PRIMARY
  KEY` column constraint was lost in the IR → VM → backend pipeline.
  `IrColumnDef` now carries `primary_key: bool`; `_do_create_table` passes it
  to `BackendColumnDef`, so `PRAGMA table_info` correctly reports pk=1.

## [1.6.0] - 2026-04-28

### Added — Phase 9: SQL Triggers (BEFORE/AFTER INSERT/UPDATE/DELETE)

- **`_create_trigger()` / `_drop_trigger()` adapter functions** — translate
  `create_trigger_stmt` / `drop_trigger_stmt` AST nodes into
  `CreateTriggerStmt` / `DropTriggerStmt` planner statements.
- **`_node_to_sql()` helper** — reconstructs body SQL from the trigger body
  AST.  Re-adds single quotes around `STRING` token values (which the lexer
  strips), normalises `new`/`old` NAME tokens to uppercase, and escapes
  embedded single quotes using SQL-standard doubling.
- **`_inject_pseudo_refs()` / `_make_trigger_executor()`** — parameter-
  substitution approach for `NEW.col` / `OLD.col` references: replaces them
  with `?` placeholders bound to the actual pre/post-update row values before
  executing the body SQL.  This avoids the cursor-lookup problem that would
  arise from creating real pseudo-tables.
- **`_split_body_sql()`** — splits trigger body SQL on the `" ; "` separator
  emitted by `_node_to_sql` for multi-statement trigger bodies.
- **`run()` new parameters** — `trigger_executor` and `trigger_depth` are
  forwarded to `sql_vm.execute()`; the executor is auto-created on top-level
  calls and re-used for nested trigger body executions.
- **`test_tier3_triggers.py`** — 44 new tests covering:
  - Grammar: parser produces `create_trigger_stmt` / `drop_trigger_stmt` nodes
    (9 tests)
  - Adapter: correct `CreateTriggerStmt` / `DropTriggerStmt` output (8 tests)
  - Backend: `InMemoryBackend` trigger storage and retrieval (8 tests)
  - Integration: end-to-end trigger correctness via `:memory:` connection
    (19 tests) including BEFORE/AFTER INSERT/UPDATE/DELETE, NEW/OLD value
    access, multi-statement bodies, trigger ordering, DROP TRIGGER, and
    transaction rollback of trigger effects.

### Fixed

- **`sql-vm`: `_do_update` old-row snapshot** — `current_row` was captured as
  a mutable reference, causing AFTER UPDATE triggers to receive the
  post-update dict in `old_row`.  Fixed by copying the dict before mutation.

## [1.5.0] - 2026-04-27

### Added — Phase 8: Window Functions (OVER / PARTITION BY)

- **`_window_func_call()` adapter function** — translates a `window_func_call`
  parse-tree node into a `WindowFuncExpr`.  Handles `COUNT(*)` (becomes
  `func="count_star"` with `arg=None`), standard `func(expr)` calls, and
  arg-free functions like `ROW_NUMBER()`.  Parses `PARTITION BY` and window
  `ORDER BY` (DESC keyword detected via token inspection).
- **`_primary()` extension** — the `window_func_call` branch is tested before
  `function_call` (matching the grammar's PEG priority rule).
- **`test_tier3_window.py`** — 41 new tests covering:
  - Grammar: parser produces `window_func_call` nodes (7 tests)
  - Adapter: `_window_func_call()` produces correct `WindowFuncExpr` (13 tests)
  - Planner: `WindowAgg` plan node structure (5 tests)
  - Integration: end-to-end SQL via `:memory:` connection (16 tests)
- **`pyproject.toml` coverage `omit`** — excludes legacy `* 2.py` duplicate
  files from coverage measurement so the 80% threshold reflects real code.

### Functions supported end-to-end

`ROW_NUMBER()`, `RANK()`, `DENSE_RANK()`, `SUM(col)`, `COUNT(*)`,
`COUNT(col)`, `AVG(col)`, `MIN(col)`, `MAX(col)`, `FIRST_VALUE(col)`,
`LAST_VALUE(col)` — all with optional `PARTITION BY` and/or `ORDER BY`
inside the `OVER (…)` clause.

## [1.4.0] - 2026-04-27

### Added — Phase 7: SAVEPOINT / RELEASE / ROLLBACK TO

- **`SAVEPOINT name`** — creates a named savepoint within the active
  transaction (implicitly begins a transaction if none is open, matching
  SQLite semantics).
- **`RELEASE [SAVEPOINT] name`** — destroys the named savepoint and all
  savepoints created after it; changes since the savepoint are kept in the
  outer transaction.
- **`ROLLBACK TO [SAVEPOINT] name`** — rolls back all changes made after
  the named savepoint.  The savepoint itself survives and can be rolled
  back to again.
- **cursor `_tcl_keyword()` fix** — `ROLLBACK TO …` is no longer
  intercepted by the TCL fast-path; it passes through to the full engine
  pipeline so the grammar can extract the savepoint name.
- **`Connection._savepoints`** — live `list[str]` tracking active
  savepoints; cleared automatically on `COMMIT` or `ROLLBACK`.
- **27 new tests** in `tests/test_tier3_savepoint.py` covering grammar,
  adapter, end-to-end integration, and error handling.

## [1.3.0] - 2026-04-27

### Added — Phase 6: CREATE / DROP VIEW

- **`CREATE VIEW [IF NOT EXISTS] name AS query`** — the engine intercepts
  `CreateViewStmt` before calling `plan()` and stores the view's defining
  `SelectStmt` in the connection's `_view_defs` dict.  `IF NOT EXISTS` silently
  skips the operation when the view already exists; without the flag an existing
  view name raises `ProgrammingError`.
- **`DROP VIEW [IF EXISTS] name`** — removes the named view from `_view_defs`.
  `IF EXISTS` is a no-op when the view is absent; without the flag a missing
  name raises `ProgrammingError("no such view: …")`.
- **View expansion in the adapter** — `to_statement()` now accepts a
  `view_defs: dict[str, SelectStmt] | None` parameter that is threaded through
  `_query_stmt` → `_select` → `_table_ref` / `_join_clause`.  A plain table
  reference whose name matches an entry in `view_defs` is expanded inline to a
  `DerivedTableRef`, exactly like a non-recursive CTE.  CTEs take priority over
  views with the same name.
- **`adapter._create_view` / `_drop_view`** helper functions parse the two new
  statement forms and produce the matching planner AST nodes.
- **23 new tests** in `tests/test_tier3_views.py` covering grammar parsing,
  adapter AST construction, view expansion, and end-to-end SQL execution.

## [1.2.0] - 2026-04-27

### Added — Phase 5b: Recursive CTEs

- **End-to-end `WITH RECURSIVE` support** — `adapter._query_stmt()` detects a
  `RECURSIVE` keyword in the `with_clause` node and, when the CTE body contains
  a `set_op_clause` (UNION / UNION ALL), parses it as a `RecursiveCTERef`
  instead of a plain `SelectStmt`.  The adapter parses the anchor sub-select
  first (with the CTE name in scope for other CTEs but not for self), then
  parses the recursive body with the CTE name excluded from `active_ctes` so
  that the self-reference resolves to a plain `TableRef` for the planner.
- **`adapter._table_ref` handles `RecursiveCTERef` entries** — when a table
  name matches a `RecursiveCTERef` key in `active_ctes`, the ref is returned
  directly (with alias applied) rather than being wrapped in a `DerivedTableRef`.
  The planner's `RecursiveCTERef` path then produces a `RecursiveCTE` plan node.
- **`adapter._select` / `_join_clause`** — `ctes` parameter type extended to
  `dict[str, SelectStmt | RecursiveCTERef] | None` so recursive CTE refs flow
  through JOIN right-hand-side table references as well.
- **22 new tests** in `tests/test_tier3_recursive_cte.py`:
  - `TestRecursiveCTEGrammar` (6 tests) — grammar and adapter: `RecursiveCTERef`
    production, anchor/recursive field contents, `union_all` flag, alias
    propagation, self-reference left as `TableRef`.
  - `TestRecursiveCTEIntegration` (11 tests) — end-to-end: simple tree traversal,
    subtree starting at a node, org-chart depth computation, UNION vs UNION ALL,
    empty anchor, leaf-only query, multiple roots, ORDER BY and LIMIT on
    recursive results, COUNT aggregate over CTE.
  - `TestRecursiveCTEErrors` (5 tests) — error handling: unknown table in
    anchor, unknown column in anchor, type mismatch in WHERE, non-existent
    recursive column, LIMIT before recursion completes.

## [1.1.0] - 2026-04-27

### Added — Phase 5a: Non-recursive CTEs

- **`adapter._query_stmt()`** extended to detect an optional `with_clause`
  child node in the parse tree.  Each `cte_def` is parsed into a `SelectStmt`
  and recorded in an `active_ctes` dict that accumulates left-to-right so
  later CTEs can reference earlier ones.
- **`adapter._table_ref(ctes=)`** — when a plain table name matches a key in
  `active_ctes`, it is rewritten to a `DerivedTableRef` (alias defaults to the
  CTE name if no explicit `AS` is given).  This means CTEs are resolved
  entirely at the adapter layer; the planner, codegen, and VM see ordinary
  derived-table (subquery) nodes and require no changes.
- **`adapter._select(ctes=)` / `_join_clause(ctes=)`** — `ctes` parameter
  threaded through so JOIN right-hand-side table refs are also resolved.
- **`test_tier3_cte.py`** — 18 new tests: 5 grammar / adapter unit tests,
  9 end-to-end integration tests, and 4 error / edge-case tests.

## [1.0.0] - 2026-04-27

### Added — Phase 4b: FOREIGN KEY constraints

- **`Connection._fk_child` / `_fk_parent: dict`** — two mutable dicts
  initialized in `__init__` and threaded through every `Cursor.execute()` →
  `engine.run()` → `vm.execute()` call so FK registrations from `CREATE TABLE`
  persist for subsequent DML.
- **`engine.run()` `fk_child` / `fk_parent` parameters** — forwarded to
  `vm.execute()`.
- **`adapter._col_def()` REFERENCES parsing** — recognises `REFERENCES table`
  and `REFERENCES table(col)` grammar variants; stores `(ref_table, ref_col)`
  tuple as `ColumnDef.foreign_key` (ref_col is `None` when not specified).
- **18 new tests** in `tests/test_tier3_foreign_keys.py`:
  - `TestForeignKeyPipeline` — grammar, adapter, codegen pipeline unit tests.
  - `TestForeignKeyIntegration` — valid inserts, NULL FK passthrough, multi-child,
    delete-after-child-removed, table-survival.
  - `TestForeignKeyErrors` — missing parent on INSERT/UPDATE, RESTRICT on DELETE,
    error message content, multi-FK column enforcement.

## [0.9.0] - 2026-04-27

### Added — Phase 4a: CHECK constraints

- **`Connection._check_registry: dict`** — mutable dict initialized to `{}` on
  connection creation and threaded through `Cursor → engine.run() → vm.execute()`.
  Mutations from `CREATE TABLE` persist in this dict across `execute()` calls.
- **`engine.run()` `check_registry` parameter** — forwarded to `vm.execute()` so
  the same dict is used for both registration (CREATE TABLE) and enforcement
  (INSERT/UPDATE).
- **`adapter._col_def()` CHECK parsing** — recognises the `CHECK ( expr )` grammar
  variant and passes the parsed expression as `check_expr` on the `ColumnDef`.
- **20 new tests** in `tests/test_tier3_check_constraints.py`:
  - `TestCheckConstraintPipeline` — unit tests for grammar, adapter, planner, codegen.
  - `TestCheckConstraintIntegration` — valid inserts, boundary values, NULL semantics,
    UPDATE enforcement, multi-column checks, compound `AND` range check.
  - `TestCheckConstraintErrors` — violation on INSERT and UPDATE, error message
    mentions the column name, compound lower/upper bound violations.

## [0.8.0] - 2026-04-27

### Added — Phase 3: ALTER TABLE ADD COLUMN

- **`ALTER TABLE t ADD [COLUMN] col_def`** — full pipeline support across all layers:
  grammar, lexer keywords, adapter, planner, codegen IR, VM execution, and the
  InMemoryBackend.  Existing rows are backfilled with NULL (or the column default
  if one is provided).

- **Grammar** (`code/grammars/sql.grammar`, `sql-lexer _grammar.py`,
  `sql-parser _grammar.py`) — added `alter_table_stmt` rule; `ALTER`, `ADD`, and
  `COLUMN` registered as SQL keywords so they tokenize as KEYWORD not NAME.

- **`sql-backend`** — added abstract `add_column(table, column)` method to
  `Backend`; `InMemoryBackend` appends the column and backfills all existing rows
  with NULL; `ColumnAlreadyExists` error class added.

- **`storage-sqlite`** — `SqliteFileBackend.add_column` raises
  `Unsupported("ALTER TABLE ADD COLUMN")` (file-format rewrite not yet
  implemented).

- **`sql-planner`** — `AlterTableStmt` AST node; `AlterTable` plan node; planner
  dispatch `_plan_alter_table`.

- **`sql-codegen`** — `AlterTable` IR instruction; compiler case
  `PlanAlterTable → AlterTable` using `_to_ir_col` for type conversion.

- **`sql-vm`** — `_do_alter_table` handler; `ColumnAlreadyExists` VM error;
  `_translate_backend_error` extended to map `be.ColumnAlreadyExists`.

- **`mini_sqlite.adapter`** — `_alter_table` parser; `alter_table_stmt` dispatch.

- **`mini_sqlite.errors.translate`** — maps `ColumnAlreadyExists` to
  `OperationalError`.

- **`test_tier3_alter_table.py`** — 16 new tests across three classes:
  - `TestAlterTablePipeline` (5 tests): grammar, adapter, planner, codegen.
  - `TestAlterTableIntegration` (9 tests): nullable add, NOT NULL, INSERT after
    ALTER, UPDATE on new column, WHERE filter, multiple columns, commit.
  - `TestAlterTableErrors` (2 tests): table-not-found, duplicate-column.

## [0.7.0] - 2026-04-27

### Added — Phase 2: EXISTS / NOT EXISTS subquery expressions

- **`EXISTS (subquery)` and `NOT EXISTS (subquery)`** — fully supported in
  `WHERE`, `HAVING`, and `SELECT` list positions.  Only uncorrelated subqueries
  are supported in this version (the subquery may not reference columns from
  the outer query).

- **Grammar** (`code/grammars/sql.grammar`) — `EXISTS "(" query_stmt ")"` added
  as an alternative in the `primary` rule, before the existing subquery-in-parens
  alternative.  `NOT EXISTS` works automatically via the existing `not_expr`
  grammar rule.

- **Adapter** (`mini_sqlite.adapter._primary`) — recognises the `EXISTS`
  keyword token and constructs an `ExistsSubquery(query=SelectStmt)` from the
  child `query_stmt` node.

- **`_flatten_project_over_aggregate`** (engine) — extended to handle
  `Project(Having(Aggregate(...)))` in addition to the pre-existing
  `Project(Aggregate(...))` case.  Without this fix, HAVING clauses with
  non-standard predicates (including EXISTS) caused an "unsupported plan node:
  Having" error during codegen.

- **`test_tier3_exists.py`** — 26 new tests across three classes:
  - `TestExistsBasic` (6 tests): grammar parsing, TRUE/FALSE result verification.
  - `TestExistsIntegration` (13 tests): WHERE, HAVING, SELECT-list, AND/OR
    combinations, filtered subqueries, LIMIT 0 subquery, empty-table cases.
  - `TestNotExistsIntegration` (7 tests): same coverage for `NOT EXISTS`.

## [0.6.1] - 2026-04-27

### Added — ML observer hook: IndexPolicy.on_query_event forwarding

- **`IndexPolicy.on_query_event(event: QueryEvent) -> None`** (optional hook) —
  documented as a third, fully optional method on the `IndexPolicy` protocol.
  When implemented by a custom policy, the advisor forwards every
  `QueryEvent` to it immediately after the drop loop completes.  This gives
  ML-based or adaptive policies access to raw runtime signals — table scanned,
  filtered columns, `rows_scanned`, `rows_returned`, `used_index`, and
  `duration_us` — so they can maintain their own feature history without
  needing to intercept the advisor's internal state.

  Detection follows the same `hasattr` / `callable` pattern already used for
  `should_drop`: a policy that does not implement `on_query_event` is simply
  never called, preserving full backward compatibility with v2-style policies.

- **`IndexAdvisor.on_query_event` restructured** — the early `return` for
  policies without `should_drop` has been replaced by a guarded `if
  callable(should_drop_fn):` block so execution always reaches the
  `on_query_event` forwarding at the end of the method, regardless of whether
  the drop loop ran.

- **`tests/test_tier3_ml_hook.py`** — 14 new tests covering:
  - Protocol surface: `HitCountPolicy` has no `on_query_event`; v2 policies
    remain backward compatible.
  - Forwarding behaviour: single and multiple events forwarded in order; the
    exact same `QueryEvent` object is passed; hook fires even when
    `should_drop` is absent; hook fires after the drop loop.
  - ML policy integration via `Connection`: policy accumulates events from
    real queries, sees `used_index` after index creation, coexists with
    `should_drop`, survives `set_policy` swaps, and exposes selectivity
    signals.

## [0.6.0] - 2026-04-23

### Added — Phase 9.7: Composite (multi-column) automatic index support (IX-8)

- **`IndexAdvisor._pair_hits: dict[tuple[str, str, str], int]`** — new
  accumulator tracking `(table, col_a, col_b)` predicate pairs observed in
  full-table scans.  Pair keys are always normalised to ascending column-name
  order to avoid double-counting `(a, b)` and `(b, a)`.

- **`IndexAdvisor._auto_index_meta: dict[str, tuple[str, tuple[str, ...]]]`** —
  maps auto-created index name → `(table, columns_tuple)`.  Replaces name
  parsing for drop-loop bookkeeping; correctly handles composite names like
  `auto_orders_user_id_status` that would confuse a `split("_", 2)` approach.

- **`IndexAdvisor._record_pair(table, col_a, col_b)` callback** — increments
  `_pair_hits` for the normalised pair key, then calls
  `_maybe_create_composite_index` when the policy threshold is reached.  Pair
  callbacks are processed **before** single-column callbacks inside `_walk` so
  that if both thresholds fire in the same observation, the composite is created
  first and the subsequent single-column check correctly skips creating a
  redundant index on the leading column.

- **`IndexAdvisor._maybe_create_composite_index(table, col_a, col_b)`** —
  creates a two-column B-tree index `auto_<table>_<col_a>_<col_b>` unless any
  existing index already has `col_a` as its leading column (which would make
  the composite redundant for leading-column-only queries).  Registers the new
  index in `_auto_index_meta`.

- **`IndexAdvisor.observe_plan` updated** — passes `pair_callback=self._record_pair`
  to `_walk`.

- **`_walk` pair callback support** — the helper now accepts an optional
  `pair_callback(table, col_a, col_b)` argument.  Inside the
  `Filter(Scan(...))` branch, all `(col_i, col_j)` pairs from the predicate
  column list are dispatched to `pair_callback` before the per-column
  `callback` calls, ensuring composite creation precedes single-column creation.
  The `IndexScan` branch now destructures `columns=idx_cols` (was `column=col`)
  and iterates the tuple.

- **`engine._extract_scan_info` updated** — the `IndexScan` match arm now
  reads `columns=cols` (was `column=col`) and returns `list(cols)`.

### Tests

- `tests/test_tier3_composite.py` — 21 new tests across three classes:
  - `TestAdvisorComposite` (8 tests) — pair hit accumulation, composite index
    creation at threshold, naming convention, skipping composite when
    single-column index on leading column already exists, no duplicate creation,
    independent columns not cross-correlated, `_auto_index_meta` population,
    pair hits reset after composite drop.
  - `TestPlannerComposite` (8 tests) — planner uses composite index for both
    columns, leading-column prefix match, non-leading column cannot use
    composite, composite preferred over single-column for two-column query,
    range on second column, lower-bound range, equality on both columns,
    BETWEEN on second column.
  - `TestCompositeIntegration` (5 tests) — full end-to-end create cycle,
    range correctness, equality correctness, `auto_index=False` has no
    composite, composite drop resets pair hits.

## [0.5.0] - 2026-04-23

### Added — Phase 9.6: Automatic index drop logic (IX-7)

- **`IndexPolicy.should_drop` optional method** — the protocol now documents
  an optional `should_drop(index_name, table, column, queries_since_last_use)`
  method.  Policies without it continue to work (the advisor detects the method
  via `hasattr`).

- **`HitCountPolicy.cold_window` parameter** — new keyword-only argument
  (default 0, which disables drop logic).  When positive, `should_drop`
  returns `True` once an auto-created index hasn't been seen in
  `queries_since_last_use >= cold_window` consecutive SELECT scans.
  Negative values raise `ValueError`.

- **`HitCountPolicy.should_drop` method** — implements the optional drop
  decision.  Always returns `False` when `cold_window == 0`; otherwise
  returns `queries_since_last_use >= cold_window`.  Accepts `index_name`,
  `table`, and `column` (unused in this implementation — custom policies
  may inspect them).

- **`IndexAdvisor.on_query_event(event: QueryEvent)` hook** — second hook on
  the advisor (alongside the existing `observe_plan`).  Called by the engine
  after each SELECT scan:
  - Increments `_query_count` (the global SELECT scan counter).
  - Records `_last_use[index_name] = _query_count` when `event.used_index`
    is a known auto-index.
  - Iterates all tracked auto-indexes and calls `policy.should_drop` on each;
    drops cold indexes via `backend.drop_index(name, if_exists=True)`.
  - Clears drop-tracking state and hit counts for dropped indexes so they
    can be re-created if the query pattern returns.
  - Drop failures are swallowed — the advisor continues running.

- **`IndexAdvisor` drop-tracking state** — three new internal fields:
  `_query_count: int`, `_last_use: dict[str, int]`,
  `_created_at: dict[str, int]`.

- **`engine.run()` wires `event_cb`** — passes `advisor.on_query_event` as
  `event_cb` to `vm.execute()` and pre-populates `filtered_columns` via
  `_extract_scan_info(optimized)`.  The callback is only set for SELECT-type
  plans; DML and DDL never advance the cold-window counter.

- **`_extract_scan_info(plan)` helper** in `engine.py` — walks the logical
  plan to extract the primary scan table and filtered column names for
  pre-populating `QueryEvent`.  Uses structural pattern matching; returns
  `("", [])` for DDL/DML.

- **`QueryEvent` re-exported** from `mini_sqlite` top-level namespace and
  added to `__all__`.

### Tests

- `tests/test_tier3_drop.py` — 42 new tests across four classes:
  - `TestHitCountPolicyColdWindow` — 10 tests for the `cold_window` parameter
    and `should_drop` semantics.
  - `TestQueryEventEmission` — 8 tests for VM-level event emission (table,
    rows_scanned, rows_returned, filtered_columns, duration_us, index usage).
  - `TestAdvisorDropLogic` — 10 tests for advisor drop loop (query counting,
    last-use tracking, drop at threshold, reset on use, non-fatal failures,
    v2-policy compatibility, hit-count reset after drop).
  - `TestDropIntegration` — 6 end-to-end tests via `mini_sqlite.connect()`
    (full create-then-drop cycle, re-creation after drop, `cold_window=0`
    never drops, `auto_index=False` has no advisor, `QueryEvent` export).

## [0.4.0] - 2026-04-22

### Added — Phase 9.5: Automatic B-tree index creation (IndexAdvisor)

- **`CREATE INDEX` / `DROP INDEX` DDL** — end-to-end support for explicit
  index management:
  - Grammar extended with `create_index_stmt` and `drop_index_stmt` rules.
  - `sql-parser` regenerated from the updated grammar.
  - `sql-planner` gained `CreateIndexStmt`, `DropIndexStmt` AST nodes and
    `CreateIndex`, `DropIndex` plan nodes.  The planner dispatches to
    `_plan_create_index` / `_plan_drop_index` which emit the new plan nodes.
  - `sql-codegen` gained `CreateIndex` and `DropIndex` IR instructions plus
    compiler lowering.
  - `sql-vm` handles `CreateIndex` and `DropIndex` by calling
    `backend.create_index` and `backend.drop_index`.
  - `adapter.py` gains `_create_index()` and `_drop_index()` helper
    functions and their dispatch cases in `_stmt_dispatch`.
  - `CREATE UNIQUE INDEX` and `CREATE INDEX IF NOT EXISTS` are both
    supported.  `DROP INDEX IF EXISTS` is supported.

- **`IndexScan` planner node** — the planner can now substitute a
  `Filter(Scan(t))` with an `IndexScan(t)` when an index covering the
  predicate column exists on the backend.  Range bounds are extracted from
  EQ / GT / GTE / LT / LTE / BETWEEN predicates.  All five optimizer passes
  (`constant_folding`, `dead_code`, `limit_pushdown`, `predicate_pushdown`,
  `projection_pruning`) handle `IndexScan` as a leaf node.

- **`IndexAdvisor`** (`mini_sqlite.advisor`) — observes every optimised
  query plan and auto-creates B-tree indexes for filtered-but-unindexed
  columns:
  - Hooks into `engine.run()` via the new `advisor` keyword parameter.
    Called with the optimised plan before code generation.
  - Walks the plan tree looking for `Filter(Scan(t), predicate)` patterns
    and records `(table, column)` hit counts.
  - Uses `auto_{table}_{column}` naming convention for created indexes.
  - Skips creation if any existing index already covers the column (first
    key match).
  - Handles `IndexAlreadyExists` from the backend gracefully (race-safe
    no-op).

- **`IndexPolicy` / `HitCountPolicy`** (`mini_sqlite.policy`) — pluggable
  decision interface for auto-index creation:
  - `IndexPolicy` — `@runtime_checkable` `Protocol` requiring `should_create(table, column, hit_count) → bool`.
  - `HitCountPolicy(threshold=3)` — creates an index when a column's
    filter-hit count reaches the configured threshold.  Default threshold 3.
    Threshold must be ≥ 1 (raises `ValueError` otherwise).
  - Any object implementing `should_create` satisfies the protocol without
    subclassing.

- **`Connection.set_policy(policy)`** — replace the active
  `IndexPolicy` on a live connection without losing accumulated hit counts.
  No-op when `auto_index=False`.

- **`connect(auto_index=True)`** — new `auto_index` keyword parameter.
  `True` (default): an `IndexAdvisor` is attached to the connection.
  `False`: no advisor; automatic index management is disabled entirely.

- **`mini_sqlite.__all__`** additions: `HitCountPolicy`, `IndexAdvisor`,
  `IndexPolicy`.

### Tests

- `tests/test_tier2_features.py` — 43 additional tests covering:
  - `TestCreateDropIndex` (8 tests): CREATE INDEX, CREATE UNIQUE INDEX,
    CREATE INDEX IF NOT EXISTS idempotence, DROP INDEX, DROP INDEX IF EXISTS,
    multi-column indexes, correctness parity (indexed vs. un-indexed).
  - `TestHitCountPolicy` (10 tests): threshold semantics, protocol
    conformance, error cases, custom policy protocol.
  - `TestIndexAdvisor` (9 tests): advisor creation, set_policy, auto-index
    naming, threshold behavior (below/at/above), no-duplicate creation,
    explicit index prevents auto creation, correctness before/after.
  - `TestConnectAutoIndex` (5 tests): `auto_index` parameter, `__all__`
    exports.

## [0.3.0] - 2026-04-21

### Added — Phase 9: Tier-2 SQL features (CASE, derived tables, chained set ops, TCL)

- **CASE expression** (`CASE WHEN … THEN … [ELSE …] END`) — both searched and
  simple CASE forms now parse and execute end-to-end.  The adapter converts
  simple CASE into equality comparisons; the codegen emits a
  `JumpIfFalse`-based chain; the VM evaluates branches lazily.  CASE can appear
  in SELECT items, WHERE predicates, ORDER BY keys, and HAVING clauses.

- **Derived tables** (`(SELECT …) AS alias` in FROM) — subqueries used as
  table sources now work end-to-end.  The adapter translates to
  `DerivedTableRef`; the planner emits a `DerivedTable` plan node with resolved
  output columns; the codegen emits `RunSubquery`; the VM executes the inner
  program against the same backend and exposes the rows via `_SubqueryCursor`.

- **Chained set operations** — `A UNION B UNION C`, `A INTERSECT B EXCEPT C`,
  etc.  The adapter builds a left-associative tree of
  `UnionStmt`/`IntersectStmt`/`ExceptStmt` nodes; the planner dispatches
  through `plan()` for each left operand so nesting resolves correctly.

- **Explicit TCL interception** — `BEGIN`, `COMMIT`, and `ROLLBACK` SQL
  statements are now intercepted in `Cursor.execute()` *before*
  `_ensure_transaction_if_needed` runs, delegating to three new
  `Connection`-level methods:
  - `_tcl_begin()` — opens a transaction; raises `OperationalError` if one is
    already active.
  - `_tcl_commit()` — commits the active transaction; raises `OperationalError`
    if none exists.
  - `_tcl_rollback()` — rolls back the active transaction; raises
    `OperationalError` if none exists.
  This prevents a double-transaction collision (the connection's implicit
  transaction opening racing with the VM's `BeginTransaction` instruction).

- **`_flatten_children()` recursion in `engine.py`** — the
  `_flatten_project_over_aggregate` helper now recurses into child plans
  (including `DerivedTable`, `Filter`, `Join`, `Union`, etc.) before processing
  the outer plan, so `Project(Aggregate(...))` patterns inside derived tables
  are correctly rewritten before codegen sees them.

### Fixed

- **INSERT with explicit column list** — `_insert()` in the adapter now
  correctly parses the column name list when an `insert_body` grammar node
  separates the column list from the values.

- **`_stmt_dispatch` routing** — statements that arrive as `query_stmt` nodes
  (the grammar's outer wrapper for SELECT + set-op tails) are now handled
  explicitly; previously only bare `select_stmt` nodes were routed, causing
  parse errors for UNION queries at the top level.

### Tests

- `tests/test_tier2_features.py` — 34 new integration tests across six classes:
  `TestCaseExpression` (11), `TestDerivedTables` (5), `TestChainedSetOps` (5),
  `TestExplicitTransactions` (4), `TestSubqueriesInWhere` (5),
  `TestCrossJoin` (4).
- Mini-sqlite total: **165 tests, 89.79% coverage**.

## [0.2.0] - 2026-04-20

### Added — Phase 8: file-backed `connect()` and byte-compatibility oracle tests

- **`mini_sqlite.connect("path.db")`** now works end-to-end against a real
  SQLite `.db` file.  Previously any non-`:memory:` path raised
  `InterfaceError`; now `connect()` routes to `SqliteFileBackend(path)` from
  the `storage_sqlite` package.  The resulting `Connection` has identical PEP
  249 semantics to the in-memory connection: `commit()`, `rollback()`,
  `execute()`, `executemany()`, context-manager auto-commit / auto-rollback,
  and `cursor()` all work.

  ```python
  with mini_sqlite.connect("app.db") as conn:
      conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
      conn.execute("INSERT INTO users VALUES (1, 'Alice')")
  # File is byte-compatible with sqlite3's own .db format.
  ```

- **DDL auto-commit semantics**: `Connection._ensure_transaction_if_needed`
  now begins a fresh single-statement transaction for every DDL statement
  (`CREATE TABLE`, `DROP TABLE`, `ALTER TABLE`).  `Cursor.execute` calls the
  new `Connection._post_execute()` hook after each statement; for DDL that
  hook immediately commits the single-statement transaction so schema changes
  are persisted to disk even if no DML follows.  Any previously open DML
  transaction is committed first, matching the behaviour of the stdlib
  `sqlite3` module.

- **`Connection._post_execute()`** — new internal method that auto-commits
  DDL transactions.  Non-DDL statements are a no-op.

- **`Connection._ddl_txn: bool`** — new internal flag that distinguishes a
  DDL single-statement transaction (auto-commit on `_post_execute`) from a
  normal DML transaction (user-controlled commit/rollback).

- **`tests/test_file_backend.py`** — 21 new tests in two families:

  *File-backend functional tests* (12 tests) — exercise all SQL operations
  against a real `.db` file: create/reopen database, full DDL+DML round-trip,
  SELECT with WHERE, UPDATE, DELETE, DROP TABLE, explicit commit/rollback,
  context-manager commit/rollback, NULL values, 500-row large table (exercises
  B-tree splits), multiple independent tables.

  *Byte-compatibility oracle tests* (9 tests) — use Python's stdlib `sqlite3`
  module as the reference implementation:
  - `test_oracle_mini_sqlite_writes_sqlite3_reads`: write via mini_sqlite,
    read via stdlib sqlite3 — verifies on-disk format is byte-compatible.
  - `test_oracle_sqlite3_writes_mini_sqlite_reads`: write via stdlib sqlite3,
    read via mini_sqlite — verifies mini_sqlite can parse files it did not
    produce.
  - `test_oracle_null_roundtrip`: NULL values written by mini_sqlite read as
    `None` by sqlite3.
  - `test_oracle_sqlite3_null_read_by_mini_sqlite`: NULL values written by
    sqlite3 read as `None` by mini_sqlite.
  - `test_oracle_integer_types`: full integer range (0..2⁶³−1) round-trips
    through the record layer correctly.
  - `test_oracle_text_with_special_characters`: text with quotes, Unicode,
    newlines, emojis survives the round-trip.
  - `test_oracle_schema_visible_in_sqlite3`: `sqlite_schema` written by
    mini_sqlite is visible to `sqlite3`.
  - `test_oracle_append_then_read_all`: two separate mini_sqlite sessions
    both visible to stdlib sqlite3.

- `pyproject.toml` — added `"coding-adventures-storage-sqlite"` to
  `dependencies` list.

- `BUILD` — added `-e ../storage-sqlite` to the `uv pip install` command so
  the storage-sqlite package is installed in the test environment.

### Changed

- `tests/test_module.py`: `test_connect_rejects_unknown_database` (which
  expected `InterfaceError` for a file path) replaced by
  `test_connect_file_path_creates_file` which verifies that `connect(path)`
  creates a `.db` file on disk.

## [0.1.0] - 2026-04-19

### Added

- Initial release. PEP 249 DB-API 2.0 facade over the full SQL pipeline.
- `mini_sqlite.connect(":memory:")` returns an in-memory `Connection`.
- Module globals: `apilevel="2.0"`, `threadsafety=1`, `paramstyle="qmark"`.
- `Connection` with `cursor()`, `commit()`, `rollback()`, `close()`,
  `execute()`, `executemany()`, and context manager support.
- `Cursor` with `execute()`, `executemany()`, `fetchone()`,
  `fetchmany()`, `fetchall()`, `description`, `rowcount`, iteration
  protocol, and `close()`.
- ASTNode → planner Statement adapter covering SELECT (with WHERE,
  ORDER BY, LIMIT, OFFSET, DISTINCT, GROUP BY, HAVING, aggregates,
  INNER/CROSS joins), INSERT VALUES, UPDATE, DELETE, CREATE TABLE
  [IF NOT EXISTS], DROP TABLE [IF EXISTS].
- `?` parameter binding via source-level substitution (the vendored SQL
  lexer has no QMARK token, so we escape values into SQL literals
  before handing the statement to the pipeline). Arity validated, with
  backslash-escape string literals to match the lexer's rules.
- `Project(Aggregate(...))` flattening pass in the engine so the codegen
  (which expects Aggregate as the core operator) can compile aggregate
  queries wrapped by the planner in a Project for schema uniformity.
- `INSERT INTO t VALUES (...)` without a column list resolves against
  the backend's declared schema before planning.
- PEP 249 exception hierarchy with translation from every underlying
  pipeline exception family, including lexer and parser errors →
  `ProgrammingError`.
- Output value coercion: `True`/`False` → `1`/`0` to match sqlite3.

