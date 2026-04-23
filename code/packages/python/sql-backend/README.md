# sql-backend (Python)

Pluggable data-source interface for the SQL query pipeline.

The `sql-backend` package is the **outermost ring** of the SQL pipeline — the
only part that actually touches data. Every other stage (planner, optimizer,
codegen, VM) is data-agnostic; they all call into a `Backend` instance when
they need to read or write rows.

This package ships five things:

1. The `Backend` abstract base class — the interface every data source implements.
2. The supporting types — `Row`, `Cursor`, `RowIterator`, `ColumnDef`, `SqlValue`.
3. The error hierarchy — `BackendError` and its eight subclasses.
4. `InMemoryBackend` — the reference implementation, used for `:memory:` connections in `mini-sqlite` and as the yardstick for every other backend.
5. `IndexDef` — descriptor for named indexes, used by `create_index` / `list_indexes`.

## Where it fits

```
┌────────────────────────────┐
│          sql-vm            │  executes bytecode
└─────────────┬──────────────┘
              │  calls
              ▼
┌────────────────────────────┐
│   Backend (interface)      │  ← this package
└─────────────┬──────────────┘
              │  implemented by
      ┌───────┴──────────────────────────────┐
      ▼              ▼           ▼           ▼
InMemoryBackend  CsvBackend  FileBackend  SqliteBackend
 (this package)  (sql-csv-   (mini-       (future)
                  source)     sqlite)
```

The VM never knows whether the rows it reads come from a hash map, a CSV
file, a file on disk, or a remote server. That decoupling is the whole
point of this package.

## Usage

```python
from sql_backend import InMemoryBackend, ColumnDef

backend = InMemoryBackend()

backend.create_table(
    "users",
    [
        ColumnDef(name="id", type_name="INTEGER", primary_key=True),
        ColumnDef(name="name", type_name="TEXT", not_null=True),
        ColumnDef(name="email", type_name="TEXT", unique=True),
    ],
    if_not_exists=False,
)

backend.insert("users", {"id": 1, "name": "alice", "email": "a@example.com"})

# Iterate rows.
it = backend.scan("users")
while True:
    row = it.next()
    if row is None:
        break
    print(row)
it.close()
```

## Transactions

`InMemoryBackend` supports snapshot-and-restore transactions:

```python
h = backend.begin_transaction()
backend.insert("users", {"id": 2, "name": "bob", "email": "b@example.com"})
backend.rollback(h)  # the insert is undone
```

Nested transactions raise `Unsupported`. Backends that don't support
transactions at all raise `Unsupported` from all three transaction methods.

## Implementing a new backend

Subclass `Backend`, implement every abstract method, and raise `Unsupported`
from anything you can't support. Then run the conformance suite:

```python
from sql_backend.conformance import run_required, run_read_write, run_ddl, run_transaction

def my_factory():
    return build_my_preloaded_backend()

run_required(my_factory)      # minimum — every backend must pass
run_read_write(my_factory)    # if your backend accepts INSERT/UPDATE/DELETE
run_ddl(my_factory)           # if your backend supports CREATE/DROP TABLE
run_transaction(my_factory)   # if your backend implements BEGIN/COMMIT/ROLLBACK
```

A backend that passes the relevant tiers is a drop-in replacement for
`InMemoryBackend`.

## Indexes

`Backend` exposes four index methods. `InMemoryBackend` implements them via a
linear scan (no physical B-tree); `SqliteFileBackend` maintains a real on-disk
index B-tree in `IndexTree`.

```python
from sql_backend import InMemoryBackend, ColumnDef, IndexDef

backend = InMemoryBackend()
backend.create_table(
    "users",
    [
        ColumnDef(name="id",  type_name="INTEGER", primary_key=True),
        ColumnDef(name="age", type_name="INTEGER"),
    ],
)
backend.insert("users", {"id": 1, "age": 30})
backend.insert("users", {"id": 2, "age": 25})
backend.insert("users", {"id": 3, "age": 30})

# Register an index.
backend.create_index(IndexDef(name="idx_age", table="users", columns=["age"]))

# Equality lookup — yields rowids in ascending key order.
rowids = list(backend.scan_index("idx_age", [30], [30]))  # age = 30

# Range scan with exclusive bounds.
rowids = list(backend.scan_index("idx_age", [25], None, lo_inclusive=False))

# List and drop.
print(backend.list_indexes("users"))   # [IndexDef(name='idx_age', ...)]
backend.drop_index("idx_age")
backend.drop_index("idx_age", if_exists=True)   # no error
```

## Error hierarchy

```
BackendError
├── TableNotFound
├── TableAlreadyExists
├── ColumnNotFound
├── ConstraintViolation       (NOT NULL / UNIQUE / PRIMARY KEY)
├── Unsupported                (e.g. "transactions")
├── Internal                   (escape hatch — use sparingly)
├── IndexAlreadyExists         (create_index on a duplicate name)
└── IndexNotFound              (drop_index / scan_index on unknown name)
```

## Development

```
uv venv --clear
uv pip install -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```

Or, from the repo root:

```
./build-tool --packages python/sql-backend
```

## Specification

See [`code/specs/sql-backend.md`](../../../specs/sql-backend.md) for the
full interface specification, conformance test checklist, and cross-language
signatures.
