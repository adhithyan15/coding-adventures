/**
 * types.ts — The storage contract and schema definitions.
 *
 * KVStorage is the interface that all storage backends implement.
 * Think of it as the JDBC of browser storage — one interface, multiple
 * implementations (IndexedDB for production, Map-of-Maps for testing).
 *
 * The interface is intentionally minimal: get, getAll, put, delete.
 * No queries, no joins, no cursors. This is a key-value store, not SQL.
 */

export interface StorageRecord {
  [key: string]: unknown;
}

export interface IndexSchema {
  name: string;
  keyPath: string;
  unique?: boolean;
}

export interface StoreSchema {
  name: string;
  keyPath: string;

  /**
   * renamedFrom — the OLD store name that should be migrated into this store.
   *
   * When this field is set AND the old store still exists in the database
   * (i.e., this is the first open after a rename), IndexedDBStorage will:
   *   1. Create the new store (if not already present — handled by the
   *      normal schema creation loop above).
   *   2. Open a cursor on the old store and copy every record into the
   *      new store.
   *   3. Delete the old store once all records have been copied.
   *
   * All of this happens inside the versionchange transaction — it is
   * fully atomic. If the page crashes mid-migration, the next open will
   * retry from scratch (the old store will still be present).
   *
   * On subsequent opens (after the first successful migration), the old
   * store no longer exists, so `db.objectStoreNames.contains(renamedFrom)`
   * returns false and no migration runs — making it idempotent.
   *
   * Example:
   *   { name: "tasks", keyPath: "id", renamedFrom: "todos" }
   *   → on v4→v5 upgrade, copies all "todos" records into "tasks"
   *     and deletes the "todos" store.
   */
  renamedFrom?: string;

  indexes?: IndexSchema[];
}

export interface StorageConfig {
  dbName: string;
  version: number;
  stores: StoreSchema[];
}

/**
 * KVStorage — the contract every storage backend must fulfill.
 *
 * Every method returns a Promise because the browser IndexedDB API
 * is asynchronous. Even MemoryStorage returns Promises (they just
 * resolve immediately) to keep the interface uniform.
 */
export interface KVStorage {
  open(): Promise<void>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  get<T = any>(storeName: string, key: string): Promise<T | undefined>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  getAll<T = any>(storeName: string): Promise<T[]>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  put(storeName: string, record: any): Promise<void>;
  delete(storeName: string, key: string): Promise<void>;
  close(): void;
}
