# Changelog — CodingAdventures.SqlOptimizer

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-06-30

### Added
- `OptimizedPlan` abstract record hierarchy — mirrors `LogicalPlan` with two additions:
  - `OptScan` carries `RequiredColumns` (projection-pruning hint) and `ScanLimit` (limit push-down hint).
  - `OptEmptyResult` terminal that short-circuits dead plan branches.
- `IPass` interface — named, pure tree-rewriting pass.
- Five built-in optimization passes:
  1. `ConstantFoldingPass` — evaluates constant sub-expressions at compile time (arithmetic, comparisons, logical short-circuits, unary negation, string concatenation).
  2. `PredicatePushdownPass` — splits AND conjuncts and pushes each one as close to the scan as possible, passing through Sort, Distinct, and Project nodes.
  3. `ProjectionPruningPass` — collects referenced column names and annotates `OptScan.RequiredColumns` so the scan can avoid materialising unused columns.
  4. `DeadCodeEliminationPass` — replaces filter-on-FALSE, LIMIT 0, and any node whose input is `OptEmptyResult` (for inner/cross joins: both sides must be live) with `OptEmptyResult`.
  5. `LimitPushdownPass` — propagates a LIMIT count hint through Project and Filter nodes to `OptScan.ScanLimit`; blocked by Sort, Distinct, and Aggregate.
- `SqlOptimizer` static class:
  - `Lift(LogicalPlan)` — 1-to-1 structural conversion, no optimization.
  - `DefaultPasses()` — canonical five-pass list.
  - `Optimize(LogicalPlan)` — Lift + DefaultPasses, fixed-point iteration (max 10 rounds).
  - `OptimizeWithPasses(LogicalPlan, IReadOnlyList<IPass>)` — Lift + caller-supplied passes, fixed-point iteration.
- 50 xUnit `[Fact]` tests covering all passes, edge cases (div-by-zero, null comparisons, nested arithmetic), end-to-end pipeline, and all DDL plan types.
- `BUILD` / `BUILD_windows` — `dotnet test` invocation with 80 % line coverage threshold via coverlet.msbuild.
- `required_capabilities.json` — no capabilities required (pure in-memory library).
