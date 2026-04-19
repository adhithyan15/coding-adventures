# mini-sqlite (Python)

A PEP 249 DB-API 2.0 compatible facade over the SQL pipeline:
`sql-lexer` → `sql-parser` → adapter → `sql-planner` → `sql-optimizer` →
`sql-codegen` → `sql-vm` → `sql-backend`.

`mini-sqlite` is the user-facing package. It owns connection state,
parameter binding, transaction lifecycle, error translation, and the
cursor API — but implements none of the actual SQL processing. That
work is delegated to the packages below it.

## Quick start

```python
import mini_sqlite

conn = mini_sqlite.connect(":memory:")
cur = conn.cursor()

cur.execute("""
    CREATE TABLE employees (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        dept TEXT,
        salary INTEGER
    )
""")

cur.executemany(
    "INSERT INTO employees (id, name, dept, salary) VALUES (?, ?, ?, ?)",
    [
        (1, "Alice", "eng",   90000),
        (2, "Bob",   "eng",   75000),
        (3, "Carol", "sales", 65000),
    ]
)

cur.execute("SELECT name, salary FROM employees WHERE dept = ?", ("eng",))
for row in cur:
    print(row)
```

## Module surface

```python
mini_sqlite.apilevel      # "2.0"
mini_sqlite.threadsafety  # 1
mini_sqlite.paramstyle    # "qmark"

mini_sqlite.connect(database: str, *, autocommit: bool = False) -> Connection
```

`database`:

- `":memory:"` — uses the in-memory backend (no persistence).
- any other string — a filesystem path (future FileBackend).

## Exception hierarchy

PEP 249 standard tree: `Warning`, `Error`, `InterfaceError`,
`DatabaseError`, `DataError`, `OperationalError`, `IntegrityError`,
`InternalError`, `ProgrammingError`, `NotSupportedError`. All pipeline
errors are translated into the appropriate PEP 249 class before being
raised to the caller.

## SQL supported in v1

From `code/specs/mini-sqlite-python.md`:

- `SELECT` with `FROM`, `WHERE`, `GROUP BY`, `HAVING`, `ORDER BY`,
  `LIMIT`, `OFFSET`, `DISTINCT`, `INNER`/`CROSS` joins.
- `INSERT INTO ... VALUES`, `UPDATE`, `DELETE`.
- `CREATE TABLE [IF NOT EXISTS]`, `DROP TABLE [IF EXISTS]`.
- `?` parameter placeholders (qmark style).

Not yet implemented in v1: LEFT/RIGHT/FULL joins, subqueries,
`UNION`, CTEs, `CASE`, window functions, indexes, triggers, views,
`PRAGMA`, foreign keys, BLOB, and the file-backed persistence.
