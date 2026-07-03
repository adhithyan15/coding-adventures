# sql-planner (Haskell)

Haskell logical query planner for the Mini-SQLite Level 1 pipeline.

## What it does

Transforms a `Statement` into a tree of `LogicalPlan` nodes using an
8-step bottom-up pipeline for SELECT:

```
Scan → Filter → Aggregate → Having → Project → Distinct → Sort → Limit
```

No I/O, no database connections — pure functional data transformation.
Errors are reported via `Either PlanError LogicalPlan`.

## How it fits in the stack

```
sql-parser  →  sql-planner  →  sql-optimizer  →  sql-codegen  →  sql-vm
               (this pkg)
```

## Usage

```haskell
import SqlPlanner

let schema = inMemorySchema [("users", ["id", "name", "age", "email"])]

let stmt = SelectStmt
        { stmtDistinct = False
        , stmtColumns  = [OutputStar]
        , stmtFrom     = [TableRef "users" Nothing]
        , stmtJoins    = []
        , stmtWhere    = Just (BinaryOp BinGt (Column Nothing "age") (Literal (Just (LitInt 18))))
        , stmtGroupBy  = []
        , stmtHaving   = Nothing
        , stmtOrderBy  = []
        , stmtLimit    = Nothing
        }

case plan schema stmt of
    Left err  -> print err
    Right lp  -> print lp
```

## Key types

| Type | Description |
|------|-------------|
| `SqlExpr` | Algebraic type for scalar expressions (14 variants) |
| `LogicalPlan` | Algebraic type for plan nodes (15 variants) |
| `Statement` | Algebraic type for SQL statements (6 variants) |
| `PlanError` | Sum type for planning errors (5 variants) |
| `SchemaProvider` | Newtype wrapping a column-lookup function |
| `inMemorySchema` | Build a schema from an association list |

## Running tests

```
cabal test all
```
