# IndexedDB

A Promise-based wrapper around the browser's IndexedDB API. No external dependencies.

## Architecture

One interface (`KVStorage`), two implementations:

- **IndexedDBStorage** — wraps the raw browser IndexedDB API in Promises.
  Each method opens a transaction, performs the operation, and returns a
  Promise that resolves on success or rejects on error.

- **MemoryStorage** — in-memory Map-of-Maps for testing and environments
  where IndexedDB is unavailable (Node, SSR).

Both implement the same `KVStorage` interface, so consuming code can swap
between them without changes.

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
