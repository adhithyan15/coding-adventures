/**
 * memory-storage.ts — In-memory KVStorage implementation.
 *
 * Internally holds a Map<storeName, Map<key, record>>. Every operation
 * is synchronous but wrapped in Promise.resolve() to match the KVStorage
 * interface.
 *
 * Used for:
 *   - Unit tests (no browser APIs needed)
 *   - Environments where IndexedDB is unavailable (SSR, Node scripts)
 *   - Fallback when IndexedDB fails to open
 */

import type { KVStorage, StorageRecord, StoreSchema } from "./types.js";

export class MemoryStorage implements KVStorage {
  private stores = new Map<string, Map<string, StorageRecord>>();
  private keyPaths: Map<string, string>;
  private opened = false;

  constructor(storeSchemas: StoreSchema[]) {
    this.keyPaths = new Map();
    for (const schema of storeSchemas) {
      this.keyPaths.set(schema.name, schema.keyPath);
    }
  }

  async open(): Promise<void> {
    // Initialize empty maps for each declared store
    for (const [name] of this.keyPaths) {
      if (!this.stores.has(name)) {
        this.stores.set(name, new Map());
      }
    }
    this.opened = true;
  }

  async get<T extends StorageRecord>(storeName: string, key: string): Promise<T | undefined> {
    const store = this.stores.get(storeName);
    if (!store) throw new Error(`Unknown store: ${storeName}`);
    return store.get(key) as T | undefined;
  }

  async getAll<T extends StorageRecord>(storeName: string): Promise<T[]> {
    const store = this.stores.get(storeName);
    if (!store) throw new Error(`Unknown store: ${storeName}`);
    return Array.from(store.values()) as T[];
  }

  async put<T extends StorageRecord>(storeName: string, record: T): Promise<void> {
    const store = this.stores.get(storeName);
    const keyPath = this.keyPaths.get(storeName);
    if (!store || !keyPath) throw new Error(`Unknown store: ${storeName}`);
    const key = record[keyPath] as string;
    if (key === undefined) throw new Error(`Record missing key field "${keyPath}"`);
    store.set(key, record);
  }

  async delete(storeName: string, key: string): Promise<void> {
    const store = this.stores.get(storeName);
    if (!store) throw new Error(`Unknown store: ${storeName}`);
    store.delete(key);
  }

  close(): void {
    this.opened = false;
  }
}
