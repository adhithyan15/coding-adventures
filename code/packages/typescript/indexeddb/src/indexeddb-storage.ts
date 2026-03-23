/**
 * indexeddb-storage.ts — Promise wrapper around the raw browser IndexedDB API.
 *
 * IndexedDB is the browser's built-in persistent storage engine. It stores
 * JavaScript objects directly (no JSON.stringify needed) and supports
 * transactions for atomic reads/writes.
 *
 * The raw API is callback-based (circa 2011, pre-Promise). Every operation
 * returns an IDBRequest with onsuccess/onerror handlers. This class wraps
 * each operation in a Promise so consumers can use async/await.
 *
 * === Core IndexedDB Concepts ===
 *
 * DATABASE: A named container. Each origin (domain) can have multiple.
 *   Opened via `indexedDB.open(name, version)`.
 *
 * VERSION: An integer that triggers `onupgradeneeded` when it increases.
 *   This is the ONLY time you can create/modify object stores. Think of
 *   it like a database migration — you must declare your schema upfront.
 *
 * OBJECT STORE: A named collection of records (like a table, but without
 *   columns). Records are JS objects stored by a key path (e.g., "id").
 *
 * TRANSACTION: Every read/write happens inside a transaction. Three modes:
 *   - "readonly": can read, cannot write
 *   - "readwrite": can read and write
 *   - "versionchange": created automatically during onupgradeneeded
 *   Transactions auto-commit when all requests complete.
 *
 * INDEX: A secondary key for lookups beyond the primary key. Created on
 *   an object store during onupgradeneeded.
 *
 * REQUEST: Every store operation (get, put, delete, getAll) returns an
 *   IDBRequest. Results are available in request.onsuccess; errors in
 *   request.onerror.
 */

import type { KVStorage, StorageRecord, StorageConfig } from "./types.js";

export class IndexedDBStorage implements KVStorage {
  private db: IDBDatabase | null = null;
  private config: StorageConfig;

  constructor(config: StorageConfig) {
    this.config = config;
  }

  /**
   * open — Opens (or creates) the database.
   *
   * indexedDB.open() is the entry point to all of IndexedDB. It returns
   * an IDBOpenDBRequest with three possible outcomes:
   *
   *   1. onupgradeneeded — version is higher than what's on disk (or
   *      database doesn't exist yet). This is where we create stores.
   *
   *   2. onsuccess — database opened successfully. The IDBDatabase
   *      object is in request.result.
   *
   *   3. onerror — something went wrong (permissions, corruption, etc.)
   */
  open(): Promise<void> {
    return new Promise((resolve, reject) => {
      const request = indexedDB.open(this.config.dbName, this.config.version);

      // onupgradeneeded fires BEFORE onsuccess. This is the only place
      // where we can create or modify object stores. The transaction
      // is a special "versionchange" transaction that allows schema changes.
      request.onupgradeneeded = () => {
        const db = request.result;
        for (const schema of this.config.stores) {
          // Don't recreate existing stores (happens when upgrading versions)
          if (!db.objectStoreNames.contains(schema.name)) {
            const store = db.createObjectStore(schema.name, {
              keyPath: schema.keyPath,
            });
            // Create any secondary indexes declared in the schema
            if (schema.indexes) {
              for (const idx of schema.indexes) {
                store.createIndex(idx.name, idx.keyPath, {
                  unique: idx.unique ?? false,
                });
              }
            }
          }
        }
      };

      request.onsuccess = () => {
        this.db = request.result;
        resolve();
      };

      request.onerror = () => {
        reject(new Error(`Failed to open IndexedDB "${this.config.dbName}": ${request.error?.message}`));
      };
    });
  }

  /**
   * request — helper that opens a transaction and wraps a store operation
   * in a Promise.
   *
   * Every IndexedDB operation follows the same pattern:
   *   1. Open a transaction on the named store with the given mode
   *   2. Get the object store from the transaction
   *   3. Call the operation (get/getAll/put/delete) — returns an IDBRequest
   *   4. Wait for request.onsuccess or request.onerror
   *
   * We also listen to transaction.onerror as a fallback — if the transaction
   * itself fails (e.g., quota exceeded), the individual request's onerror
   * might not fire.
   */
  private request<T>(
    storeName: string,
    mode: IDBTransactionMode,
    fn: (store: IDBObjectStore) => IDBRequest,
  ): Promise<T> {
    return new Promise((resolve, reject) => {
      if (!this.db) {
        reject(new Error("Database not opened. Call open() first."));
        return;
      }
      const tx = this.db.transaction(storeName, mode);
      const store = tx.objectStore(storeName);
      const req = fn(store);

      req.onsuccess = () => resolve(req.result as T);
      req.onerror = () => reject(new Error(`IndexedDB request failed: ${req.error?.message}`));
      tx.onerror = () => reject(new Error(`IndexedDB transaction failed: ${tx.error?.message}`));
    });
  }

  async get<T extends StorageRecord>(storeName: string, key: string): Promise<T | undefined> {
    return this.request<T | undefined>(storeName, "readonly", (store) => store.get(key));
  }

  async getAll<T extends StorageRecord>(storeName: string): Promise<T[]> {
    return this.request<T[]>(storeName, "readonly", (store) => store.getAll());
  }

  async put<T extends StorageRecord>(storeName: string, record: T): Promise<void> {
    // put() is an upsert: inserts if key is new, replaces if key exists.
    // The key is extracted from the record using the store's keyPath.
    await this.request<void>(storeName, "readwrite", (store) => store.put(record));
  }

  async delete(storeName: string, key: string): Promise<void> {
    await this.request<void>(storeName, "readwrite", (store) => store.delete(key));
  }

  close(): void {
    if (this.db) {
      this.db.close();
      this.db = null;
    }
  }
}
