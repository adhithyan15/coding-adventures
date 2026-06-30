# Changelog — coding-adventures-sql-optimizer (Java)

All notable changes to this package are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-06-30

### Added

- `SqlOptimizer` — top-level class with static `optimize`, `optimizeWithPasses`,
  `defaultPasses`, and `lift` API methods.
- `SqlOptimizer.OptimizedPlan` sealed interface mirroring `SqlPlanner.LogicalPlan`
  with all 15 variants plus:
  - `EmptyResult` — a new leaf node for provably-empty subtrees.
  - `Scan` extended with `requiredColumns` (nullable `List<String>`) and
    `scanLimit` (nullable `Long`) annotations set by later passes.
- `SqlOptimizer.Pass` interface — `name()` + `apply()` for composable transformations.
- **Five default optimization passes** (in pipeline order):
  1. `ConstantFolding` — arithmetic, comparison, Boolean short-circuit, NULL
     propagation, IS NULL/IS NOT NULL on literals.
  2. `PredicatePushdown` — AND-split + route conjuncts through Sort, Distinct,
     Project, and INNER/CROSS Join sides; outer-join safety guards.
  3. `ProjectionPruning` — top-down `(table, column)` required-set tracking;
     annotates Scan nodes with the minimal column list.
  4. `DeadCodeElimination` — converts `Filter(false)`, `Filter(null)`, `Limit(0)`,
     and cascading EmptyResult into `EmptyResult`; removes `Filter(true)`.
     Intentionally does NOT collapse `Aggregate(EmptyResult)`.
  5. `LimitPushdown` — propagates `LIMIT N` (with no offset) through Project and
     Filter to annotate Scan nodes with an early-stop row count.
- 45 JUnit 5 tests; JaCoCo line coverage ≥ 80 %.
