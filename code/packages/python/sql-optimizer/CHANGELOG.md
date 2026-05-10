# Changelog

## [0.8.0] - 2026-05-05

### Fixed

- **`ConstantFolding` now preserves `upsert` on `Insert` nodes**
  (`constant_folding.py`) — the pattern match for `Insert` was extended to capture
  `upsert=up`, call `_fold_upsert(up)` on it, and pass `upsert=new_up` to the
  rebuilt node.  Without this fix the optimizer silently stripped the `UpsertAction`
  from every `Insert` that passed through constant folding, causing the VM to
  execute a plain INSERT instead of the intended upsert.

### Added

- **`_fold_upsert(up: UpsertAction | None) -> UpsertAction | None`** — helper
  that applies constant folding to each assignment's `value` expression inside a
  `UpsertAction`.  Returns `None` unchanged, forwards DO-NOTHING actions unchanged,
  and rebuilds `UpsertAction` with folded assignments for DO-UPDATE actions.

## [0.7.0] - 2026-05-04

### Fixed

- **`ConstantFolding` silently dropped `on_conflict` from `Insert` nodes**
  (`constant_folding.py`) — the pattern match `case Insert(table=t, columns=cols,
  source=src, returning=ret)` did not capture `on_conflict`, so when rebuilding
  the node as `Insert(table=t, columns=cols, source=new_src, returning=new_ret)`
  it defaulted back to `None`.  This caused `INSERT OR REPLACE` and
  `INSERT OR IGNORE` to silently behave as plain `INSERT` after optimization,
  raising `IntegrityError` instead of replacing or ignoring conflicting rows.
  Fix: the pattern now captures `on_conflict=oc` and passes it through.

## [0.6.0] - 2026-05-04

### Fixed

- **`ConstantFolding` silent NULL for `||`** (`constant_folding.py`) — the
  `_apply_binary` function's `match` statement did not have a case for
  `BinaryOp.CONCAT`.  Python's structural pattern matching silently falls
  through unmatched cases and returns `None`, so `'hello' || 'world'` was
  constant-folded to `Literal(value=None)` instead of `Literal(value='helloworld')`.
  Added `case BinaryOp.CONCAT: return str(lv) + str(rv)` to fix this.

## [0.5.0] - 2026-05-04

### Fixed

- **`PredicatePushdown` outer-join correctness** — predicates on the
  null-padded side of an outer join were incorrectly being pushed inside
  the join's sub-scan, converting an outer join into a de-facto inner
  join. Fix: `_distribute_conjuncts` now consults the join `kind` before
  pushing to each side.
  - `LEFT JOIN` → only left-side predicates may be pushed into the left
    scan; right-side predicates remain above the join.
  - `RIGHT JOIN` → only right-side predicates may be pushed.
  - `FULL JOIN` → no single-side push at all.
  - `INNER` / `CROSS` → unchanged (both sides OK).

### Added

- `JoinKind` imported from `sql_planner` in `predicate_pushdown.py` to
  support the outer-join guard.

### Fixed (RETURNING)

- **`ConstantFolding` preserves `returning` on DML nodes** (`constant_folding.py`)
  — the `Insert`, `Update`, and `Delete` match arms in `_fold_plan` were
  rebuilding the nodes without passing the `returning` field, silently stripping
  the RETURNING clause before codegen.  All three cases now capture `returning=ret`
  and emit `returning=tuple(_fold_expr(r) for r in ret)` so that RETURNING
  expressions are folded rather than dropped.

## [0.4.0] - 2026-04-23

### Changed — Phase 9.7: Composite (multi-column) automatic index support (IX-8)

- **`ConstantFolding` `IndexScan` branch updated** — destructures `columns=cols`
  (was `column=col`) and reconstructs the node with `columns=cols` to stay
  consistent with the renamed field on `sql_planner.plan.IndexScan`.  No
  semantic change; the pass only rewrites `residual` predicates inside an
  `IndexScan`, leaving bound tuples and column metadata unchanged.

## [0.3.0] - 2026-04-21

### Added

- **`DerivedTable` plan node** now handled by all five optimizer passes — each
  pass recurses into `DerivedTable.query` so the inner plan is fully optimised:
  - `ConstantFolding` — folds constants inside the inner query.
  - `PredicatePushdown` — passes through unchanged (predicates cannot cross the
    derived-table boundary without aliasing analysis).
  - `ProjectionPruning` — passes through unchanged (inner columns are named by
    the derived-table alias; pruning would require alias mapping).
  - `DeadCodeElimination` — eliminates dead nodes inside the inner query.
  - `LimitPushdown` — recurses into the inner query but does NOT push the outer
    `LIMIT` inside (the inner query has its own shape).

- **`CaseExpr` expression node** now handled by expression-level passes:
  - `ConstantFolding` — folds each WHEN condition and result branch.  Short-
    circuit evaluation is intentionally deferred to the VM; the optimizer only
    constant-folds expressions that are already literals.
  - `PredicatePushdown` — traverses CASE branches during column-set extraction
    so predicates referencing columns inside CASE branches are handled correctly.

## [0.2.0] - 2026-04-21

### Added

- **`Intersect` and `Except` plan nodes** now handled by all five optimizer passes:
  - `ConstantFolding` — recursively folds both `left` and `right` sub-plans.
  - `PredicatePushdown` — passes through unchanged (predicates cannot cross a
    set-op boundary without altering semantics).
  - `ProjectionPruning` — passes through unchanged (both sides must retain their
    full column sets for the set operator to compare rows).
  - `DeadCodeElimination` — applies set-algebra short-circuits:
    - `Intersect` where either side is `EmptyResult` → `EmptyResult`.
    - `Except` where the left side is `EmptyResult` → `EmptyResult`.
    - `Except` where the right side is `EmptyResult` → left side unchanged.
  - `LimitPushdown` — passes through unchanged (pushing a `LIMIT` inside
    `INTERSECT`/`EXCEPT` changes the result set, so no hint is attached).

- **`Begin`, `Commit`, `Rollback` plan nodes** now handled by all five optimizer
  passes as transparent pass-throughs (no rewriting is applied to transaction
  control nodes).

## [0.1.0] - 2026-04-19

### Added

- Initial release. Pure rewrite passes over `sql-planner`'s `LogicalPlan`.
- `Pass` Protocol + `optimize(plan)` and `optimize_with_passes(plan, passes)`
  entry points.
- `ConstantFolding` pass — evaluates arithmetic / comparison / boolean
  operators on literals, propagates NULL per SQL three-valued logic,
  simplifies `TRUE AND x` / `FALSE OR x` / `NOT TRUE`.
- `PredicatePushdown` pass — splits AND-conjunctions and pushes each
  component as close to its source scan as possible (through Project,
  Sort; not through Aggregate, Limit, or Join bounds).
- `ProjectionPruning` pass — top-down required-column propagation
  annotating `Scan.required_columns`.
- `DeadCodeElimination` pass — `Filter(FALSE)` → `EmptyResult`,
  `Limit(0)` → `EmptyResult`, `Filter(TRUE)` is removed, empty Union
  branches collapse.
- `LimitPushdown` pass — attaches a `scan_limit` hint when safe to do so.
