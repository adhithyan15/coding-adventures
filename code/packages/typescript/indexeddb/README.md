# IndexedDB

A Promise-based wrapper around the browser's IndexedDB API.

## Architecture

Two storage implementations sharing a common interface:

- **IndexedDBStorage** — wraps the raw browser IndexedDB API in Promises.
  Each method opens a transaction, performs the operation, and returns a
  Promise that resolves on success or rejects on error. Currently implements
  `KVStorage` (CRUD only).

- **MemoryStorage** — re-exported from `@coding-adventures/storage`. Implements
  the full `Storage` interface (CRUD + SQL querying + transactions).

The unified `Storage` interface and all schema types now live in
`@coding-adventures/storage`. This package re-exports them for backward
compatibility. The old `KVStorage` interface (CRUD-only subset) is kept as
a backward-compatible alias. New code should prefer `Storage` from
`@coding-adventures/storage`.

## Usage

```typescript
import { IndexedDBStorage } from "@coding-adventures/indexeddb";

const storage = new IndexedDBStorage({
  dbName: "my-app",
  version: 1,
  stores: [
    { name: "users", keyPath: "id" },
    { name: "posts", keyPath: "id", indexes: [{ name: "authorId", keyPath: "authorId" }] },
  ],
});

await storage.open();
await storage.put("users", { id: "1", name: "Alice" });
const user = await storage.get("users", "1");
const allUsers = await storage.getAll("users");
await storage.delete("users", "1");
```

## Testing

```bash
npm install
npm run test
```

## Spec

See [`/code/specs/checklist-app.md`](/code/specs/checklist-app.md) for the
full specification.
