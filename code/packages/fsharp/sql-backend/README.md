# fsharp/sql-backend

Pluggable SQL backend abstractions and an in-memory reference backend for F#.

This package ports the Python `sql-backend` contract into F#: schema metadata,
row iterators, positioned cursors, typed backend errors, DDL/DML operations,
index descriptors, rowid scans, and transaction snapshots.

## Usage

```fsharp
let backend = InMemoryBackend()
backend.CreateTable("users", [| ColumnDef("id", "INTEGER", primaryKey = true) |], false)
backend.Insert("users", row [ "id", box 1 ])
```
