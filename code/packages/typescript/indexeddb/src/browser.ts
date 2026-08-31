/** Browser-safe CRUD entrypoint with no SQL storage dependency. */

export type {
  IndexSchema,
  KVStorage,
  StorageConfig,
  StoreSchema,
} from "./browser-types.js";
export { IndexedDBStorage } from "./indexeddb-storage.js";
export { MemoryStorage } from "./memory-storage.js";
