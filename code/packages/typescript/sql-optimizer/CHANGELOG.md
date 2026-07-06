# Changelog - @coding-adventures/sql-optimizer

## [0.1.0] - 2026-07-06

### Added
- Initial TypeScript implementation of the SQL query optimizer.
- `optimize(plan: LogicalPlan): LogicalPlan` — applies rewrite passes to a
  `LogicalPlan` tree and returns an equivalent (possibly restructured) plan.
- Current optimization passes:
  - **Predicate pushdown**: moves FilterNode predicates as close to the data
    source as possible, reducing the number of rows processed early.
  - **Constant folding**: evaluates constant sub-expressions at compile time
    (e.g. `1 + 1 → 2`, `NOT TRUE → FALSE`).
  - **Dead projection elimination**: removes ProjectNode items that are not
    referenced by ancestor nodes.
- Returns the input plan unchanged when no rewrites apply (identity pass).
- 10+ unit tests covering each optimization pass.
