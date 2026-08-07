# storage-sqlite — a real `.sqlite` file as a query-engine backend

`SqliteFileBackend` exposes a genuine SQLite database file as the
[`Backend`](../sql-backend) the mini-sqlite pipeline reads and writes through, so
the *unmodified* query engine (`sql-lexer → parser → planner → optimizer →
codegen → sql-vm`) can run `SELECT` against a real `.sqlite` on disk.

This is the Rust sibling of `python/storage-sqlite`, which already proved this
architecture end to end. It is built entirely on the from-scratch,
zero-dependency [`sqlite-file`](../sqlite-file) reader — so reading a real
database links **no** third-party SQLite. The real library appears only as a
dev-dependency *oracle* in the cross-check tests.

## Where it fits

```
mini-sqlite ─▶ planner ─▶ codegen ─▶ sql-vm ─▶ Backend
                                               ├── InMemoryBackend   (":memory:")
                                               └── SqliteFileBackend (a .sqlite file)  ← this crate
                                                        └── sqlite-file (zero-dep on-disk reader)
```

## Scope (this increment)

**Read-only.** The read methods a `SELECT` needs are implemented for real:

| Method | Behaviour |
|--------|-----------|
| `tables()` | user table names from `sqlite_schema` (skips `sqlite_*` internals) |
| `columns(t)` | column list recovered from the table's `CREATE TABLE` text |
| `scan(t)` | every row as `Row = BTreeMap<String, SqlValue>`, with `INTEGER PRIMARY KEY` columns materialized from the rowid |

Every mutating method (`insert`/`update`/`delete`, DDL, indexes) returns
`BackendError::Unsupported`. Writing a byte-compatible file is a later increment
(the storage engine's Phase-F writer). Wiring mini-sqlite's `connect()` to open a
file through this backend is the next step.

## Usage

```rust,ignore
use coding_adventures_storage_sqlite::SqliteFileBackend;

let bytes = std::fs::read("collection.anki2")?;
let backend = SqliteFileBackend::open(bytes)?;
for table in backend.tables() {
    println!("{table}: {:?}", backend.columns(&table)?);
}
```

## Verification

`tests/cross_check.rs` builds genuine `.sqlite` files with `rusqlite` (dev-only)
and asserts this backend reads back the same tables, columns, and rows — types,
NULLs, and rowid aliases included — that the real library reports over SQL.

## Testing

```
cargo test -p coding-adventures-storage-sqlite
```
