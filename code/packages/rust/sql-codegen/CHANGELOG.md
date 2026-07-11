# Changelog

## [0.5.0] - Unreleased

### Added

- **`FULL OUTER JOIN`** now executes correctly instead of degrading to a cross
  product. A single nested loop can't tell whether a given right row matched
  *any* left row (the inner side is re-scanned per outer row), so
  `compile_join_projected` now emits **two passes** for `JoinKind::Full`: pass 1
  is an ordinary LEFT JOIN (matched pairs + left-only rows), and pass 2 is a
  RIGHT *anti*-join that evaluates `ON` but suppresses the matched-pair emit —
  it emits only the right rows that matched no left row, NULL-padded on the
  left. The union is a FULL JOIN with no duplicated matched pairs. Ordering
  across the two passes is handled by the surrounding `ORDER BY` sort.
- Refactored the join loop body out of `compile_join_projected` into a reusable
  `emit_join_pass(outer, inner, condition, eval_condition, emit_matched,
  emit_unmatched, columns)` helper. INNER/LEFT/RIGHT/CROSS are now single calls
  with the appropriate flags; FULL is two calls. **No new VM instructions** —
  both passes reuse the existing `ClearMatch`/`SetMatch`/`JumpIfMatched`
  match-flag machinery.

## [0.4.0] - Unreleased

### Added

- **SQLite-style column names for un-aliased function calls.** `output_column_name`
  now labels an un-aliased function-call output column with the reconstructed
  call text — `SELECT UPPER(name), LENGTH(name)` yields columns `UPPER(name)` and
  `LENGTH(name)` instead of two `?`s, matching what real SQLite returns (an
  un-aliased expression column is named after its source text). A new
  `render_expr_label` helper best-effort-renders columns, simple literals, and
  nested calls, returning `None` (→ `?`) for shapes it can't faithfully print, so
  we never emit a misleading name. Aliases still win, and non-function complex
  expressions keep the `?` default. Together with the sql-vm positional-projection
  fix this retires the differential-oracle `string_functions` divergence.

### Removed

- Dropped the unused top-level `SortKey` import (it is re-imported inside the test
  module where it is actually used), clearing a compiler warning.

## [0.3.0] - Unreleased

### Added

- **`LEFT` and `RIGHT OUTER JOIN`** now execute correctly. `compile_join_projected`
  keeps a per-outer-row match flag (new `ClearMatch`/`SetMatch`/`JumpIfMatched`
  instructions, requiring matching sql-vm support): for each outer row it clears
  the flag, sets it whenever the `ON` condition holds, and after the inner loop
  emits one NULL-padded row iff nothing matched — the NULL padding falls out
  because `CloseScan` drops the inner cursor, so its columns read NULL while the
  outer cursor keeps its values. `RIGHT a b` compiles as `LEFT b a` (roles swap;
  the projection references each table by name, so the output is unchanged).
  Verified against real SQLite by the mini-sqlite differential oracle: `left_join`
  and `right_join` now match and are removed from the known-divergence ledger.
- `FULL OUTER JOIN` still degrades to a cross product (a single forward pass
  can't emit the unmatched right rows); it stays in the ledger for a later
  increment.

## [0.2.0] - Unreleased

### Fixed

- **Qualified columns across a join now resolve correctly.** `SELECT a.name,
  b.tag FROM a JOIN b ON a.id = b.aid` previously returned a single all-`NULL`
  row: a `FROM a`/`FROM b` with no `AS` alias opened both cursors under the same
  `None` key (so they collided and every `a.x`/`b.y` read whichever advanced
  last), and the projection was emitted *after* the join loop with no live
  cursor. New `compile_join_projected` keys each side by its **effective alias**
  (explicit alias, else table name — exactly what a `LoadColumn` qualifier looks
  up) and emits the projected columns *inside* the inner loop, so both the `ON`
  condition and the output columns resolve against the right row. `Project(Join)`
  now routes through this path. Verified against real SQLite by the mini-sqlite
  differential-conformance oracle (the `inner_join` case, previously a ledger
  divergence, now matches). Outer joins still degrade to a cross product (tracked
  separately); their columns now resolve correctly too.

## [0.1.0] - 2026-07-01

### Added

- Initial implementation of the Rust bytecode code generator for the Mini-SQLite Level 1 pipeline.
- `compile(plan: &OptimizedPlan) -> Program` public API function.
- `Program` struct containing a flat `Vec<Instruction>`.
- Complete `Instruction` enum with 37 variants covering:
  - Stack ops: `LoadConst`, `LoadColumn`
  - Arithmetic/logic: `BinaryOpInstr`, `UnaryOpInstr`
  - Null tests: `IsNull`, `IsNotNull`
  - Predicates: `Between`, `Like`, `InList`
  - Scan control: `OpenScan`, `AdvanceCursor`, `JumpIfExhausted`, `CloseScan`
  - Row assembly: `BeginRow`, `EmitColumn`, `EmitRow`
  - Aggregation: `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey`
  - Control flow: `Label`, `Jump`, `JumpIfTrue`, `JumpIfFalse`, `Halt`
  - DDL: `CreateTableInstr`, `DropTableInstr`
  - DML: `InsertRow`, `UpdateRows`, `DeleteRows`
  - Transactions: `BeginTransaction`, `CommitTransaction`, `RollbackTransaction`
  - Post-processing: `SortResult`, `DistinctResult`, `LimitResult`
- `BinaryOp`, `UnaryOp`, `AggFn`, `CompiledSortKey` supporting types.
- Post-op peeling: `Sort`, `Limit`, `Distinct` wrappers are stripped and emitted after `Halt`.
- Scan loop pattern: `OpenScan` → `Label` → `AdvanceCursor` → `JumpIfExhausted` → body → `Jump` → `Label` → `CloseScan`.
- Filter compilation: predicate check with `JumpIfFalse` inside the scan loop.
- Project compilation: `BeginRow` + per-column expression + `EmitColumn` + `EmitRow`.
- Aggregate compilation: two-phase (accumulation loop + finalization/emission).
- Having compilation: predicate check after FinalizeAgg, before EmitRow.
- Join compilation: nested-loop join with optional ON condition.
- INSERT compilation: one `InsertRow` per VALUES row.
- UPDATE compilation: scan loop with optional predicate filter + `UpdateRows`.
- DELETE compilation: scan loop with optional predicate filter + `DeleteRows`.
- DDL compilation: `CreateTableInstr` and `DropTableInstr` as single-instruction programs.
- Thread-local recursion depth guard (`MAX_EXPR_DEPTH = 512`) to prevent stack overflow.
- 50+ unit tests covering all major compilation paths.
- Knuth-style literate programming: all code includes inline explanations, diagrams, and analogies.
