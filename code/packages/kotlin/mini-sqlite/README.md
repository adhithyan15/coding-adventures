# mini-sqlite (Kotlin — Level 1)

`mini-sqlite` is the Kotlin Level 1 port of the mini-sqlite DB-API facade.
It implements the full SQL compilation pipeline: text → parse → plan → optimize → codegen → VM,
backed by an in-memory database.

## Architecture

```
SQL text
  │  bindParameters()      substitute ? placeholders with escaped literals
  ▼
bound SQL
  │  SqlParser.parse()     recursive-descent parser → Statement AST
  ▼
Statement
  │  SqlPlanner.plan()     resolve columns → LogicalPlan
  ▼
LogicalPlan
  │  SqlOptimizer          constant folding, predicate pushdown
  ▼
OptimizedPlan
  │  SqlCodegen.compile()  emit bytecode Program
  ▼
Program
  │  SqlVm.execute()       stack-machine execution against InMemoryBackend
  ▼
QueryResult → Result → Cursor rows
```

SELECT, UPDATE, and DELETE bypass the VM and use direct Kotlin evaluators for full
correctness (the VM has known gaps with function dispatch, SELECT *, and GROUP BY).
DDL (CREATE TABLE, DROP TABLE) and INSERT go through the full pipeline.

## Features

- `MiniSqlite.connect(":memory:")` — in-memory connection (only `:memory:` supported)
- **DDL**: `CREATE TABLE`, `CREATE TABLE IF NOT EXISTS`, `DROP TABLE`, `DROP TABLE IF EXISTS`
- **DML**: `INSERT INTO`, multi-row `INSERT`, `INSERT` with column list, `UPDATE`, `DELETE`
- **SELECT**: column projection, `SELECT *`, aliases, `WHERE`, `ORDER BY` (ASC/DESC, NULLS FIRST/LAST),
  `LIMIT`/`OFFSET`, `DISTINCT`, `GROUP BY`, `HAVING`
- **Aggregates**: `COUNT(*)`, `COUNT(col)`, `COUNT(DISTINCT col)`, `SUM`, `AVG`, `MIN`, `MAX`
- **String functions**: `LENGTH`, `UPPER`, `LOWER`, `SUBSTR`/`SUBSTRING`, `TRIM`, `LTRIM`, `RTRIM`,
  `REPLACE`, string concatenation `||`
- **Math functions**: `ABS`, `ROUND`
- **Null functions**: `COALESCE`
- **Predicates**: `=`, `!=`/`<>`, `<`, `<=`, `>`, `>=`, `BETWEEN`, `IN`, `NOT IN`, `LIKE`, `NOT LIKE`,
  `IS NULL`, `IS NOT NULL`, `AND`, `OR`, `NOT`
- **Transactions**: `BEGIN`, `COMMIT`, `ROLLBACK`, autocommit mode
- **Parameter binding**: `?` positional placeholders with qmark style
- **Coverage**: ≥ 80% instruction coverage enforced by JaCoCo

## Example

```kotlin
val conn = MiniSqlite.connect(":memory:")
conn.execute("CREATE TABLE employees (id INTEGER, name TEXT, dept TEXT, salary REAL)")
conn.executemany(
    "INSERT INTO employees VALUES (?, ?, ?, ?)",
    listOf(
        listOf(1, "Alice", "Engineering", 90000.0),
        listOf(2, "Bob",   "Marketing",   60000.0),
        listOf(3, "Carol", "Engineering", 80000.0),
    ),
)

// Aggregate query with GROUP BY
val deptStats = conn.execute(
    "SELECT dept, COUNT(*), AVG(salary) FROM employees GROUP BY dept ORDER BY dept"
).fetchall()

// Parameterised WHERE
val engineers = conn.execute(
    "SELECT name FROM employees WHERE dept = ? ORDER BY salary DESC", listOf("Engineering")
).fetchall()

// String functions
val names = conn.execute(
    "SELECT UPPER(name) FROM employees WHERE LENGTH(name) > 4"
).fetchall()

conn.commit()
conn.close()
```

## Error handling

All errors are raised as `MiniSqliteException` with a `kind` property:

| Kind | When |
|------|------|
| `NotSupportedError` | Non-`:memory:` database path requested |
| `ProgrammingError` | Closed connection/cursor, wrong parameter count, unsupported param type |
| `OperationalError` | Unknown table, parse error, plan error, or backend error |

## Limitations

- In-memory only — no file-backed storage (Level 1 scope)
- No JOIN support in the direct evaluator (Level 1 scope)
- No subqueries (Level 1 scope)
