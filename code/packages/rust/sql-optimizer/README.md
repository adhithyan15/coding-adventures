# coding-adventures-sql-optimizer

Logical query plan optimizer for the Mini-SQLite SQL pipeline (Rust, Level 1).

## Where it fits

```
sql-lexer → sql-parser → sql-planner → sql-optimizer → (sql-codegen) → (sql-vm) → mini-sqlite
```

This crate accepts a `LogicalPlan` produced by `sql-planner` and applies five
optimization passes to produce an `OptimizedPlan`. The optimized plan is
semantically equivalent to the original but faster to execute.

## Optimization passes

| # | Pass                 | What it does                                               |
|---|---------------------|------------------------------------------------------------|
| 1 | ConstantFolding     | Evaluates constant sub-expressions at plan time            |
| 2 | PredicatePushdown   | Moves Filter nodes closer to Scan leaves                   |
| 3 | ProjectionPruning   | Annotates Scans with only the columns they need            |
| 4 | DeadCodeElimination | Replaces provably-empty subtrees with `EmptyResult`        |
| 5 | LimitPushdown       | Attaches `scan_limit` hints to Scan nodes under a Limit    |

## Usage

```rust
use coding_adventures_sql_optimizer::{optimize, OptimizedPlan};
use coding_adventures_sql_planner::LogicalPlan;

let plan = LogicalPlan::Scan { table: "users".into(), alias: None };
let opt: OptimizedPlan = optimize(plan);
// → OptimizedPlan::Scan { table: "users", alias: None,
//                          required_columns: None, scan_limit: None }
```

Custom pass list:

```rust
use coding_adventures_sql_optimizer::{optimize_with_passes, ConstantFoldingPass, DeadCodeEliminationPass};

let passes: Vec<&dyn coding_adventures_sql_optimizer::Pass> = vec![
    &ConstantFoldingPass,
    &DeadCodeEliminationPass,
];
let opt = optimize_with_passes(plan, &passes);
```

## Key types

- `OptimizedPlan` — mirrors `LogicalPlan` with two extra fields on `Scan`
  (`required_columns`, `scan_limit`) and a new `EmptyResult` variant.
- `Pass` — trait for optimization passes.
- `optimize(plan)` — apply all five passes.
- `optimize_with_passes(plan, passes)` — apply a custom pass list.
- `default_passes()` — return the five default passes.

## Building

```sh
# Linux/macOS
./BUILD

# Windows
.\BUILD_windows.bat
```

Or directly:

```sh
set CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=rust-lld
cargo test --package coding-adventures-sql-optimizer
```
