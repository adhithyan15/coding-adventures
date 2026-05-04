# csharp/sql-backend

Pluggable SQL backend abstractions and an in-memory reference backend for C#.

This package ports the Python `sql-backend` contract into C#: schema metadata,
row iterators, positioned cursors, typed backend errors, DDL/DML operations,
simple index descriptors, rowid scans, and transaction snapshots.

## Usage

```csharp
var backend = new InMemoryBackend();
backend.CreateTable("users", new[] { new ColumnDef("id", "INTEGER", PrimaryKey: true) }, false);
backend.Insert("users", new Row { ["id"] = 1 });
```
