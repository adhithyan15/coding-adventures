# Changelog

## 0.4.2 — FULL OUTER JOIN matches SQLite (ledger 6 → 5)

Stream A / L2 of the full-SQLite-replacement roadmap: retire the differential
oracle's `full_join` divergence — the last *wrong-result* entry in the ledger.

- `SELECT ... FROM a FULL JOIN b ON ...` previously degraded to a cross product
  (a single forward pass can't emit the right rows that matched no left row).
  `sql-codegen` (0.5.0) now compiles FULL JOIN as **two passes**: a LEFT JOIN
  (matched pairs + left-only rows) unioned with a RIGHT anti-join (right rows
  that matched no left row, NULL-padded on the left). No new VM instructions —
  it reuses the outer-join match flag. See that crate's changelog.
- `tests/differential_oracle.rs`: removed `full_join` from the `LEDGER` (now
  **5** entries, all aggregate computed-column *naming* divergences whose rows
  already match). Added a second FULL JOIN case, `full_join_multi`, with
  duplicate join keys on both sides (many-to-many) plus rows unmatched on each
  side — asserted against real SQLite to guard the two-pass implementation
  against double-emitting matched pairs or dropping an anti-join row.
- No mini-sqlite `src/` changes — the fix lands in the shared pipeline; this
  crate's bump documents the conformance gain the oracle now enforces.

## 0.4.1 — Scalar functions match SQLite (ledger 7 → 6)

Stream B / L3 of the full-SQLite-replacement roadmap: retire the differential
oracle's `string_functions` divergence, the last *wrong-value* entry in the
ledger.

- `SELECT UPPER(name), LENGTH(name)` previously came back as `LENGTH(name),
  LENGTH(name)` with both columns named `?`. Two independent bugs, one symptom:
  - **sql-vm** (0.2.1): Phase-4 materialization collapsed each row's positional
    `(name, value)` pairs through a `HashMap` keyed by column name, so two
    same-named output columns kept only the *last* value. Now projects by
    position (the row buffer is already parallel to the locked column list).
  - **sql-codegen** (0.4.0): un-aliased function columns were labelled `?`. Now
    labelled with the reconstructed call text (`UPPER(name)`), matching SQLite.
- `tests/differential_oracle.rs`: removed `string_functions` from the `LEDGER`
  (now **6** entries — `full_join` plus five aggregate computed-column *naming*
  divergences); the case is now asserted to match real SQLite exactly.
- No mini-sqlite `src/` changes — the fixes land in the shared pipeline crates;
  this crate's bump documents the conformance gain the oracle now enforces.

## 0.4.0 — Open a real `.sqlite` file

Stream C / L4 of the full-SQLite-replacement roadmap: `connect()` can now open a
**real SQLite database file**, and the entire query pipeline (parser → planner →
optimizer → codegen → VM) runs unmodified over it.

- `connect("<path>")` reads the file's bytes and drives the engine through the
  read-only `SqliteFileBackend` (`storage-sqlite`, built on the zero-dep
  `sqlite-file` reader — no third-party SQLite at runtime). `":memory:"` is
  unchanged. A missing file or non-SQLite bytes surface as `OperationalError`;
  the previous blanket `NotSupportedError` for any non-`:memory:` name is gone.
- `ConnectionState.backend` is now `Box<dyn Backend>` so either backend plugs in;
  the connection tracks its own transaction handle (`current_transaction` is not
  a `Backend`-trait method). File-backed connections are **read-only** for now —
  `INSERT`/`UPDATE`/`CREATE` against a file return an error rather than silently
  no-op; the byte-compatible writer is a later milestone.
- New `tests/file_backed.rs` (rusqlite dev-dep oracle): builds a genuine `.sqlite`
  file, opens it via `connect(path)`, and asserts `SELECT` (projection, `WHERE`,
  `ORDER BY`, aggregates, and the `INTEGER PRIMARY KEY` rowid alias) returns what
  the real library does — proving the whole engine runs over a real file.

This graduates the Rust port past the old Level-0 rule ("file-backed connections
raise `NotSupportedError`", conformance fixture 12); the fixture's
`connect_expect_error` still holds because a *missing* file still errors.

## 0.3.0 — Differential conformance oracle (baseline)

First step of the roadmap to make mini-sqlite a drop-in SQLite replacement
(`code/specs/mini-sqlite-full-conformance.md`, Stream A / L2).

- **`tests/differential_oracle.rs`** — a differential-conformance harness that
  runs the same SQL through mini-sqlite *and* real bundled SQLite (`rusqlite`, a
  **dev-dependency only** — never linked by the shipped crate) and asserts they
  agree: matching columns (case-insensitive), matching rows (order-sensitive only
  under `ORDER BY`), and error-vs-success agreement. This is the measuring
  instrument the whole conformance roadmap is gated on, mirroring the
  `sqlite-file` crate's cross-check against the real on-disk format.
- On introduction it measured **12 of 22 seed cases already matching SQLite** and
  reproduced **10 genuine gaps**, recorded in an explicit known-divergence
  `LEDGER` (shrinking the ledger is the conformance metric): qualified column
  refs across a join resolve to NULL (breaks even `INNER JOIN`); `LEFT`/`RIGHT`/
  `FULL JOIN` drop their `ON` clause; aggregate columns are misnamed (`agg_N` vs
  `SUM(n)`); and `UPPER()` returns the wrong value. Each is a tracked follow-up
  increment. No shipped code changed in this release — only the test harness.

## 0.2.1 — Security hardening (post-review fixes)

- `quote_sql_string`: strip NUL bytes from `Text` parameters before escaping.
  Some lexers treat an embedded `\0` as a string terminator, which could allow
  a malicious value to inject raw SQL after the NUL.
- `read_quoted`: fix `''` doubled-single-quote handling in the parameter
  scanner.  Previously the scanner exited on the first `'` of a `''` escape
  sequence, causing downstream `?` placeholders to be mis-positioned.
- `sql-vm — SaveGroupKey`: cap GROUP BY distinct keys at 1 000 000.  Queries
  over high-cardinality GROUP BY columns could exhaust memory; now returns
  `VmError::ResourceLimit` instead.
- `sql-vm — CountDistinct`: cap distinct-value set at 1 000 000 entries.
  `COUNT(DISTINCT large_blob_col)` over millions of rows could accumulate
  gigabytes of hex strings; now returns `VmError::ResourceLimit` instead.
- `sql-vm — VmError`: add `ResourceLimit(String)` variant used by the two new
  caps above.

## 0.2.0 — Level 1 graduation (full pipeline)

Route ALL SQL — DDL, DML, and SELECT — through the complete Mini-SQLite
pipeline instead of the hand-rolled Level 0 executor:

```
sql-lexer → sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm
```

All 45 conformance + unit tests pass. Key changes in the pipeline:

### mini-sqlite facade
- Replace `InMemoryDatabase` (Level 0 hand-rolled store) with
  `coding-adventures-sql-backend::InMemoryBackend` as the storage layer.
- Remove `coding-adventures-sql-execution-engine` dependency; SELECT now
  goes through the same pipeline as INSERT/UPDATE/DELETE/DDL.
- Add `SchemaProvider` adapter (`backend_as_schema_provider`) so the planner
  can resolve table schemas against the live backend.
- Transaction management: `begin_transaction` is called automatically on the
  first DML/DDL within a connection; `commit()` and `rollback()` delegate to
  the backend's transaction handles.
- Add `serde_json` dev-dependency and a JSON-driven conformance test runner
  that loads all 24 fixtures from `code/specs/mini-sqlite-conformance/fixtures/`.
- `sql_literal()` ensures float parameters include a decimal point.
- Parameter binding (`bind_parameters`): qmark-style `?` substitution.
- `MiniSqliteError` variants unchanged; `API_LEVEL`, `THREAD_SAFETY`,
  `PARAM_STYLE` constants unchanged.

### sql-lexer
- Add `CONCAT_OP` token (`||`) declared before any single-pipe token so the
  lexer always prefers the longer two-character match.

### sql-parser
- Add `||` to the `additive` rule so `||` is parsed as a binary operator.
- Make `FROM table_ref { join_clause }` optional in `select_stmt` so
  `SELECT expr` without a FROM clause is accepted.
- Add optional `-` before `NUMBER` in `limit_clause` for `LIMIT -1`.
- Allow optional `DISTINCT` before args in `function_call` for `COUNT(DISTINCT col)`.

### sql-planner
- Support `SELECT expr` without FROM via a `__dual__` virtual table (`SELECT 1 + 1`).
- `plan_limit()` handles `LIMIT -1` (returns `count = Some(-1)` = all rows).
- HAVING aggregate deduplication: `COUNT(*)` in HAVING reuses the SELECT slot.

### sql-optimizer
- `Project(EmptyResult)` no longer collapses to `EmptyResult` — the Project's
  column list encodes the output schema needed for `DefineColumns`.

### sql-codegen
- Add `AggFn::CountDistinct` variant for `COUNT(DISTINCT col)`.
- Add `Instruction::DefineColumns(Vec<String>)` — sets output column names
  without emitting rows (for `LIMIT 0` queries).
- Add `Instruction::CallBuiltin(String, usize)` — dispatches named scalar
  SQL built-in function calls (LENGTH, UPPER, LOWER, TRIM, SUBSTR, REPLACE,
  ABS, COALESCE, ROUND).
- Compile `Project(EmptyResult)` → `DefineColumns(col_names)` instead of nothing.
- Compile `FunctionCall` → `CallBuiltin` instead of `LoadConst(Null)`.
- Add `agg_slots` field to `Compiler` so `compile_expr` emits `FinalizeAgg`
  with the correct slot index in multi-aggregate HAVING predicates.

### sql-vm
- Add `__dual__` virtual table support in `OpenScan`: yields one empty row
  for `SELECT expr` without FROM.
- Fix `BinaryOp::Concat` to propagate NULL (was converting NULL to empty string).
- Add `DefineColumns` instruction handler: sets output column names without
  emitting rows.
- Add `CallBuiltin` instruction handler with full `call_builtin()` dispatcher.
- Implement GROUP BY multi-group aggregation: `SaveGroupKey` now tracks
  per-group accumulators; `CloseScan` triggers group iteration mode; the
  finalize/predicate/emit block is re-executed once per group.
- Implement `AggFn::CountDistinct` accumulator with lazy `HashSet<String>`.
- Fix `apply_limit` for `LIMIT -1`: negative count = all rows (SQLite semantics).
- Fix `UpdateRows` to carry column names for multi-column SET assignments.
- HAVING + GROUP BY: `JumpIfFalse` advances group iterator on predicate-false
  instead of jumping to the skip label prematurely.

## 0.1.0

- Add a Level 0 Rust mini-sqlite facade backed by in-memory tables.
- Support DB-API-inspired connection and cursor methods, qmark binding,
  snapshot commit/rollback, and SELECT delegation through the Rust SQL
  execution engine.
