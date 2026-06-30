# Changelog — coding-adventures-sql-optimizer

All notable changes to this package will be documented here.

## [0.1.0] — 2026-06-30

### Added

- **`OptimizedPlan`** sealed class — mirrors all `LogicalPlan` variants plus:
  - `Scan.requiredColumns: List<String>?` — null means "all columns"
  - `Scan.scanLimit: Long?` — pushed-down row-count limit for early termination
  - `EmptyResult` object — sentinel for provably-empty subtrees
- **`Pass` interface** — `name: String` + `apply(OptimizedPlan): OptimizedPlan`
- **`SqlOptimizer` object** — public entry point with:
  - `lift(LogicalPlan): OptimizedPlan` — structural 1:1 conversion
  - `defaultPasses(): List<Pass>` — ordered pass pipeline
  - `optimize(LogicalPlan): OptimizedPlan` — lift + run all default passes
  - `optimizeWithPasses(LogicalPlan, List<Pass>): OptimizedPlan` — custom pipeline
- **5 optimization passes** (all stateless, composable):
  1. `ConstantFoldingPass` — fold literal arithmetic/boolean/comparison at compile time; AND/OR short-circuit; NULL propagation; division by zero intentionally NOT folded
  2. `PredicatePushdownPass` — push `Filter` through `Project`, `Sort`, `Distinct`; distribute AND conjuncts to INNER join sides; respect LEFT/RIGHT join semantics; stop at `Aggregate` and `Limit`
  3. `ProjectionPruningPass` — top-down required-column tracking; annotate `Scan.requiredColumns`; `SELECT *` disables pruning
  4. `DeadCodeEliminationPass` — `Filter(FALSE/NULL)` → `EmptyResult`; `Limit(0)` → `EmptyResult`; propagate through `Project`, `Sort`, `Distinct`, `Limit`, `Having`; INNER `Join(EmptyResult)` → `EmptyResult`; `Union(EmptyResult, x)` → `x`; `Aggregate` intentionally NOT eliminated (COUNT returns 0)
  5. `LimitPushdownPass` — push `LIMIT N` (with no OFFSET or OFFSET=0) through `Project` and `Filter` to `Scan.scanLimit`; stop at `Sort`, `Aggregate`, `Join`, `Distinct`; nested limits take the minimum
- **42 JUnit 5 tests** covering all 5 passes, edge cases, NULL propagation, DML pass-through, and multi-pass pipeline integration
- JaCoCo coverage threshold: 80% line coverage enforced via `check` task
- `required_capabilities.json` — no capabilities required (pure in-memory)
