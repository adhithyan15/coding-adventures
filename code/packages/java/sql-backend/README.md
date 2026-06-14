# java/sql-backend

Pluggable SQL backend abstractions and an in-memory reference backend for Java.

This package ports the Python `sql-backend` contract into Java: schema
metadata, row iterators, positioned cursors, typed backend errors, DDL/DML
operations, index descriptors, rowid scans, and transaction snapshots.

## Usage

```java
var backend = new SqlBackend.InMemoryBackend();
backend.createTable("users", List.of(new SqlBackend.ColumnDef("id", "INTEGER")), false);
backend.insert("users", row("id", 1));
```
