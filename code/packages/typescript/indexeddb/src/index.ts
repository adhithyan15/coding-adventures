/**
 * @coding-adventures/indexeddb
 *
 * Promise-based IndexedDB wrapper. Two implementations of the same
 * KVStorage interface:
 *
 *   - IndexedDBStorage: wraps the raw browser IndexedDB API
 *   - MemoryStorage: in-memory Map-of-Maps for tests and fallback
 *
 * Usage:
 * ```typescript
 * import { IndexedDBStorage, MemoryStorage } from "@coding-adventures/indexeddb";
 * import type { KVStorage } from "@coding-adventures/indexeddb";
 * ```
 */

export type { KVStorage, StorageRecord, StorageConfig, StoreSchema, IndexSchema } from "./types.js";
export { IndexedDBStorage } from "./indexeddb-storage.js";
export { MemoryStorage } from "./memory-storage.js";
