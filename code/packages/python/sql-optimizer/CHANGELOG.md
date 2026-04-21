# Changelog

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
