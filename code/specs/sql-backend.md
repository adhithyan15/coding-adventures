# SQL Backend Specification

## Overview

This document specifies the `sql-backend` package: the **pluggable backend interface**
that separates the SQL VM from any particular data store.

The backend is the outermost ring of the SQL pipeline — it is the only part that
touches actual data:

```
sql-vm
    │  calls into
    ▼
Backend (interface)
    │  implemented by
    ├── InMemoryBackend  (this package — reference implementation)
    ├── CsvBackend       (sql-csv-source package)
    ├── SqliteBackend    (storage-sqlite package — future)
    └── ... any future backend
```

**The core idea: data-agnostic queries**

The VM knows how to execute SQL opcodes. It does not know whether the data lives
in a hash map, a CSV file, a SQLite database, or a remote HTTP endpoint. All of
that is hidden behind the `Backend` interface.

This is the same design used by:
- PostgreSQL's foreign data wrapper (FDW) system — any data source can look like a table
- Apache Arrow's DataFusion — queries run over CSV, Parquet, or in-memory arrays
- SQLite's virtual table mechanism — backends implement a small C interface

---

## Where It Fits

```
Depends on: nothing (leaf package — no SQL-pipeline dependencies)
Used by:    sql-vm (calls the backend interface during execution)
            sql-planner (schema provider role — columns(table) → column names)
```

The `sql-backend` package ships:
1. The `Backend` interface definition
2. The `SchemaProvider` interface definition (used by the planner)
3. An `InMemoryBackend` reference implementation
4. The shared `conformance` test helpers every backend must pass

---

## Supported Languages

All 17 repository languages implement this package:
`csharp`, `dart`, `elixir`, `fsharp`, `go`, `haskell`, `java`, `kotlin`, `lua`,
`perl`, `python`, `ruby`, `rust`, `starlark`, `swift`, `typescript`, `wasm`.

---

## The Backend Interface

```
Backend:
    -- Schema
    tables()                          → Vec<String>
    columns(table: String)            → Result<Vec<ColumnDef>, BackendError>

    -- Read
    scan(table: String)               → Result<RowIterator, BackendError>

    -- Write
    insert(table: String, row: Row)   → Result<(), BackendError>
    update(table: String,
           cursor: &mut Cursor,
           assignments: Map<String, SqlValue>)
                                      → Result<(), BackendError>
    delete(table: String,
           cursor: &mut Cursor)       → Result<(), BackendError>

    -- DDL
    create_table(table: String,
                 columns: Vec<ColumnDef>,
                 if_not_exists: bool) → Result<(), BackendError>
    drop_table(table: String,
               if_exists: bool)       → Result<(), BackendError>

    -- Transactions (optional — backends may return Err::Unsupported)
    begin_transaction()               → Result<TransactionHandle, BackendError>
    commit(handle: TransactionHandle) → Result<(), BackendError>
    rollback(handle: TransactionHandle) → Result<(), BackendError>
```

Language-specific signatures:

**Rust**
```rust
pub trait Backend: Send + Sync {
    fn tables(&self) -> Vec<String>;
    fn columns(&self, table: &str) -> Result<Vec<ColumnDef>, BackendError>;
    fn scan(&self, table: &str) -> Result<Box<dyn RowIterator>, BackendError>;
    fn insert(&self, table: &str, row: Row) -> Result<(), BackendError>;
    fn update(&self, table: &str, cursor: &mut dyn Cursor, assignments: HashMap<String, SqlValue>) -> Result<(), BackendError>;
    fn delete(&self, table: &str, cursor: &mut dyn Cursor) -> Result<(), BackendError>;
    fn create_table(&self, table: &str, columns: Vec<ColumnDef>, if_not_exists: bool) -> Result<(), BackendError>;
    fn drop_table(&self, table: &str, if_exists: bool) -> Result<(), BackendError>;
    fn begin_transaction(&self) -> Result<TransactionHandle, BackendError>;
    fn commit(&self, handle: TransactionHandle) -> Result<(), BackendError>;
    fn rollback(&self, handle: TransactionHandle) -> Result<(), BackendError>;
}
```

**TypeScript**
```typescript
export interface Backend {
  tables(): string[];
  columns(table: string): ColumnDef[];  // throws BackendError

  scan(table: string): RowIterator;     // throws BackendError

  insert(table: string, row: Row): void;
  update(table: string, cursor: Cursor, assignments: Record<string, SqlValue>): void;
  delete(table: string, cursor: Cursor): void;

  createTable(table: string, columns: ColumnDef[], ifNotExists: boolean): void;
  dropTable(table: string, ifExists: boolean): void;

  beginTransaction(): TransactionHandle;
  commit(handle: TransactionHandle): void;
  rollback(handle: TransactionHandle): void;
}
```

**Go**
```go
type Backend interface {
    Tables() []string
    Columns(table string) ([]ColumnDef, error)
    Scan(table string) (RowIterator, error)
    Insert(table string, row Row) error
    Update(table string, cursor Cursor, assignments map[string]SqlValue) error
    Delete(table string, cursor Cursor) error
    CreateTable(table string, columns []ColumnDef, ifNotExists bool) error
    DropTable(table string, ifExists bool) error
    BeginTransaction() (TransactionHandle, error)
    Commit(handle TransactionHandle) error
    Rollback(handle TransactionHandle) error
}
```

**Python**
```python
class Backend(ABC):
    @abstractmethod
    def tables(self) -> list[str]: ...
    @abstractmethod
    def columns(self, table: str) -> list[ColumnDef]: ...
    @abstractmethod
    def scan(self, table: str) -> RowIterator: ...
    @abstractmethod
    def insert(self, table: str, row: Row) -> None: ...
    @abstractmethod
    def update(self, table: str, cursor: Cursor, assignments: dict[str, SqlValue]) -> None: ...
    @abstractmethod
    def delete(self, table: str, cursor: Cursor) -> None: ...
    @abstractmethod
    def create_table(self, table: str, columns: list[ColumnDef], if_not_exists: bool) -> None: ...
    @abstractmethod
    def drop_table(self, table: str, if_exists: bool) -> None: ...
    @abstractmethod
    def begin_transaction(self) -> TransactionHandle: ...
    @abstractmethod
    def commit(self, handle: TransactionHandle) -> None: ...
    @abstractmethod
    def rollback(self, handle: TransactionHandle) -> None: ...
```

---

## Supporting Types

### Row

A single table row — a map from column name to SQL value.

```
Row = Map<String, SqlValue>

SqlValue = Null
         | Int(i64)
         | Float(f64)
         | Text(String)
         | Bool(bool)
```

Type widening rules (for arithmetic in the VM):
- `Int` and `Float` interoperate; result is `Float`
- `Text`, `Bool`, `Null` do not participate in arithmetic

---

### ColumnDef

Describes one column in a table schema.

```
ColumnDef {
    name:        String
    type_name:   String       -- "INTEGER", "TEXT", "REAL", "BOOLEAN", etc.
                               -- stored as a string; backends interpret it
    not_null:    bool
    primary_key: bool
    unique:      bool
    default:     Option<SqlValue>
}
```

The `type_name` field is the string token from the `CREATE TABLE` statement, just
as it appears in the SQL grammar. Backends interpret this string according to their
own type system. The VM does not enforce column types — that is the backend's
responsibility.

---

### RowIterator

An iterator over the rows of a table, one row at a time. The VM calls `next()` in
a loop until the iterator signals exhaustion.

```
RowIterator:
    next() → Option<Row>     -- Some(row) or None when exhausted
    close()                  -- release backend resources (file handles, etc.)
```

Backends that hold all rows in memory can return a simple list-backed iterator.
Backends with streaming data (files, network) should stream rows lazily. The VM
treats both identically.

---

### Cursor

A `Cursor` is a `RowIterator` that also supports positioned updates and deletes.
Backends that support mutation must provide a `Cursor` that can identify the current
row for UPDATE and DELETE.

```
Cursor extends RowIterator:
    current_row() → Option<Row>   -- the most recent row returned by next()
```

How a backend implements "current row identity" is an internal detail:
- An in-memory backend uses the current index into its Vec
- A file-backed backend uses a byte offset
- A SQLite backend uses a rowid

The backend's `update` and `delete` methods receive the cursor and can interrogate
`current_row()` to identify which row to modify.

---

### TransactionHandle

An opaque token representing an active transaction. The backend issues it on
`begin_transaction()` and accepts it on `commit()` or `rollback()`.

```
TransactionHandle = opaque token (backend-specific)
```

Backends that do not support transactions must return
`BackendError::Unsupported { operation: "transactions" }` from all three transaction
methods. The VM propagates this error to the caller.

---

## BackendError Types

```
BackendError =
    | TableNotFound      { table: String }
    | TableAlreadyExists { table: String }
    | ColumnNotFound     { table: String, column: String }
    | ConstraintViolation { table: String, column: String, message: String }
                           -- NOT NULL, UNIQUE, PRIMARY KEY violations
    | Unsupported        { operation: String }
                           -- e.g. "transactions", "update", "delete"
    | Internal           { message: String }
```

Backends **must** translate their native error types into these. The VM does not
know about SQLite error codes, CSV parse errors, or HTTP status codes — it only
sees `BackendError`.

---

## The SchemaProvider Interface

The SQL planner needs to know which columns each table has in order to qualify
column references and detect ambiguity. The `Backend` interface satisfies this role
via its `columns()` method.

A minimal interface is defined for planners that want to operate independently of
a full Backend (e.g. in unit tests):

```
SchemaProvider:
    columns(table: String) → Result<Vec<String>, PlanError>
```

`InMemoryBackend` implements both `Backend` and `SchemaProvider`. Any `Backend`
implementation can be wrapped to provide `SchemaProvider` semantics.

---

## The InMemoryBackend Reference Implementation

`InMemoryBackend` is the reference implementation shipped in this package. It stores
tables as a `Map<String, TableData>` where `TableData` contains:
- The table schema: `Vec<ColumnDef>`
- The rows: `Vec<Row>` (ordered by insertion)

It supports all Backend operations including transactions (snapshot-and-restore).

### Construction

```
InMemoryBackend::new() → InMemoryBackend

-- Pre-populate with tables and rows for testing:
InMemoryBackend::from_tables(tables: Map<String, (Vec<ColumnDef>, Vec<Row>)>) → InMemoryBackend
```

### Constraint enforcement

The `InMemoryBackend` enforces these constraints on `insert` and `update`:
- `NOT NULL`: value must not be `Null`
- `UNIQUE`: no existing row has the same value for this column
- `PRIMARY KEY`: implies `NOT NULL` + `UNIQUE`

`DEFAULT` values: if a column has a default and the inserted row omits that column,
the default is applied automatically.

### Transaction semantics

`InMemoryBackend` implements transactions via **snapshot-and-restore**:
- `begin_transaction()` deep-clones the entire table map and returns a handle
- `commit()` discards the snapshot (changes are already applied to live state)
- `rollback()` replaces the live state with the snapshot

Nested transactions are not supported — `begin_transaction()` while one is already
active returns `BackendError::Unsupported { operation: "nested transactions" }`.

### Thread safety

`InMemoryBackend` is **not thread-safe** by default. For concurrent use, wrap it
in a language-appropriate mutex. Thread-safety is not required by the `Backend`
interface — backends that support concurrency (e.g. SQLite in WAL mode) handle it
internally.

---

## Implementing a New Backend

To add a new data source, implement the `Backend` interface:

1. **Implement `tables()` and `columns()`** — schema discovery
2. **Implement `scan()`** — return a `RowIterator`
3. **Implement `insert()`, `update()`, `delete()`** — mutations (or return `Unsupported`)
4. **Implement `create_table()` and `drop_table()`** — DDL (or return `Unsupported`)
5. **Implement `begin_transaction()`, `commit()`, `rollback()`** — transactions (or return `Unsupported`)
6. **Run the conformance tests** — all required tests must pass before the backend is merged

Read-only backends (like `sql-csv-source`) may return `Unsupported` for all write,
DDL, and transaction methods. The VM respects this and raises an appropriate error
to the caller.

---

## Conformance Tests

The `sql-backend` package ships a shared `conformance` module with helpers that
every backend must pass. Tests are written against the `Backend` interface — any
implementation that passes all required tests is correct.

### Required (all backends must pass)

1. `tables()` returns registered table names
2. `columns(t)` returns column definitions for table `t`
3. `columns(unknown)` raises `BackendError::TableNotFound`
4. `scan(t)` returns all rows in insertion order
5. `scan(unknown)` raises `BackendError::TableNotFound`
6. `scan()` on empty table returns zero rows

### Required for read-write backends

7. `insert()` adds a row retrievable by subsequent `scan()`
8. `insert()` with missing NOT NULL column raises `ConstraintViolation`
9. `insert()` duplicate primary key raises `ConstraintViolation`
10. `insert()` applies column defaults when column is omitted
11. `update()` modifies the row visible in subsequent `scan()`
12. `update()` with NOT NULL violation raises `ConstraintViolation`
13. `delete()` removes the row from subsequent `scan()`

### Required for DDL backends

14. `create_table()` makes the table visible in `tables()` and `columns()`
15. `create_table(if_not_exists=false)` on existing table raises `TableAlreadyExists`
16. `create_table(if_not_exists=true)` on existing table is a no-op
17. `drop_table()` removes the table from `tables()`
18. `drop_table(if_exists=false)` on missing table raises `TableNotFound`
19. `drop_table(if_exists=true)` on missing table is a no-op

### Required for transaction backends

20. Mutations in a committed transaction are visible after commit
21. Mutations in a rolled-back transaction are not visible after rollback
22. Rollback restores exact state to pre-transaction

### Optional (backends may skip)

23. `begin_transaction()` while transaction active returns `Unsupported`

---

## Relationship to Existing Packages

- **Replaces** the `DataSource` interface in `sql-execution-engine`. The old `DataSource`
  was read-only (`schema` + `scan` only). The new `Backend` adds full DML, DDL, and
  transaction support.
- `sql-csv-source` currently implements `DataSource` for the old engine. It will be
  updated to implement `Backend`, returning `Unsupported` for all write operations
  (CSV files are naturally read-only).
- The `MemoryStorage` and `InMemoryBackend` serve different purposes:
  `MemoryStorage` is the storage abstraction for Chief of Staff agents (records with
  namespaces, revisions, and leases); `InMemoryBackend` is a SQL table store.
  They should not be merged.
- Any future `SqliteBackend` will implement this `Backend` interface, replacing
  the current `DataSource` shim in TypeScript's `MemoryStorage.query()`.
