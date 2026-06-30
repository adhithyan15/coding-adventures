# coding-adventures-sql-optimizer (Kotlin)

A pure Kotlin logical query plan optimizer for the coding-adventures mini-sqlite
Level 1 pipeline. Takes a `LogicalPlan` from `coding-adventures-sql-planner` and
applies a pipeline of rule-based optimization passes to produce an `OptimizedPlan`.

## Where It Fits

```
SqlParser  →  SqlPlanner  →  SqlOptimizer  →  SqlCodegen  →  SqlVm
(string)      (LogicalPlan)  (OptimizedPlan)  (bytecode)     (results)
```

This package sits between the planner and the code generator. Its output,
`OptimizedPlan`, is structurally identical to `LogicalPlan` but carries two
extra annotations on `Scan` nodes:

| Field             | Meaning                                                 |
|-------------------|---------------------------------------------------------|
| `requiredColumns` | Only these columns need to be fetched (null = all)      |
| `scanLimit`       | Stop after this many rows (null = no early termination) |

And one extra node type:

| Node          | Meaning                                              |
|---------------|------------------------------------------------------|
| `EmptyResult` | Subtree is provably empty — skip execution entirely  |

## Optimization Passes

Passes are applied in order by `SqlOptimizer.optimize()`. Each pass is stateless
and implements the `Pass` interface (`name` + `apply`).

### 1. ConstantFolding

Evaluates sub-expressions containing only literal values at "compile time":

```kotlin
1 + 1            // → Literal(2)
true AND false   // → Literal(false)
x AND false      // → Literal(false)   (short-circuit)
x OR  true       // → Literal(true)    (short-circuit)
NULL + 5         // → Literal(null)    (NULL propagation)
NULL IS NULL     // → Literal(true)
```

Division by zero is intentionally **not** folded — the runtime produces the error.

### 2. PredicatePushdown

Moves `Filter` nodes as close to their data source as possible:

```
Filter(Project(Scan, cols), p)   →   Project(Filter(Scan, p), cols)
Filter(Sort(input, keys), p)     →   Sort(Filter(input, p), keys)
Filter(Distinct(input), p)       →   Distinct(Filter(input, p))
```

For `INNER JOIN`, AND conjuncts are split and pushed to the side they reference.
For `LEFT`/`RIGHT` join, only conjuncts that reference the preserved side are pushed.
Pushdown stops at `Aggregate` and `Limit`.

### 3. ProjectionPruning

Performs a top-down traversal carrying a set of required `(table, column)` pairs.
At each `Scan`, the set is intersected with the scan's alias to populate
`Scan.requiredColumns`. `SELECT *` disables pruning for that branch.

### 4. DeadCodeElimination

Replaces provably-empty subtrees with `EmptyResult`:

```
Filter(_, FALSE)                  → EmptyResult
Limit(_, count=0)                 → EmptyResult
Project(EmptyResult, _)           → EmptyResult
Join(EmptyResult, _, INNER)       → EmptyResult
Union(EmptyResult, x)             → x
Aggregate(EmptyResult, ...)       → Aggregate(...)  ← NOT eliminated
```

`Aggregate` is intentionally preserved: `SELECT COUNT(*) FROM empty_table` must
return the row `(0)`, not an empty result set.

### 5. LimitPushdown

Propagates `LIMIT N` (with no OFFSET or OFFSET=0) down through row-preserving
operators to set `Scan.scanLimit`, allowing storage to stop early:

```
Limit(Project(Filter(Scan, p), cols), N)
→  Limit(Project(Filter(Scan(scanLimit=N), p), cols), N)
```

Stops at `Sort`, `Aggregate`, `Join`, and `Distinct` (they must see all rows).
Multiple nested `Limit` nodes take the minimum.

## API

```kotlin
import com.codingadventures.sqloptimizer.*
import com.codingadventures.sqlplanner.*

// Lift + run all 5 default passes
val optimized: OptimizedPlan = SqlOptimizer.optimize(logicalPlan)

// Lift only (no passes)
val lifted: OptimizedPlan = SqlOptimizer.lift(logicalPlan)

// Custom pass list
val optimized = SqlOptimizer.optimizeWithPasses(logicalPlan, listOf(
    ConstantFoldingPass,
    DeadCodeEliminationPass
))

// Implement your own pass
object MyPass : Pass {
    override val name = "MyPass"
    override fun apply(plan: OptimizedPlan): OptimizedPlan = TODO()
}
```

## Building

First build the sql-planner JAR (dependency):

```sh
cd ../sql-planner && gradle jar
```

Then build and test the optimizer:

```sh
cd ../sql-optimizer
gradle test          # runs tests + JaCoCo coverage report
gradle check         # additionally enforces ≥80% line coverage
```

Output JAR: `gradle-build/libs/coding-adventures-sql-optimizer-0.1.0.jar`
Coverage report: `gradle-build/reports/jacoco/test/html/index.html`

## Tests

42 JUnit 5 tests covering:

- Constant folding: arithmetic, boolean short-circuit, NULL propagation, IS NULL,
  BETWEEN, IN, NOT IN, string concatenation, division-by-zero safety
- Predicate pushdown: through Project/Sort/Distinct; stop at Aggregate/Limit;
  INNER join split; LEFT/RIGHT join direction enforcement
- Projection pruning: required-column annotation; SELECT * disables pruning
- Dead code elimination: Filter(FALSE), Limit(0), EmptyResult propagation,
  Aggregate preservation, Union simplification
- Limit pushdown: through Project/Filter; stop at Sort/Aggregate/Join/Distinct;
  offset guard; nested limit minimum
- DML pass-through: INSERT, UPDATE, DELETE, CREATE TABLE, DROP TABLE
- Full-pipeline integration: CF→DCE folding false predicates, LP+PP combined

## Package Structure

```
sql-optimizer/
├── src/main/kotlin/com/codingadventures/sqloptimizer/
│   └── SqlOptimizer.kt          # OptimizedPlan, Pass, 5 passes, SqlOptimizer
├── src/test/kotlin/com/codingadventures/sqloptimizer/
│   └── SqlOptimizerTest.kt      # 42 JUnit 5 tests
├── build.gradle.kts
├── settings.gradle.kts
├── BUILD
├── BUILD_windows
├── CHANGELOG.md
├── README.md
└── required_capabilities.json
```

## Dependencies

- **Kotlin 2.1.20** (JVM 21 target)
- **coding-adventures-sql-planner 0.1.0** (local JAR)
- **JUnit 5.11.4** (test)
- **JaCoCo 0.8.12** (coverage)
