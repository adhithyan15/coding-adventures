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

import type { KVStorage, StorageConfig } from "./types.js";

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
      //
      // We run in two phases:
      //
      //   Phase 1 (synchronous): Create any new stores declared in the schema.
      //     Guarded by objectStoreNames.contains() so re-running on a later
      //     upgrade is always a no-op.
      //
      //   Phase 2 (async cursor loop): For any store that declares renamedFrom,
      //     if the OLD store still exists (first upgrade after a rename), copy
      //     every record from the old store to the new store via a cursor, then
      //     delete the old store once the cursor is exhausted.
      //
      //     The versionchange transaction stays open until ALL pending requests
      //     — including cursor continuations — complete. So deleting the old
      //     store inside the cursor's onsuccess handler is safe: the transaction
      //     is still alive, and we only delete AFTER the last record is copied.
      request.onupgradeneeded = () => {
        const db = request.result;

        // ── Phase 1: Create new stores ───────────────────────────────────────
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

        // ── Phase 2: Migrate renamed stores ──────────────────────────────────
        //
        // The versionchange transaction is accessible via request.transaction.
        // It is valid for the entire duration of onupgradeneeded, including
        // all async callbacks (cursor onsuccess), until the transaction commits.
        const tx = request.transaction;
        if (!tx) return; // safety: should always be set during onupgradeneeded

        for (const schema of this.config.stores) {
          if (!schema.renamedFrom) continue;
          // Guard: only migrate if the OLD store still exists.
          // If it was already deleted on a previous version upgrade, skip.
          if (!db.objectStoreNames.contains(schema.renamedFrom)) continue;

          // Capture both store references before the cursor opens so we
          // can access them in the async callback without closure issues.
          const oldStoreName = schema.renamedFrom;
          const oldStore = tx.objectStore(oldStoreName);
          const newStore = tx.objectStore(schema.name);

          // Open a cursor on every record in the old store.
          // onsuccess fires once per record (cursor advances), then fires
          // one final time with cursor = null when exhausted.
          const cursorReq = oldStore.openCursor();
          cursorReq.onsuccess = (event) => {
            const cursor = (event.target as IDBRequest<IDBCursorWithValue | null>).result;

            if (cursor) {
              // Copy this record to the new store, then advance the cursor.
              newStore.put(cursor.value);
              cursor.continue();
            } else {
              // Cursor exhausted — all records copied. Now it is safe to
              // delete the old store. deleteObjectStore() is valid here
              // because the versionchange transaction is still active.
              db.deleteObjectStore(oldStoreName);
            }
          };
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

  async get<T = unknown>(storeName: string, key: string): Promise<T | undefined> {
    return this.request<T | undefined>(storeName, "readonly", (store) => store.get(key));
  }

  async getAll<T = unknown>(storeName: string): Promise<T[]> {
    return this.request<T[]>(storeName, "readonly", (store) => store.getAll());
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  async put(storeName: string, record: any): Promise<void> {
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
