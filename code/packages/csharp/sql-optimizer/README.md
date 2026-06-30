# CodingAdventures.SqlOptimizer

Logical query optimizer for the Mini-SQLite Level 1 C# pipeline.

Accepts a `LogicalPlan` tree produced by `CodingAdventures.SqlPlanner` and
returns an `OptimizedPlan` tree that carries execution hints and has dead
branches removed.

## Where it fits

```
sql-lexer → sql-parser → sql-planner → [sql-optimizer] → sql-codegen → sql-vm
```

The optimizer is a pure in-memory library: no I/O, no database access.

## Quick start

```csharp
using CodingAdventures.SqlPlanner;
using CodingAdventures.SqlOptimizer;

// ... obtain a LogicalPlan from SqlPlanner ...
LogicalPlan logical = planner.Plan(stmt);

// Optimize with the default five-pass pipeline
OptimizedPlan opt = SqlOptimizer.Optimize(logical);
```

## Five optimization passes

### 1. Constant Folding (`ConstantFoldingPass`)

Evaluates sub-expressions whose operands are all literals at "compile time".

```
1 + 2            →  3
TRUE AND col     →  col
FALSE AND col    →  FALSE
NOT TRUE         →  FALSE
-(5)             →  -5
"hello" + " "   →  "hello "
```

### 2. Predicate Pushdown (`PredicatePushdownPass`)

Splits `AND` conjuncts and moves each one as close to the scan as possible,
reducing the number of rows that expensive operators (Sort, Join) must process.

Transparent nodes (the predicate passes through): `Sort`, `Distinct`, `Project`.

Blocking nodes: `Aggregate`, `Having`, `Union`.

### 3. Projection Pruning (`ProjectionPruningPass`)

Inspects the column names referenced by `Project` and `Filter` expressions and
annotates `OptScan.RequiredColumns`.  The execution engine can use this hint to
skip materialising columns that are never read.

### 4. Dead Code Elimination (`DeadCodeEliminationPass`)

Replaces provably empty plan branches with `OptEmptyResult`:

| Condition | Rewrite |
|-----------|---------|
| `Filter(_, FALSE)` | `EmptyResult` |
| `Filter(_, TRUE)` | input (tautology elimination) |
| `Limit(_, 0, _)` | `EmptyResult` |
| Any node whose input is `EmptyResult` | `EmptyResult` |
| Inner/Cross join where either side is `EmptyResult` | `EmptyResult` |

### 5. Limit Pushdown (`LimitPushdownPass`)

Propagates the row count from a `LIMIT` clause down to `OptScan.ScanLimit`.
The scan can stop reading after that many rows, avoiding a full table scan.

Blocked by `Sort` and `Distinct` (which require all rows before producing output).

## API reference

### `SqlOptimizer` (static class)

| Method | Description |
|--------|-------------|
| `Lift(LogicalPlan)` | 1:1 structural conversion, no optimization |
| `DefaultPasses()` | Returns the canonical 5-pass list |
| `Optimize(LogicalPlan)` | Lift + DefaultPasses + fixed-point iteration |
| `OptimizeWithPasses(LogicalPlan, IReadOnlyList<IPass>)` | Lift + custom passes + fixed-point |

### `IPass` interface

```csharp
public interface IPass {
    string Name { get; }
    OptimizedPlan Apply(OptimizedPlan plan);
}
```

Implement this interface to add custom optimization passes.

## `OptimizedPlan` node types

| Node | Description |
|------|-------------|
| `OptScan(Table, Alias, RequiredColumns?, ScanLimit?)` | Table scan with optimizer annotations |
| `OptFilter(Input, Predicate)` | Row filter |
| `OptProject(Input, Columns)` | Column projection |
| `OptJoin(Left, Right, Kind, Condition?)` | Join |
| `OptAggregate(Input, GroupBy, Aggregates)` | GROUP BY + aggregation |
| `OptHaving(Input, Predicate)` | HAVING filter |
| `OptSort(Input, Keys)` | ORDER BY |
| `OptLimit(Input, Count?, Offset?)` | LIMIT / OFFSET |
| `OptDistinct(Input)` | DISTINCT deduplication |
| `OptUnion(Left, Right, All)` | UNION / UNION ALL |
| `OptInsert` / `OptUpdate` / `OptDelete` | DML |
| `OptCreateTable` / `OptDropTable` | DDL |
| `OptEmptyResult` | Provably empty result (produced by dead-code elimination) |

## Running the tests

```
dotnet test tests/CodingAdventures.SqlOptimizer.Tests/CodingAdventures.SqlOptimizer.Tests.csproj \
    --disable-build-servers \
    /p:CollectCoverage=true \
    "/p:Include=[CodingAdventures.SqlOptimizer]*" \
    /p:Threshold=80 \
    /p:ThresholdType=line
```

Or via the BUILD file:

```sh
# Unix
cat BUILD | sh

# Windows
Get-Content BUILD_windows | Invoke-Expression
```

## Package information

| Property | Value |
|----------|-------|
| Package ID | `CodingAdventures.SqlOptimizer` |
| Target framework | `net9.0` |
| License | MIT |
| Dependencies | `CodingAdventures.SqlPlanner` |
