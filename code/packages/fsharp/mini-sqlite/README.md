# CodingAdventures.MiniSqlite.FSharp

Level 1 F# port of the mini-sqlite facade.  SQL is routed through the full
five-stage pipeline:

```
SQL text
  → SqlParser  (tokenise + parse into AST)
  → SqlPlanner (resolve columns, annotate types)
  → SqlOptimizer (constant folding, predicate pushdown)
  → SqlCodegen  (emit bytecode instructions)
  → SqlVm       (execute against InMemoryBackend)
```

The public API is DB-API-inspired, identical to Level 0:

```fsharp
use conn = MiniSqlite.Connect(":memory:")

// DDL
conn.Execute("CREATE TABLE t (id INTEGER, name TEXT)") |> ignore

// DML with qmark binding
conn.Execute("INSERT INTO t VALUES (?, ?)", [| box 1L; box "Alice" |]) |> ignore

// Queries
let cur = conn.Execute("SELECT id, name FROM t WHERE id = ?", [| box 1L |])
for row in cur.FetchAll() do
    printfn "%A" (row |> Seq.toList)

// Batch insert
conn.ExecuteMany("INSERT INTO t VALUES (?, ?)",
    [ [| box 2L; box "Bob" |]; [| box 3L; box "Carol" |] ]) |> ignore

// Transactions
conn.Commit()
conn.Rollback()
```

## Supported SQL

- `CREATE TABLE` / `DROP TABLE`
- `INSERT INTO … VALUES (…)` with `?` parameter binding
- `UPDATE … SET … WHERE …`
- `DELETE FROM … WHERE …`
- `SELECT` with:
  - column projection and `AS` aliases
  - `*` wildcard
  - scalar expressions: arithmetic, comparisons, `LIKE`, `IS NULL`, `IN`, `BETWEEN`
  - scalar functions: `LOWER`, `UPPER`, `LENGTH`, `SUBSTR`, `TRIM`, `ABS`, `ROUND`
  - `WHERE` clause with full expression support including NULLs
  - `ORDER BY` (ASC/DESC, multi-column; NULLs sort first in ASC, last in DESC)
  - `LIMIT` / `OFFSET`
  - `DISTINCT`
  - aggregate functions: `COUNT(*)`, `COUNT(expr)`, `SUM`, `AVG`, `MIN`, `MAX`
  - `COUNT(DISTINCT expr)` and `SUM(DISTINCT expr)` etc.
  - `GROUP BY` (multi-column)
  - `HAVING` with aggregate predicates

## Metadata

```fsharp
MiniSqlite.ApiLevel    // "1"
MiniSqlite.ThreadSafety // "Serialized"
MiniSqlite.ParamStyle  // "qmark"
```

## Limitations (deferred to later levels)

- File-backed databases (`:memory:` only at Level 1)
- JOINs
- Subqueries
- Indexes
- Full SQLite type affinity rules
