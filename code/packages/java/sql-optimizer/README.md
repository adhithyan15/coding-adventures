# coding-adventures-sql-optimizer (Java)

A pure in-memory SQL logical query optimizer written in Java 21.

## What it does

`SqlOptimizer` transforms a `SqlPlanner.LogicalPlan` (from the
`coding-adventures-sql-planner` package) into an `OptimizedPlan` by running a
configurable pipeline of optimization passes.

The five default passes are:

| # | Pass                  | What it does                                                               |
|---|-----------------------|----------------------------------------------------------------------------|
| 1 | ConstantFolding       | Evaluates literal arithmetic, comparisons, Boolean logic, and NULL rules.  |
| 2 | PredicatePushdown     | Moves Filter nodes closer to Scans; splits AND conjuncts for joins.        |
| 3 | ProjectionPruning     | Annotates Scan nodes with only the columns they need to emit.              |
| 4 | DeadCodeElimination   | Replaces provably-empty subtrees with `EmptyResult`; removes trivial TRUE filters. |
| 5 | LimitPushdown         | Propagates `LIMIT N` to Scan nodes as an early-stop row count.             |

## How it fits in the stack

```
sql-lexer → sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm
                                           ↑ this package
```

## Usage

```java
import com.codingadventures.sqlplanner.SqlPlanner;
import com.codingadventures.sqloptimizer.SqlOptimizer;

var schema  = new SqlPlanner.InMemorySchemaProvider(Map.of("users", List.of("id", "name", "age")));
var planner = new SqlPlanner(schema);
var logical = planner.plan(stmt);            // LogicalPlan
var opt     = SqlOptimizer.optimize(logical); // OptimizedPlan
```

### Custom pass pipeline

```java
var opt = SqlOptimizer.optimizeWithPasses(logical, List.of(
    new SqlOptimizer.ConstantFolding(),
    new SqlOptimizer.DeadCodeElimination()
));
```

### Implementing your own pass

```java
public class MyPass implements SqlOptimizer.Pass {
    @Override public String name() { return "MyPass"; }
    @Override public SqlOptimizer.OptimizedPlan apply(SqlOptimizer.OptimizedPlan plan) {
        // ... transform and return
        return plan;
    }
}
```

## OptimizedPlan extensions

`OptimizedPlan` is a sealed interface mirroring `LogicalPlan`, with two additions:

- **`EmptyResult`** — a leaf node meaning "zero rows will ever be produced".
  Introduced by `DeadCodeElimination`; propagates upward through Filter, Sort,
  Project, INNER/CROSS Join, etc.  Intentionally does *not* collapse
  `Aggregate(EmptyResult)` because `SELECT COUNT(*) FROM empty` must return one row.

- **`Scan` annotations** — `Scan` gains two nullable fields:
  - `requiredColumns` — set by `ProjectionPruning`; the storage layer can read
    only these columns.
  - `scanLimit` — set by `LimitPushdown`; the storage layer can stop reading
    after this many rows (only valid when there is no ORDER BY above).

## Building & testing

```bash
# Build the sql-planner JAR first (dependency)
cd ../sql-planner && gradle jar

# Run tests + coverage check
cd ../sql-optimizer
gradle test           # runs 45+ tests; JaCoCo ≥80% line coverage required
```

## Requirements

- Java 21 (sealed interfaces + records + pattern matching)
- `coding-adventures-sql-planner` JAR on the classpath (built locally via Gradle)
