/**
 * memory-storage.ts — Lightweight in-memory KVStorage implementation.
 *
 * This is a CRUD-only MemoryStorage for browser fallback scenarios:
 * when IndexedDB is unavailable (private browsing, SSR, etc.), the app
 * falls back to this in-memory store. It works identically for CRUD
 * operations, just loses persistence on reload.
 *
 * For the full Storage implementation with SQL querying and transactions,
 * use MemoryStorage from @coding-adventures/storage instead.
 *
 * === Why this exists separately from @coding-adventures/storage ===
 *
 * The storage package's MemoryStorage imports sql-execution-engine and
 * sql-parser, which depend on Node.js built-ins (fs, path, url) for
 * grammar file loading. When Vite bundles a browser app, it follows
 * these imports and fails because Node.js modules can't be resolved
 * in a browser context. By keeping this CRUD-only version in the
 * indexeddb package, browser apps get a working fallback without
 * pulling in the sql chain.
 */

import type { KVStorage, StoreSchema } from "./types.js";

export class MemoryStorage implements KVStorage {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private stores = new Map<string, Map<string, any>>();
  private keyPaths: Map<string, string>;

  constructor(storeSchemas: StoreSchema[]) {
    this.keyPaths = new Map();
    for (const schema of storeSchemas) {
      this.keyPaths.set(schema.name, schema.keyPath);
    }
  }

  // ── Connection ──────────────────────────────────────────────────────────

  async open(): Promise<void> {
    for (const [name] of this.keyPaths) {
      if (!this.stores.has(name)) {
        this.stores.set(name, new Map());
      }
    }
  }

  close(): void {
    // No-op for in-memory storage
  }

  // ── CRUD ────────────────────────────────────────────────────────────────

  async get<T = unknown>(storeName: string, key: string): Promise<T | undefined> {
    const store = this.stores.get(storeName);
    if (!store) throw new Error(`Unknown store: ${storeName}`);
    return store.get(key) as T | undefined;
  }

  async getAll<T = unknown>(storeName: string): Promise<T[]> {
    const store = this.stores.get(storeName);
    if (!store) throw new Error(`Unknown store: ${storeName}`);
    return Array.from(store.values()) as T[];
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  async put(storeName: string, record: any): Promise<void> {
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
}
