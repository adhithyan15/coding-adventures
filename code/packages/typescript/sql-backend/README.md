# typescript/sql-backend

Pluggable SQL backend abstractions and an in-memory reference backend for TypeScript.

This package ports the Python `sql-backend` contract: SQL values, row iterators,
positioned cursors, schema metadata, typed backend errors, DDL/DML operations,
index descriptors, rowid scans, transactions, savepoints, triggers, and schema
version fields.

## Usage

```typescript
import { ColumnDef, InMemoryBackend } from "@coding-adventures/sql-backend";

const backend = new InMemoryBackend();
backend.createTable("users", [new ColumnDef("id", "INTEGER", { primaryKey: true })], false);
backend.insert("users", { id: 1 });
```
