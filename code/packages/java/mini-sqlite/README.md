# mini-sqlite

`mini-sqlite` provides two implementations of an in-memory SQL facade for Java:

| Level | Class | Engine |
|-------|-------|--------|
| 0     | `MiniSqlite`           | Hand-rolled parser and executor (educational) |
| 1     | `MiniSqliteConnection` | Full five-package pipeline (production quality) |

## Level 1 — `MiniSqliteConnection`

Level 1 wires the complete SQL pipeline:

```
SQL text
  │
  ▼  SqlTextParser.parse()      [this package]
SqlPlanner.Statement
  │
  ▼  SqlPlanner.plan()          [sql-planner]
SqlPlanner.LogicalPlan
  │
  ▼  SqlOptimizer.optimize()    [sql-optimizer]
SqlOptimizer.OptimizedPlan
  │
  ▼  SqlCodegen.compile()       [sql-codegen]
SqlCodegen.Program
  │
  ▼  SqlVm.execute()            [sql-vm]
SqlVm.QueryResult
```

### Usage

```java
var conn = MiniSqliteConnection.connect(":memory:");
conn.execute("CREATE TABLE users (id INTEGER, name TEXT, age INTEGER)");
conn.execute("INSERT INTO users VALUES (?, ?, ?)", List.of(1, "Alice", 30));
conn.execute("INSERT INTO users VALUES (?, ?, ?)", List.of(2, "Bob", 25));

// SELECT with ORDER BY
var cur = conn.execute("SELECT name, age FROM users ORDER BY age");
List<List<Object>> rows = cur.fetchall();
// [["Bob", 25L], ["Alice", 30L]]

// Aggregate + HAVING
cur = conn.execute(
    "SELECT name, COUNT(*) AS n FROM users GROUP BY name HAVING COUNT(*) > 0");
```

### Features

- `SELECT` with column projection, `AS` aliases, `WHERE`, `GROUP BY`, `HAVING`,
  `ORDER BY` (multi-column, `ASC`/`DESC`, `NULLS FIRST`/`LAST`), `LIMIT`, `OFFSET`,
  `DISTINCT`, `JOIN` (`INNER`, `LEFT`, `RIGHT`, `CROSS`)
- `INSERT INTO table [(cols)] VALUES (...)` with and without explicit column list
- `UPDATE table SET col = expr WHERE ...`
- `DELETE FROM table WHERE ...`
- `CREATE TABLE`, `DROP TABLE`
- Aggregate functions: `COUNT(*)`, `COUNT(expr)`, `SUM`, `AVG`, `MIN`, `MAX`
- Scalar functions: `UPPER`, `LOWER`, `LENGTH`, `TRIM`, `ABS`, `COALESCE`, `||` concat
- `BETWEEN`, `IN (...)`, `LIKE` predicates
- `IS NULL` / `IS NOT NULL`
- Qmark (`?`) parameter binding
- Transaction control: `commit()`, `rollback()`, `BEGIN`, `COMMIT`, `ROLLBACK`
- Autocommit mode via `Options`

### Error handling

All pipeline errors are mapped to `MiniSqliteException` with a DB-API 2.0 kind:
`ProgrammingError`, `OperationalError`, `InterfaceError`, `NotSupportedError`.

### DB-API 2.0 constants

```java
MiniSqliteConnection.API_LEVEL   // "2.0"
MiniSqliteConnection.THREADSAFETY // 1
MiniSqliteConnection.PARAMSTYLE  // "qmark"
```

---

## Level 0 — `MiniSqlite`

`MiniSqlite` is an educational implementation that hand-rolls parsing and execution
inline. It supports a subset of SQL sufficient for simple CRUD operations and is kept
as a learning reference.

```java
var conn = MiniSqlite.connect(":memory:");
conn.execute("CREATE TABLE users (id INTEGER, name TEXT)");
conn.execute("INSERT INTO users VALUES (?, ?)", List.of(1, "Alice"));
var cursor = conn.execute("SELECT name FROM users");
var rows = cursor.fetchall();
```

File-backed databases are out of scope; opening anything other than `:memory:` raises
a `NotSupportedError` in both levels.
