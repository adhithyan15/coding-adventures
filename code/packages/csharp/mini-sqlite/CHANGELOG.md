# Changelog

## 0.2.0

Graduate to Level 1: wire the full sql-backend → sql-planner → sql-optimizer → sql-codegen → sql-vm pipeline.

### New features
- Full SQL pipeline: `SqlPlanner` → `SqlOptimizer` → `SqlCodegen` → `SqlVm` replaces direct interpretation.
- `BackendSchemaAdapter` bridges `InMemoryBackend` to `PlSchemaProvider` so the planner can resolve column names.
- `SqlStatementParser`: hand-rolled recursive-descent parser that emits planner `Statement` AST nodes directly, supporting the full SQL subset exercised by the 24 conformance fixtures.

### Bug fixes and extensions over Level 0
- **INSERT without column list**: expanded to explicit column list via `_backend.Columns()` before planning, fixing null-row inserts.
- **SELECT without FROM** (`SELECT expr`): short-circuited to a constant-expression evaluator (`EvalScalarSelect`) before the planner, which rejects FROM-less queries.
- **SELECT \***: expanded to explicit column list by querying the schema before planning.
- **ORDER BY non-projected columns**: hoisted missing sort-key columns into the projection temporarily, then stripped from the result.
- **LIMIT -1**: parsed as `null` count (unlimited rows) per SQLite semantics.
- **LIMIT 0 schema recovery**: restored result column names even when the optimizer eliminates the plan into `OptEmptyResult` (e.g. `WHERE FALSE` or `LIMIT 0`).
- **COUNT(\*)**: uses `AggFunc.CountStar` (counts nulls) vs `AggFunc.Count` (skips nulls) by detecting `AggArg.Star` in the codegen.
- **COUNT(DISTINCT col)**: distinct value deduplication implemented in `AggAccumulator.Seen` (`HashSet<object>`); duplicates and NULLs are skipped before accumulation.
- **HAVING + GROUP BY**: new codegen path (`CompileAggregateWithHaving` + `CompileHavingExpr`) that resolves `AggExpr` nodes in HAVING predicates to `FinalizeAgg(slot_i)` instructions, and limits emitted aggregate output columns to those in the SELECT projection (avoiding phantom HAVING-only aggregates appearing as extra columns).
- **NULL ORDER BY**: fixed default `NullOrder` to `NullsFirst` for ASC and `NullsLast` for DESC (matching SQLite's "NULL is less than everything" semantics); sort comparison (`CompareForSort`) applies null placement independently of direction so that `NULLS LAST` in DESC does not get double-negated.
- **CONCAT / `||`**: added to both `EvalBuiltinScalar` (from-dual path) and `SqlVm.EvalScalar` (table query path); NULL propagates per SQL standard.
- **ROUND(x)**: uses `MidpointRounding.AwayFromZero` to match SQLite's round-half-away-from-zero semantics instead of .NET's default banker's rounding.
- **Transaction rollback**: `Commit()` takes a new backend snapshot after committing so that a subsequent `Rollback()` can restore to the committed state.

### Packages modified
| Package | Files changed |
|---------|--------------|
| `CodingAdventures.MiniSqlite` | `MiniSqliteConnection.cs` (new parser + pipeline wiring + scalar eval + workarounds) |
| `CodingAdventures.SqlCodegen` | `SqlCodegen.cs` (COUNT(*) fix, HAVING support, LIMIT-0 schema recovery) |
| `CodingAdventures.SqlVm` | `SqlVm.cs` (CONCAT function, DISTINCT accumulator, sort fix) |

All 76 unit tests pass (74 + 2 new conformance fixtures `24-having-aggregate`, `23-null-in-order-by`).

## 0.1.0

- Add a Level 0 in-memory mini-sqlite facade.
- Support connections, cursors, qmark binding, basic DDL/DML, simple SELECT queries, and transaction snapshots.
