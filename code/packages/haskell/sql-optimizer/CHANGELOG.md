# Changelog

All notable changes to `sql-optimizer` (Haskell) will be documented here.

## [0.1.0] - 2026-06-30

### Added
- `OptimizedPlan` algebraic data type mirroring `LogicalPlan` with additions:
  - `EmptyResult` constructor for plans provably yielding zero rows
  - `optRequiredCols :: Maybe [String]` on `OptScan` (set by projection pruning)
  - `optScanLimit :: Maybe Integer` on `OptScan` (set by limit pushdown)
- `Pass` record type: `{ passName :: String, passApply :: OptimizedPlan -> OptimizedPlan }`
- `lift :: LogicalPlan -> OptimizedPlan` — structural isomorphism, no rewrites
- `defaultPasses :: [Pass]` — five-pass optimization pipeline
- `optimize :: LogicalPlan -> OptimizedPlan` — apply default pipeline
- `optimizeWithPasses :: [Pass] -> LogicalPlan -> OptimizedPlan` — custom pipeline

### Optimization Passes
1. **constantFolding** — bottom-up arithmetic, boolean short-circuit, NULL propagation, IsNull/IsNotNull on literals
2. **predicatePushdown** — split AND conjuncts, push through Sort/Distinct/Project/InnerJoin/CrossJoin; stops at Aggregate/Limit/OuterJoin
3. **projectionPruning** — top-down required-column annotation on OptScan nodes
4. **deadCodeElimination** — collapses Filter-FALSE, Limit-0, EmptyResult propagation through Project/Sort/Limit/Distinct/Inner-Join/Having/Union
5. **limitPushdown** — propagates row-count from OptLimit through Project/Filter down to OptScan.optScanLimit

### Tests
- 47 hspec tests across Lift, ConstantFolding, PredicatePushdown, ProjectionPruning, DeadCodeElimination, LimitPushdown, EndToEnd, and Pass sections
