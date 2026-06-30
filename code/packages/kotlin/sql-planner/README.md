# coding-adventures-sql-planner (Kotlin)

Kotlin logical query planner for the Mini-SQLite Level 1 pipeline.

## What it does

Transforms a `Statement` into a tree of `LogicalPlan` nodes using an
8-step bottom-up pipeline for SELECT:

```
Scan → Filter → Aggregate → Having → Project → Distinct → Sort → Limit
```

No I/O, no database connections — pure in-memory data transformation.
Errors are reported as `PlanException` subclasses (consistent with the Kotlin
`sql-backend` style).

## How it fits in the stack

```
sql-parser  →  sql-planner  →  sql-optimizer  →  sql-codegen  →  sql-vm
               (this pkg)
```

## Usage

```kotlin
val schema = InMemorySchemaProvider(mapOf(
    "users" to listOf("id", "name", "age", "email")
))

val planner = SqlPlanner(schema)

val stmt = Statement.Select(
    distinct = false,
    columns  = listOf(OutputColumn.Star),
    from     = listOf(TableRef("users", null)),
    joins    = emptyList(),
    where    = SqlExpr.BinaryOp(BinaryOperator.GT, SqlExpr.Column(null, "age"), SqlExpr.Literal(18L)),
    groupBy  = emptyList(),
    having   = null,
    orderBy  = emptyList(),
    limit    = null)

val plan: LogicalPlan = planner.plan(stmt)   // throws PlanException on error
```

## Key types

| Type | Description |
|------|-------------|
| `SqlExpr` | Sealed class for scalar expressions (14 subclasses) |
| `LogicalPlan` | Sealed class for plan nodes (15 subclasses) |
| `Statement` | Sealed class for SQL statements (6 subclasses) |
| `PlanException` | Base class for planning errors |
| `SchemaProvider` | Interface for column-name lookup |
| `InMemorySchemaProvider` | Map-backed schema provider |

## Running tests

```
gradle test
```

Coverage threshold: 80% line coverage (enforced by JaCoCo).
