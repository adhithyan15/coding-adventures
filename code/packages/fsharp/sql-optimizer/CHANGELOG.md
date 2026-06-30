# Changelog — CodingAdventures.SqlOptimizer.FSharp

## [0.1.0] — 2026-06-30

### Added
- `OptimizedPlan` discriminated union — mirrors `LogicalPlan` with extra `Scan` hints
  (`requiredColumns`, `scanLimit`) and a new `EmptyResult` terminal case.
- `SqlOptimizer.lift` — pure conversion from `LogicalPlan` to `OptimizedPlan`.
- Five-pass optimization pipeline (`SqlOptimizer.defaultPasses`):
  1. **ConstantFolding** — bottom-up constant expression evaluation (arithmetic,
     comparison, boolean short-circuit, NULL propagation, `NOT`/`NEG`, `IS NULL`,
     `IS NOT NULL`). Division by zero is left for the VM.
  2. **PredicatePushdown** — split `AND` conjuncts, push filters through `Sort`,
     `Distinct`, `Project`, and `Join` (with outer-join safety).
  3. **ProjectionPruning** — top-down `(alias, column)` requirement tracking;
     annotates `Scan` nodes with the minimal column subset.
  4. **DeadCodeElimination** — converts `Filter(_, FALSE/NULL)` and `Limit(_, 0)`
     to `EmptyResult`; propagates `EmptyResult` through `Project`, `Sort`,
     `Limit`, `Distinct`, `Having`, INNER/CROSS `Join`, and `Union` sides.
     `Aggregate(EmptyResult)` is intentionally preserved (`COUNT(*)` must yield 0).
  5. **LimitPushdown** — attaches `scanLimit` hints through `Project` and `Filter`
     to `Scan` nodes when `OFFSET` is absent or zero.
- `SqlOptimizer.optimizeWithPasses` — apply a custom pass list.
- `SqlOptimizer.optimize` — convenience entry point using the default pipeline.
- 49 xUnit tests covering all five passes, lift, composition, and integration
  scenarios. Line coverage exceeds 80%.
