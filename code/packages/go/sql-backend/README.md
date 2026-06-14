# go/sql-backend

Pluggable SQL backend abstractions and an in-memory reference backend for Go.

This package ports the Python `sql-backend` contract: SQL values, row
iterators, positioned cursors, schema metadata, typed backend errors, DDL/DML
operations, index descriptors, rowid scans, transactions, savepoints, triggers,
and version fields.

## Usage

```go
backend := sqlbackend.NewInMemoryBackend()
err := backend.CreateTable("users", []sqlbackend.ColumnDef{
    {Name: "id", TypeName: "INTEGER", PrimaryKey: true},
}, false)
if err != nil {
    return err
}
return backend.Insert("users", sqlbackend.Row{"id": int64(1)})
```
