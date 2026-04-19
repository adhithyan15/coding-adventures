# Changelog

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
