# coding-adventures-sql-planner (Java)

Java 21 logical query planner for the Mini-SQLite Level 1 pipeline.

## What it does

Transforms a `Statement` into a tree of `LogicalPlan` nodes using an
8-step bottom-up pipeline for SELECT:

```
Scan → Filter → Aggregate → Having → Project → Distinct → Sort → Limit
```

No I/O, no database connections — pure in-memory data transformation.
Errors are reported as `PlanException` subclasses (consistent with the Java
`sql-backend` style).

## How it fits in the stack

```
sql-parser  →  sql-planner  →  sql-optimizer  →  sql-codegen  →  sql-vm
               (this pkg)
```

The Java `sql-parser` package is currently a stub, so callers build
`Statement` objects directly (or wrap their own parser output).

## Usage

```java
import com.codingadventures.sqlplanner.SqlPlanner;
import com.codingadventures.sqlplanner.SqlPlanner.*;

var schema = new InMemorySchemaProvider(Map.of(
    "users", List.of("id", "name", "age", "email")
));

var planner = new SqlPlanner(schema);

var stmt = new Statement.Select(
    false,
    List.of(new OutputColumn.Star()),
    List.of(new TableRef("users", null)),
    List.of(),
    new SqlExpr.BinaryOp(BinaryOperator.GT,
        new SqlExpr.Column(null, "age"),
        new SqlExpr.Literal(18L)),
    List.of(), null, List.of(), null);

LogicalPlan plan = planner.plan(stmt);  // throws PlanException on error
```

## Key types

| Type | Description |
|------|-------------|
| `SqlExpr` | Sealed interface for scalar expressions (14 variants) |
| `LogicalPlan` | Sealed interface for plan nodes (15 variants) |
| `Statement` | Sealed interface for SQL statements (6 variants) |
| `PlanException` | Base class for planning errors |
| `SchemaProvider` | Interface for column-name lookup |
| `InMemorySchemaProvider` | Map-backed schema provider |

All variants are Java records implementing their sealed interface, giving
pattern-matching exhaustiveness at compile time.

## Running tests

```
gradle test
```

Coverage threshold: 80% line coverage (enforced by JaCoCo).
