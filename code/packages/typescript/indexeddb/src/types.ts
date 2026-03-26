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
