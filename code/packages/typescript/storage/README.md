# Storage

Unified storage interface — one contract for every backend.

## Architecture

One interface (`Storage`), multiple implementations:

- **MemoryStorage** (this package) — in-memory Map-of-Maps for tests, SSR, and fallback
- **IndexedDBStorage** (`@coding-adventures/indexeddb`) — browser persistent storage
- **Future:** SQLiteStorage, RestApiStorage, GoogleDriveStorage, OneDriveStorage, etc.

All implement the same `Storage` interface with four capabilities:

1. **Connection** — `open()` / `close()`
2. **CRUD** — `get()` / `getAll()` / `put()` / `delete()`
3. **SQL Querying** — `query()` with real SQL, parsed by `sql-parser` and executed by `sql-execution-engine`
4. **Transactions** — `transaction()` for atomic batches with rollback on failure

## Usage

```typescript
import { MemoryStorage } from "@coding-adventures/storage";
import type { Storage } from "@coding-adventures/storage";

const storage: Storage = new MemoryStorage([
  { name: "users", keyPath: "id" },
  { name: "posts", keyPath: "id" },
]);
await storage.open();

// CRUD
await storage.put("users", { id: "1", name: "Alice", age: 30 });
const user = await storage.get("users", "1");
const allUsers = await storage.getAll("users");
await storage.delete("users", "1");

// SQL Querying
const result = await storage.query("SELECT * FROM users WHERE age > 25 ORDER BY name");
console.log(result.columns); // ["id", "name", "age"]
console.log(result.rows);    // [{ id: "1", name: "Alice", age: 30 }]

// Transactions
await storage.transaction(async (tx) => {
  await tx.delete("users", "1");
  await tx.delete("posts", "p1");
  // if either throws, both roll back
});
```

## Design Philosophy

Every storage system is fundamentally a blob store: `namespace + key -> blob`.
IndexedDB calls namespaces "object stores". SQLite calls them "tables". Google
Drive calls them "folders". This interface abstracts over all of them.

Smart backends (SQLite) execute SQL natively. Dumb backends (IndexedDB, Google
Drive) load all records and delegate to the `sql-execution-engine` package for
in-memory filtering. Same interface, different performance profiles.

## Testing

```bash
npm install
npm test
```

## Spec

See [`/code/specs/sql-execution-engine.md`](/code/specs/sql-execution-engine.md) for the
SQL execution engine specification that powers the `query()` method.
