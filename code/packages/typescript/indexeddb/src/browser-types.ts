/**
 * Lightweight browser-only storage types.
 *
 * The IndexedDB and CRUD-only memory backends need only schema metadata and
 * five key/value operations. Keeping that contract here lets browser bundles
 * use those backends without resolving the full SQL-capable storage package.
 */

export interface IndexSchema {
  name: string;
  keyPath: string;
  unique?: boolean;
}

export interface StoreSchema {
  name: string;
  keyPath: string;
  renamedFrom?: string;
  indexes?: IndexSchema[];
}

export interface StorageConfig {
  dbName: string;
  version: number;
  stores: StoreSchema[];
}

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
