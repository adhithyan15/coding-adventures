# CodingAdventures.SqlOptimizer.FSharp

Logical query optimizer for the Mini-SQLite Level 1 pipeline. Transforms a
`LogicalPlan` (from `CodingAdventures.SqlPlanner.FSharp`) into an
`OptimizedPlan` through five composable, pure-function passes.

## What it does

A `LogicalPlan` is a direct translation of a SQL statement — it is correct
but not necessarily cheap. This optimizer applies algebraic rewrites that
preserve semantics while reducing execution cost:

| Pass | What it does |
|------|-------------|
| **ConstantFolding** | Evaluates `1 + 2 → 3`, `TRUE AND x → x`, `NULL + x → NULL` at plan time |
| **PredicatePushdown** | Moves `Filter` nodes below `Sort`/`Distinct`/`Join` to reduce rows early |
| **ProjectionPruning** | Annotates `Scan` with the minimal column subset the query needs |
| **DeadCodeElimination** | Replaces `Filter(FALSE)` / `LIMIT 0` subtrees with `EmptyResult` |
| **LimitPushdown** | Attaches `scanLimit` hints to `Scan` nodes for early-termination backends |

## How it fits in the stack

```
sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm
                            (this package)
```

## Usage

```fsharp
open CodingAdventures.SqlPlanner.FSharp
open CodingAdventures.SqlOptimizer.FSharp

// From a logical plan (e.g. produced by Planner.plan):
let logical = LogicalPlan.Filter(
    LogicalPlan.Scan("users", None),
    Expr.BinaryOp(Eq, Expr.Literal(SqlValue.Integer 1L), Expr.Literal(SqlValue.Integer 1L)))

// Optimize with the default 5-pass pipeline:
let optimized = SqlOptimizer.optimize logical
// → OptimizedPlan.Scan("users", None, None, None)   (filter removed, 1=1 folded to TRUE)

// Or apply a custom subset of passes:
let passes = SqlOptimizer.defaultPasses () |> List.filter (fun p -> p.Name = "ConstantFolding")
let partial = SqlOptimizer.optimizeWithPasses passes logical
```

## OptimizedPlan vs LogicalPlan

`OptimizedPlan` mirrors every case of `LogicalPlan` with two additions:

- `Scan` carries `requiredColumns: string list option` — column subset hint for
  columnar backends. `None` = all columns.
- `Scan` carries `scanLimit: int64 option` — row-count hint for early termination.
  `None` = no limit.
- `EmptyResult` — a new terminal case that signals "this subtree provably returns
  zero rows".

## Literate design notes

### NULL semantics

SQL uses three-valued logic. The `ConstantFolding` pass replicates the full
truth tables:

```
FALSE AND NULL = FALSE    (FALSE dominates)
TRUE  AND NULL = NULL     (can't short-circuit)
TRUE  OR  NULL = TRUE     (TRUE dominates)
FALSE OR  NULL = NULL     (can't short-circuit)
NULL  + 5      = NULL     (arithmetic NULL-propagation)
```

### Outer-join safety in PredicatePushdown

Pushing a right-side predicate inside a `LEFT JOIN` corrupts null-padding
semantics. We only push to a side when the join kind doesn't null-pad that side:

- `INNER / CROSS` → both sides safe
- `LEFT` → left side only
- `RIGHT` → right side only
- `FULL` → neither side

### Why Aggregate(EmptyResult) is preserved

`SELECT COUNT(*) FROM empty_table` must return one row: `0`. If we replaced
`Aggregate(EmptyResult)` with `EmptyResult`, the VM would return no rows at all.
All other operators propagate `EmptyResult` upward.

## Testing

```
dotnet test tests/CodingAdventures.SqlOptimizer.Tests/CodingAdventures.SqlOptimizer.Tests.fsproj
```

49 xUnit tests cover all five passes, the `lift` function, custom pass
composition, and multi-pass integration scenarios. Line coverage is ≥ 80%.

## Package info

| Field | Value |
|-------|-------|
| NuGet ID | `CodingAdventures.SqlOptimizer.FSharp` |
| Version | 0.1.0 |
| Target | `net9.0` |
| License | MIT |
| Depends on | `CodingAdventures.SqlPlanner.FSharp` |
