/**
 * @coding-adventures/storage
 *
 * Unified storage interface — one contract for every backend.
 *
 * The Storage interface provides four capabilities:
 *   - Connection: open() / close()
 *   - CRUD: get() / getAll() / put() / delete()
 *   - SQL Querying: query() — real SQL, parsed and executed
 *   - Transactions: transaction() — atomic batches with rollback
 *
 * Implementations:
 *   - MemoryStorage (this package) — in-memory Map-of-Maps for tests/fallback
 *   - IndexedDBStorage (@coding-adventures/indexeddb) — browser storage
 *   - Future: SQLiteStorage, RestApiStorage, GoogleDriveStorage, etc.
 *
 * Usage:
 * ```typescript
 * import { MemoryStorage } from "@coding-adventures/storage";
 * import type { Storage } from "@coding-adventures/storage";
 *
 * const storage: Storage = new MemoryStorage([
 *   { name: "users", keyPath: "id" },
 * ]);
 * await storage.open();
 * await storage.put("users", { id: "1", name: "Alice" });
 * const result = await storage.query("SELECT * FROM users WHERE name = 'Alice'");
 * ```
 */

export type {
  Storage,
  StorageRecord,
  StorageConfig,
  StoreSchema,
  IndexSchema,
  QueryResult,
  SqlValue,
} from "./types.js";
export { MemoryStorage } from "./memory-storage.js";
