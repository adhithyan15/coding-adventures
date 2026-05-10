# dart/sql-backend

Pluggable SQL backend abstractions and an in-memory reference backend for Dart.

This package ports the Python `sql-backend` contract into Dart: schema
metadata, row iterators, positioned cursors, typed backend errors, DDL/DML
operations, index descriptors, rowid scans, and transaction snapshots.

## Usage

```dart
final backend = InMemoryBackend();
backend.createTable('users', [const ColumnDef('id', 'INTEGER')], false);
backend.insert('users', {'id': 1});
```
