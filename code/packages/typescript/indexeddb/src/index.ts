/**
 * @coding-adventures/indexeddb
 *
 * Promise-based IndexedDB wrapper with two storage implementations:
 *
 *   - IndexedDBStorage: wraps the raw browser IndexedDB API (CRUD only)
 *   - MemoryStorage: in-memory Map-of-Maps with full Storage support
 *     (CRUD + SQL querying + transactions), re-exported from
 *     @coding-adventures/storage
 *
 * The Storage interface and all schema types now live in
 * @coding-adventures/storage. This package re-exports them for
 * backward compatibility. New code should import directly from
 * @coding-adventures/storage for the full interface.
 *
 * Usage:
 * ```typescript
 * import { IndexedDBStorage, MemoryStorage } from "@coding-adventures/indexeddb";
 * import type { KVStorage } from "@coding-adventures/indexeddb";
 * ```
 */

export type {
  KVStorage,
  Storage,
  StorageRecord,
  StorageConfig,
  StoreSchema,
  IndexSchema,
  QueryResult,
  SqlValue,
} from "./types.js";
export { IndexedDBStorage } from "./indexeddb-storage.js";
export { MemoryStorage } from "./memory-storage.js";
