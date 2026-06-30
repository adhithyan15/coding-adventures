# CodingAdventures.SqlPlanner.FSharp

F# logical query planner for the Mini-SQLite Level 1 pipeline.

## What it does

Transforms a parsed `Statement` into a tree of `LogicalPlan` nodes using an
8-step bottom-up pipeline for SELECT:

```
Scan → Filter → Aggregate → Having → Project → Distinct → Sort → Limit
```

No I/O, no execution. Pure data transformation: statement in, plan tree out.

## How it fits in the stack

```
sql-parser  →  sql-planner  →  sql-optimizer  →  sql-codegen  →  sql-vm
              (this pkg)
```

The F# `sql-parser` package is currently a stub, so callers build `Statement`
values directly (or wrap their own parser output).

## Usage

```fsharp
open CodingAdventures.SqlPlanner.FSharp

let schema =
    InMemorySchemaProvider(Map.ofList [ "users", ["id"; "name"; "age"] ])

let stmt =
    Statement.Select
        { Distinct = false
          Columns  = [ OutputColumn.Expr(Expr.Column(None, "name"), None) ]
          From     = [ "users", None ]
          Joins    = []
          Where    = Some (Expr.BinaryOp(Gt, Expr.Column(None, "age"), Expr.Literal(SqlValue.Integer 18L)))
          GroupBy  = []
          Having   = None
          OrderBy  = []
          Limit    = None }

match Planner.plan schema stmt with
| Ok plan  -> printfn "%A" plan
| Error e  -> printfn "Planning error: %A" e
```

## Key types

| Type | Description |
|------|-------------|
| `SqlValue` | Literal values: Null, Integer, Real, Text, Bool |
| `Expr` | Scalar expressions: Column, BinaryOp, AggExpr, Like, … |
| `Statement` | SQL statements: Select, Insert, Update, Delete, DDL |
| `LogicalPlan` | Plan nodes: Scan, Filter, Project, Join, Aggregate, … |
| `PlanError` | Planning errors: UnknownTable, AmbiguousColumn, … |
| `ISchemaProvider` | Interface for column-name lookup |

## Running tests

```
dotnet test tests/CodingAdventures.SqlPlanner.Tests/ --disable-build-servers
```
