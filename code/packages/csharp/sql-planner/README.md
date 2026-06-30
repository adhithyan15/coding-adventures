# CodingAdventures.SqlPlanner.CSharp

C# logical query planner for the Mini-SQLite Level 1 pipeline.

## What it does

Transforms a `Statement` record into a tree of `LogicalPlan` nodes using an
8-step bottom-up pipeline for SELECT:

```
Scan → Filter → Aggregate → Having → Project → Distinct → Sort → Limit
```

No I/O, no database access — pure in-memory data transformation. Errors are
reported as `PlanException` subclasses (consistent with the C# `sql-backend`).

## How it fits in the stack

```
sql-parser  →  sql-planner  →  sql-optimizer  →  sql-codegen  →  sql-vm
               (this pkg)
```

The C# `sql-parser` package is currently a stub, so callers build `Statement`
values directly (or wrap their own parser output).

## Usage

```csharp
using CodingAdventures.SqlPlanner;

var schema = new InMemorySchemaProvider(new Dictionary<string, IReadOnlyList<string>>
{
    ["users"] = new[] { "id", "name", "age" },
});

var planner = new SqlPlanner(schema);

var stmt = new SelectStatement(
    Distinct: false,
    Columns:  new[] { new OutputColumn.Expr(new SqlExpr.Column(null, "name"), null) },
    From:     new[] { ("users", (string?)null) },
    Joins:    Array.Empty<JoinClause>(),
    Where:    new SqlExpr.BinaryOp(BinaryOperator.Gt, new SqlExpr.Column(null, "age"), new SqlExpr.Literal(18L)),
    GroupBy:  Array.Empty<SqlExpr>(),
    Having:   null,
    OrderBy:  Array.Empty<SortKey>(),
    Limit:    null);

var plan = planner.Plan(stmt);  // throws PlanException on error
```

## Key types

| Type | Description |
|------|-------------|
| `SqlExpr` | Abstract record base for scalar expressions |
| `LogicalPlan` | Abstract record base for plan nodes |
| `Statement` | Abstract record base for SQL statements |
| `PlanException` | Base class for planning errors |
| `ISchemaProvider` | Interface for column-name lookup |
| `InMemorySchemaProvider` | Dictionary-backed schema provider |

## Running tests

```
dotnet test tests/CodingAdventures.SqlPlanner.Tests/ --disable-build-servers
```
