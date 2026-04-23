# Mini-SQLite (Python) Specification

## Overview

This document specifies a **full end-to-end SQL engine** in Python, built by wiring
together the seven SQL pipeline packages into a single embedded database with a
PEP 249 DB-API 2.0 compatible public API.

The name **mini-sqlite** follows the repo's naming convention (`mini-redis`,
`dartmouth-basic`) — it is not wire-format compatible with SQLite, but it is
*behaviorally* similar: embedded, single-process, serverless, supports the core
SQL subset defined in `sql.md`, and follows Python's standard database API so
applications that use `sqlite3` can use `mini_sqlite` as a drop-in replacement.

Think of this as the **reference integration** that proves all seven pipeline
packages actually compose. Once this works in Python, the same wiring replicates
across all 17 languages.

---

## Why Python First

Python is the ideal pilot language for three reasons:

1. **Existing packages are further along.** `sql-lexer` and `sql-parser` already
   exist and are well-tested in Python. We need to build five new Python packages
   (planner, optimizer, codegen, vm, backend), not seven.

2. **PEP 249 is a strong forcing function.** Every Python database driver follows
   the same `connect()` / `cursor()` / `execute()` / `fetchall()` API. Targeting
   PEP 249 makes the integration tests identical to the tests for Python's
   built-in `sqlite3` — we can run the same assertions against both and compare.

3. **Low ceremony.** Python needs no build step, no compilation, no dependency
   resolution beyond `pip`. Iterating on seven packages simultaneously is faster
   here than in Rust or Go.

---

## Architecture

```
┌───────────────────────────────────────────────────────────────────────┐
│                    Application Code                                    │
│   import mini_sqlite                                                   │
│   conn = mini_sqlite.connect(":memory:")                               │
│   cur = conn.cursor()                                                  │
│   cur.execute("SELECT name FROM users WHERE age > ?", (18,))           │
│   rows = cur.fetchall()                                                │
└───────────────────────────────┬───────────────────────────────────────┘
                                │
                                ▼
┌───────────────────────────────────────────────────────────────────────┐
│              mini-sqlite (facade package)                              │
│                                                                        │
│  Connection, Cursor, paramstyle, exception hierarchy                   │
│  Parameter binding ("?" or ":name" → literal substitution)             │
│  Transaction lifecycle (autocommit / explicit BEGIN/COMMIT/ROLLBACK)   │
└───────────────────────────────┬───────────────────────────────────────┘
                                │  sql string + bindings
                                ▼
┌───────────────────────────────────────────────────────────────────────┐
│                        Pipeline                                        │
│                                                                        │
│  sql-lexer      str              → [Token]                             │
│  sql-parser     [Token]          → ASTNode                             │
│  sql-planner    ASTNode          → LogicalPlan                         │
│  sql-optimizer  LogicalPlan      → LogicalPlan                         │
│  sql-codegen    LogicalPlan      → Program                             │
│  sql-vm         Program, Backend → QueryResult                         │
└───────────────────────────────┬───────────────────────────────────────┘
                                │  read/write calls
                                ▼
┌───────────────────────────────────────────────────────────────────────┐
│                  sql-backend                                           │
│                                                                        │
│  InMemoryBackend   — for ":memory:" connections                        │
│  FileBackend       — for file-path connections (JSON-per-table)        │
└───────────────────────────────────────────────────────────────────────┘
```

The facade is intentionally thin: it owns connection state, parameter binding,
and error translation — nothing else. All real work happens in the six underlying
pipeline packages.

---

## Package Layout

```
code/packages/python/
├── sql-lexer/              (exists)
├── sql-parser/             (exists)
├── sql-planner/            (new — per sql-planner.md)
├── sql-optimizer/          (new — per sql-optimizer.md)
├── sql-codegen/            (new — per sql-codegen.md)
├── sql-vm/                 (new — per sql-vm.md)
├── sql-backend/            (new — per sql-backend.md, includes InMemoryBackend)
└── mini-sqlite/            (new — this spec)
    ├── pyproject.toml
    ├── README.md
    ├── CHANGELOG.md
    ├── BUILD
    └── src/
        └── mini_sqlite/
            ├── __init__.py
            ├── connection.py
            ├── cursor.py
            ├── binding.py
            ├── errors.py
            ├── file_backend.py     (persistent backend)
            └── py.typed
```

Every package follows the existing Python package conventions in the repo: `src/`
layout, `pyproject.toml` with dependencies declared as `file:../<sibling>`, a
BUILD file, a README.md, a CHANGELOG.md, and `py.typed` for type checker support.

---

## Dependency Graph

```
mini-sqlite
    ├── sql-lexer
    ├── sql-parser      → sql-lexer
    ├── sql-planner     → sql-parser
    ├── sql-optimizer   → sql-planner
    ├── sql-codegen     → sql-optimizer (and therefore sql-planner)
    ├── sql-vm          → sql-codegen, sql-backend
    └── sql-backend     (leaf, no SQL-pipeline deps)
```

BUILD files must install dependencies in leaf-to-root order (per repo convention
and lessons.md). The `mini-sqlite` BUILD file chain-installs all seven packages
plus any transitive deps (`lexer`, `lang-parser`, `grammar-tools`, etc.).

---

## Public API: PEP 249 DB-API 2.0

`mini_sqlite` implements the minimum PEP 249 surface. This matches Python's
`sqlite3` stdlib so application code is portable between them.

### Module-level

```python
import mini_sqlite

# Required module globals
mini_sqlite.apilevel     # "2.0"
mini_sqlite.threadsafety # 1 (threads may share module, not connections)
mini_sqlite.paramstyle   # "qmark" (the "?" style)

# Factory
connect(database: str, *, autocommit: bool = False) -> Connection
```

`database` argument:
- `":memory:"` — uses `InMemoryBackend` (no disk persistence)
- any other string — a filesystem path; uses `FileBackend` (JSON-per-table)

### Connection

```python
class Connection:
    def cursor(self) -> Cursor: ...
    def commit(self) -> None: ...
    def rollback(self) -> None: ...
    def close(self) -> None: ...

    # Python convenience (also in sqlite3)
    def execute(self, sql: str, params: tuple | dict = ()) -> Cursor: ...
    def executemany(self, sql: str, seq: list[tuple | dict]) -> Cursor: ...

    # Context manager support
    def __enter__(self) -> Connection: ...
    def __exit__(self, exc_type, exc, tb) -> None: ...
        # if exc is None: self.commit() else: self.rollback()
```

### Cursor

```python
class Cursor:
    # Read-only attributes
    description: list[tuple] | None    # (name, type_code, None, None, None, None, None) per column
    rowcount: int                       # rows affected; -1 if unknown / SELECT
    lastrowid: int | None               # last INSERT rowid; None otherwise
    arraysize: int                      # default fetchmany() batch size (default 1)

    def execute(self, sql: str, params: tuple | dict = ()) -> Cursor: ...
    def executemany(self, sql: str, seq: list[tuple | dict]) -> Cursor: ...
    def fetchone(self) -> tuple | None: ...
    def fetchmany(self, size: int = -1) -> list[tuple]: ...
    def fetchall(self) -> list[tuple]: ...
    def close(self) -> None: ...

    # Iterator protocol
    def __iter__(self) -> Iterator[tuple]: ...
    def __next__(self) -> tuple: ...
```

### Exception Hierarchy (PEP 249 standard)

```
Exception
 └── Warning
 └── Error
      ├── InterfaceError           — API misuse
      └── DatabaseError
           ├── DataError           — type conversion failure
           ├── OperationalError    — runtime failure (table not found, etc.)
           ├── IntegrityError      — constraint violation
           ├── InternalError       — backend internal error
           ├── ProgrammingError    — SQL syntax error, wrong param count
           └── NotSupportedError   — feature not implemented
```

Errors from the pipeline are translated:

| Pipeline error                    | Raised as                    |
|-----------------------------------|------------------------------|
| `LexerError`, `ParseError`        | `ProgrammingError`           |
| `PlanError::AmbiguousColumn`      | `ProgrammingError`           |
| `PlanError::UnknownTable`         | `OperationalError`           |
| `PlanError::UnknownColumn`        | `OperationalError`           |
| `VmError::TableNotFound`          | `OperationalError`           |
| `VmError::ColumnNotFound`         | `OperationalError`           |
| `VmError::TypeMismatch`           | `DataError`                  |
| `VmError::DivisionByZero`         | `OperationalError`           |
| `VmError::ConstraintViolation`    | `IntegrityError`             |
| `VmError::TableAlreadyExists`     | `OperationalError`           |
| `BackendError::Unsupported`       | `NotSupportedError`          |
| `BackendError::Internal`          | `InternalError`              |
| Any unexpected exception          | `InternalError`              |

---

## Parameter Binding

`paramstyle = "qmark"` means `?` in SQL is replaced by positional parameters.

```python
cursor.execute("SELECT * FROM users WHERE age > ? AND dept = ?", (18, 'eng'))
```

### Rules

1. **Count mismatch raises `ProgrammingError`.** If the SQL has 3 `?` and you pass
   a 2-tuple, that's an error *before* any parsing happens (the lexer counts `?`
   tokens).

2. **Positional only.** `paramstyle = "qmark"` means we do not support `:name`
   (would be `paramstyle = "named"`) in v1. Named parameters are a follow-up.

3. **Binding is literal substitution.** Given `WHERE age > ?` and `(18,)`, the
   binding layer constructs a `LogicalPlan` where the predicate expression is
   `BinaryOp(Gt, Column("age"), Literal(Int(18)))`. It does **not** do string-level
   `?` replacement in the SQL text — the `?` is a parser token, and the literal
   is inserted at the appropriate AST position.

4. **Type coercion on input:**
   - `int` → `SqlValue::Int`
   - `float` → `SqlValue::Float`
   - `str` → `SqlValue::Text`
   - `bool` → `SqlValue::Bool`
   - `None` → `SqlValue::Null`
   - `bytes` → `ProgrammingError` in v1 (no BLOB support; follow-up)
   - Anything else → `ProgrammingError`

5. **Type coercion on output (fetchone/fetchall):**
   - `SqlValue::Int` → `int`
   - `SqlValue::Float` → `float`
   - `SqlValue::Text` → `str`
   - `SqlValue::Bool` → `int` (0 or 1, matching sqlite3's behavior)
   - `SqlValue::Null` → `None`

### Binding in the AST

The grammar already accepts `?` as a literal token. The parser emits a
`parameter_placeholder` AST node. The **binding layer** (part of `mini_sqlite`, not
the planner) walks the AST pre-planning, replaces each placeholder with the
corresponding user-supplied literal AST node, and counts them to validate arity.

---

## Transaction Lifecycle

`mini_sqlite` follows `sqlite3`-style transactional behavior:

- **Default mode:** `autocommit = False`. The first DML/DDL statement implicitly
  opens a transaction via `backend.begin_transaction()`. Subsequent statements
  run in that transaction. `conn.commit()` or `conn.rollback()` closes it.
- **Autocommit mode:** `autocommit = True`. Every statement is its own transaction.
- **Context manager:** `with conn: ...` commits on success, rolls back on exception.

SELECT statements do NOT open a transaction. Read-only queries run against the
current committed state (or the current open transaction's snapshot).

If the backend returns `BackendError::Unsupported { operation: "transactions" }`
(e.g. a future read-only backend), `commit()` and `rollback()` become no-ops
in autocommit mode and raise `NotSupportedError` in explicit mode.

---

## Pipeline Flow (Detailed)

Here is the complete sequence of calls when a user runs
`cursor.execute("SELECT name FROM users WHERE age > ?", (18,))`:

```python
# 1. Cursor.execute receives sql + params
# 2. Lex the SQL
tokens = sql_lexer.tokenize(sql)
# 3. Count placeholders, validate arity
placeholder_count = sum(1 for t in tokens if t.kind == "QMARK")
if placeholder_count != len(params):
    raise ProgrammingError(...)
# 4. Parse to AST
ast = sql_parser.parse_sql(sql)
# 5. Bind parameters into AST (replace placeholders with literal nodes)
ast = mini_sqlite.binding.bind_parameters(ast, params)
# 6. Plan — AST to LogicalPlan
plan = sql_planner.plan(ast, schema_provider=connection._backend)
# 7. Optimize — run default passes
plan = sql_optimizer.optimize(plan)
# 8. Code-gen — plan to bytecode
program = sql_codegen.compile(plan)
# 9. Execute
result = sql_vm.execute(program, connection._backend)
# 10. Populate cursor state
cursor._rows = [tuple(r) for r in result.rows]
cursor._row_iter = iter(cursor._rows)
cursor.description = [(col, None, None, None, None, None, None) for col in result.columns]
cursor.rowcount = result.rows_affected if result.rows_affected is not None else len(cursor._rows)
```

Errors at any step are translated per the table above and raised as the
appropriate PEP 249 exception.

---

## FileBackend (Persistent Storage)

`mini-sqlite` ships a `FileBackend` that persists data across process boundaries.
This is the "single file database" aspect of SQLite, implemented simply.

### File Format

A `mini-sqlite` database is a **single directory** on disk:

```
mydb/
├── manifest.json            # list of tables and their schemas
├── tables/
│   ├── users.json           # rows for the "users" table (JSON array of row maps)
│   ├── orders.json          # rows for the "orders" table
│   └── ...
└── journal.json             # transaction journal (for crash recovery)
```

Storing each table as its own JSON file has nice properties:
- Trivial to inspect with `cat` and `jq`
- Diffable in git
- Crash-resistant via atomic rename after write
- Easy to port to other languages later

The `manifest.json` format:
```json
{
  "version": 1,
  "tables": {
    "users": {
      "columns": [
        {"name": "id", "type": "INTEGER", "primary_key": true, "not_null": true},
        {"name": "name", "type": "TEXT", "not_null": true},
        {"name": "age", "type": "INTEGER"}
      ]
    }
  }
}
```

The table file format (`users.json`):
```json
[
  {"id": 1, "name": "Alice", "age": 30},
  {"id": 2, "name": "Bob",   "age": 25}
]
```

### Naming conventions

- If the user passes a plain path `mydb.db` (or any non-directory path ending in
  a suffix), the backend treats it as a directory path — `mydb.db/` — and creates
  the directory and files inside. This is a deliberate tradeoff: SQLite file
  compatibility is out of scope, and directory-of-JSON is dramatically simpler.

### Transaction journaling

`begin_transaction()` writes the current state of each mutated table to
`journal.json` before any writes. `commit()` deletes the journal. `rollback()`
restores each table from the journal.

On startup, if `journal.json` exists, the backend treats this as a crash and
rolls back automatically. This provides atomic commit semantics at the file-system
level (SQLite's own rollback journal works the same way conceptually).

### Performance caveat

This backend is **simple, not fast**. Every mutation rewrites the entire table
file. This is fine for educational and small-data use cases; it is not fine for
millions of rows. Future backends (a real B-tree file format, SQLite bytes-compatible,
etc.) are out of scope for this spec.

---

## Subset of SQL Supported in v1

From `sql.md`, the following statements are supported by v1 of mini-sqlite:

### DQL (Data Query)
- `SELECT [DISTINCT] column_list FROM table [AS alias]`
- `[INNER|LEFT|RIGHT|FULL|CROSS] JOIN table [AS alias] ON condition`
- `WHERE predicate`
- `GROUP BY column_list`
- `HAVING predicate`
- `ORDER BY column [ASC|DESC], ...`
- `LIMIT n [OFFSET m]`

### DML (Data Manipulation)
- `INSERT INTO table [(column_list)] VALUES (...)`
- `UPDATE table SET column = expr [, ...] [WHERE predicate]`
- `DELETE FROM table [WHERE predicate]`

### DDL (Data Definition)
- `CREATE TABLE [IF NOT EXISTS] table (column_def, ...)`
- `DROP TABLE [IF EXISTS] table`

### Not supported in v1
- Subqueries (scalar, IN, EXISTS)
- `UNION / INTERSECT / EXCEPT`
- CTEs (`WITH`)
- `CASE WHEN ... THEN ... END`
- Window functions (`OVER`)
- Indexes, triggers, views
- `PRAGMA` statements
- Foreign keys (parsed but not enforced)
- `BLOB` type

These can land as v2 features once v1 passes its test suite.

---

## Testing Strategy

### Unit tests (per package)

Each of the seven underlying packages (`sql-lexer` through `sql-backend`) owns
its own conformance tests per its spec. `mini-sqlite` does not re-test those.

### Integration tests (in `mini-sqlite`)

1. **Round-trip tests.** For a wide range of SQL statements:
   - Run through each pipeline stage
   - Assert the final `QueryResult` matches expectations
   - These tests catch bugs where two packages individually pass their unit tests
     but disagree on their shared interface

2. **DB-API 2.0 compliance tests.** A subset of Stuart Bishop's db-api compliance
   tests ported from `test/dbapi20.py`. Validates PEP 249 surface.

3. **Parity tests against `sqlite3`.** For every supported SQL statement, run the
   same SQL against Python's `sqlite3` and `mini_sqlite`. Compare:
   - Row count
   - Column names
   - Row data (after type coercion)
   - Error behavior (do both raise an error? same category?)

   Parity tests are the strongest integration test: they use a known-correct
   implementation as the oracle. If mini-sqlite diverges from sqlite3 on valid
   SQL, the test fails.

4. **Persistence tests.** With `FileBackend`:
   - Write some data, close the connection, reopen, verify data is intact
   - Start a transaction, crash mid-write (simulate by killing the Python process),
     reopen, verify rollback occurred
   - Concurrent writers: out of scope for v1 (single-writer only)

### Coverage target

Per repo convention: **>95% line coverage**, per the CLAUDE.md standard for libraries.

---

## Implementation Order

The six new Python packages have a strict dependency order. Build them in this
sequence, one fully-working package at a time:

1. **sql-backend** — leaf package, no SQL deps. Build `Backend` interface,
   `InMemoryBackend`, and the conformance test module. Pass all conformance tests.

2. **sql-planner** — depends only on `sql-parser` (already exists) and
   `sql-backend` (for SchemaProvider). Build the `LogicalPlan` type, the `plan()`
   function for every statement type, column resolution, and the conformance tests.

3. **sql-optimizer** — depends on `sql-planner`. Build each pass as a separate
   module, compose them in the default pipeline, pass the conformance tests.

4. **sql-codegen** — depends on `sql-optimizer` (and transitively `sql-planner`).
   Build the `Instruction` types, the `Program` type, the `compile()` function,
   pass the conformance tests.

5. **sql-vm** — depends on `sql-codegen` and `sql-backend`. Build the execution
   loop and every opcode handler. Pass the conformance tests.

6. **mini-sqlite** — depends on all of the above plus the existing `sql-lexer`
   and `sql-parser`. Build the `Connection`, `Cursor`, parameter binding, error
   translation, and `FileBackend`. Pass integration tests and parity tests.

**Each step must be fully green before starting the next.** Do not start
sql-planner while sql-backend's conformance tests are still failing.

---

## Example Session

```python
import mini_sqlite

# Connect to an in-memory database
conn = mini_sqlite.connect(":memory:")
cur = conn.cursor()

# Create a table
cur.execute("""
    CREATE TABLE employees (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        dept TEXT,
        salary INTEGER
    )
""")

# Insert some rows
cur.executemany(
    "INSERT INTO employees (id, name, dept, salary) VALUES (?, ?, ?, ?)",
    [
        (1, "Alice", "Engineering", 90000),
        (2, "Bob",   "Engineering", 75000),
        (3, "Carol", "Sales",        65000),
        (4, "Dave",  "Sales",        60000),
    ]
)
conn.commit()

# Simple query
cur.execute("SELECT name, salary FROM employees WHERE dept = ?", ("Engineering",))
print(cur.fetchall())
# [("Alice", 90000), ("Bob", 75000)]

# Aggregate
cur.execute("""
    SELECT dept, AVG(salary) AS avg_sal
    FROM employees
    GROUP BY dept
    HAVING AVG(salary) > 62000
    ORDER BY avg_sal DESC
""")
for row in cur:
    print(row)
# ("Engineering", 82500.0)
# ("Sales",        62500.0)

# Persistent database
conn2 = mini_sqlite.connect("./my_data.db")
conn2.execute("CREATE TABLE IF NOT EXISTS items (id INTEGER, name TEXT)")
conn2.execute("INSERT INTO items VALUES (?, ?)", (1, "widget"))
conn2.commit()
conn2.close()
# Contents persist to ./my_data.db/tables/items.json

# Reopen
conn3 = mini_sqlite.connect("./my_data.db")
rows = conn3.execute("SELECT * FROM items").fetchall()
print(rows)  # [(1, "widget")]
```

---

## Non-Goals

To keep v1 tractable, the following are **explicitly out of scope**:

- **Binary wire compatibility with SQLite** — we produce JSON files, not SQLite .db files
- **SQLite C API** — not a FFI target
- **Multi-process concurrency** — single-writer only; no locking
- **Indexes / query plans using indexes** — optimizer assumes full table scans
- **Query planning statistics** — no `ANALYZE`, no cost-based optimization
- **User-defined functions** — only the built-in aggregate/scalar functions
- **Triggers, views, foreign keys** — parsed but not enforced
- **BLOB / binary data** — TEXT, INTEGER, REAL, BOOLEAN, NULL only
- **Collations** — default byte-order string comparison only

Each of these is a plausible v2 follow-up spec.

---

## Relationship to Existing Specs

- **Depends on:** `sql.md`, `sql-planner.md`, `sql-optimizer.md`, `sql-codegen.md`,
  `sql-vm.md`, `sql-backend.md` (and transitively the existing lexer/parser specs).
- **Supersedes:** `sql-execution-engine.md` as the recommended way to execute SQL.
  The old engine package remains for backwards compatibility but is no longer the
  path forward.
- **Model for other languages:** Once mini-sqlite is working and tested in Python,
  a parallel implementation can follow in each of the other 16 repo languages.
  The public API surface (PEP 249 in Python) will be replaced by each language's
  idiomatic equivalent (`std::sql::Connection` in Rust, `Database/Sql` in Go, etc.),
  but the internal pipeline wiring is identical across all languages.
