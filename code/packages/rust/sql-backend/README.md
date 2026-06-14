# rust/sql-backend

Pluggable SQL backend abstractions and an in-memory reference backend for Rust.

This crate ports the Python `sql-backend` contract: SQL values, row iterators,
positioned cursors, schema metadata, typed backend errors, DDL/DML operations,
index descriptors, rowid scans, transactions, savepoints, triggers, and version
fields.

## Usage

```rust
use coding_adventures_sql_backend::{ColumnDef, InMemoryBackend, Row, SqlValue};

let mut backend = InMemoryBackend::new();
backend.create_table(
    "users",
    vec![ColumnDef::new("id", "INTEGER").primary_key()],
    false,
)?;
backend.insert("users", Row::from([("id".to_string(), SqlValue::Int(1))]))?;
# Ok::<(), coding_adventures_sql_backend::BackendError>(())
```
