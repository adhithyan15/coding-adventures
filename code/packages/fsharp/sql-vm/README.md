# CodingAdventures.SqlVm.FSharp

F# stack-machine bytecode interpreter for the Mini-SQLite Level 1 pipeline.

## Pipeline position

```
sql-lexer → sql-parser → sql-planner → sql-optimizer → sql-codegen
         → [sql-vm] → mini-sqlite
```

This package sits at the end of the compilation pipeline. It receives a
`Program` (a flat list of `Instruction` values) produced by `sql-codegen`,
interprets each instruction against a `Backend`, and returns a `QueryResult`.

## What is a stack machine?

A stack machine evaluates expressions using a single operand stack rather than
named registers. Each instruction pops zero, one, or two values from the stack,
does some work, and pushes the result back. SQL expressions compile to straight-
line sequences of these instructions:

```
-- SELECT 3 + 4
LoadConst(Integer 3)   stack: [3]
LoadConst(Integer 4)   stack: [3, 4]
BinaryOpInstr(Add)     stack: [7]
EmitColumn("3 + 4")    stack: []
```

## API

```fsharp
open CodingAdventures.SqlVm.FSharp

type QueryResult =
    { Columns: string list
      Rows: SqlValue list list
      RowsAffected: int }

module SqlVm =
    val execute : Program -> Backend -> QueryResult
```

`execute` is the single public entry point. Pass it the output of
`SqlCodegen.compile` and a `Backend` instance:

```fsharp
let backend = InMemoryBackend()
backend.CreateTable("users", [| ColumnDef("id", "INTEGER"); ColumnDef("name", "TEXT") |], false)

let plan    = SqlOptimizer.optimize (SqlPlanner.plan schema stmt)
let program = SqlCodegen.compile plan
let result  = SqlVm.execute program backend

// result.Columns = ["id"; "name"]
// result.Rows    = [[Integer 1L; Text "Alice"]; ...]
```

## Design

### Execution model

1. Pre-scan: build a `Map<string,int>` mapping label names to instruction indices.
   This makes every `Jump` O(1).
2. Main loop: increment `pc`, dispatch on the instruction case.
3. Cursors are open `IRowIterator` / `ICursor` values stored in a
   `Dictionary<string option, CursorState>`. Aliases from the codegen (`None`
   for single-table, `Some "u"` for aliased) distinguish multiple open cursors
   in a JOIN.
4. After the scan loop, post-processing instructions (`SortResult`,
   `DistinctResult`, `LimitResult`) operate on the in-memory result buffer.

### Three-valued logic

SQL has three truth values: TRUE, FALSE, and NULL (unknown).

| Expression       | Result |
|-----------------|--------|
| NULL = NULL      | NULL   |
| NULL IS NULL     | TRUE   |
| FALSE AND NULL   | FALSE  |
| TRUE AND NULL    | NULL   |
| TRUE OR NULL     | TRUE   |
| FALSE OR NULL    | NULL   |

### Aggregates

Aggregate accumulators live in a mutable `AggSlot[]` array:

| Function   | Accumulates              | Final value                |
|-----------|--------------------------|----------------------------|
| COUNT(*)   | increments on every row  | count                      |
| COUNT(col) | increments on non-NULL   | count                      |
| SUM(col)   | adds non-NULL values     | NULL if all-NULL, else sum |
| AVG(col)   | sum + count              | NULL if empty, else sum/n  |
| MIN(col)   | tracks smallest          | NULL if empty              |
| MAX(col)   | tracks largest           | NULL if empty              |

### LIKE matching

SQL `%` maps to `.*` in .NET Regex; `_` maps to `.`. Other characters are
escaped with `Regex.Escape`. The pattern is anchored with `^…$`.

## Dependencies

- `CodingAdventures.SqlBackend.FSharp` — `Backend`, `InMemoryBackend`, `Row`, `ICursor`
- `CodingAdventures.SqlCodegen.FSharp` — `Program`, `Instruction`, `BinaryOp`, `AggFn`

(sql-planner and sql-optimizer are transitive via sql-codegen.)

## Testing

```
dotnet test tests/CodingAdventures.SqlVm.FSharp.Tests.fsproj \
  --disable-build-servers \
  /p:CollectCoverage=true \
  "/p:Include=[CodingAdventures.SqlVm.FSharp]*" \
  /p:Threshold=80 /p:ThresholdType=line
```

56 xUnit tests cover all instruction categories: constants, arithmetic,
comparisons, three-valued logic, NULL predicates, BETWEEN, LIKE, IN, scan loops,
row construction, aggregates, control flow, DDL, DML, transactions, and
post-processing.
