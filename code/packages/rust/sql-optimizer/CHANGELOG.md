# Changelog — coding-adventures-sql-optimizer

All notable changes to this crate will be documented here.

## [0.1.5] - Unreleased

### Changed

- Constant folding and column collection recurse into the new `SqlExpr::Like`
  `escape` operand.

## [0.1.4] - Unreleased

### Changed

- **Preserve `SortKey.collation` through constant folding.** `fold_plan` now
  carries the new `collation` field on `Sort` keys so a `COLLATE` clause
  survives optimization.

## [0.1.3] - Unreleased

### Added

- Handle `SqlExpr::Case`: `fold_expr` folds each condition/value and the ELSE
  while keeping the branch structure (short-circuiting stays the VM's job), and
  `collect_columns_in_expr` recurses into every branch and the ELSE.

## [0.1.2] - Unreleased

### Changed

- Carry the new `SortKey.nulls_first` field through constant-folding of `Sort`
  plan nodes.

## [0.1.1] - Unreleased

### Added

- Handle the new `SqlExpr::Cast` variant: `fold_expr` folds the cast's operand
  but keeps the conversion (a cast of a constant is still evaluated by the VM),
  and `collect_columns_in_expr` recurses into the operand.

## [0.1.0] — 2026-06-30

### Added

- `OptimizedPlan` enum: mirrors `LogicalPlan` from `sql-planner` with two
  extra annotations on `Scan` (`required_columns: Option<Vec<String>>` and
  `scan_limit: Option<i64>`) and a new `EmptyResult` sentinel variant.
- `Pass` trait: `name() -> &str` + `apply(OptimizedPlan) -> OptimizedPlan`.
- `lift(LogicalPlan) -> OptimizedPlan`: structural conversion, no optimization.
- `optimize(LogicalPlan) -> OptimizedPlan`: apply all five default passes.
- `optimize_with_passes(LogicalPlan, &[&dyn Pass]) -> OptimizedPlan`:
  caller-supplied pass list.
- `default_passes() -> Vec<Box<dyn Pass>>`: returns the five passes in order.

### Five optimization passes

1. **ConstantFoldingPass** — bottom-up evaluation of constant sub-expressions;
   SQL three-valued logic (NULL propagation, AND/OR short-circuits); integer
   arithmetic via `i64::checked_*`; string concatenation.

2. **PredicatePushdownPass** — pushes `Filter` nodes through `Sort`,
   `Project`, and `Distinct`; blocked by `Limit` and `Aggregate`.

3. **ProjectionPruningPass** — top-down required-column propagation;
   annotates `Scan.required_columns` with only the columns the rest of the
   plan needs; wildcard (`*`) disables pruning.

4. **DeadCodeEliminationPass** — replaces `Filter(_, FALSE)`,
   `Filter(_, NULL)`, and `Limit(_, Some(0))` with `EmptyResult`;
   propagates `EmptyResult` upward through `Project`, `Sort`, `Limit`,
   `Distinct`, `Having`, and `INNER`/`CROSS` joins; does NOT eliminate
   `Aggregate(EmptyResult)` (COUNT(*) must still return 0).

5. **LimitPushdownPass** — attaches `scan_limit = count + offset` hints to
   `Scan` nodes; pushes through `Sort` but not through `Filter`; preserves
   the tighter of two competing hints.

### Test coverage

- 60+ unit tests covering lift, all five passes individually, and
  full-pipeline integration scenarios.
