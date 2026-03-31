/**
 * memory-storage.ts — In-memory Storage implementation.
 *
 * Internally holds a Map<storeName, Map<key, record>>. Every CRUD operation
 * is synchronous but wrapped in Promise.resolve() to match the async
 * Storage interface.
 *
 * === When to use MemoryStorage ===
 *
 *   - Unit tests: no browser APIs needed, fast, deterministic
 *   - SSR / Node scripts: IndexedDB unavailable
 *   - Fallback: when the primary backend fails to open
 *   - Prototyping: get an app running before choosing a backend
 *
 * === Query support ===
 *
 * MemoryStorage implements query() by acting as a DataSource for the
 * sql-execution-engine. On each query:
 *
 *   1. Parse the SQL string via sql-parser
 *   2. Build a DataSource that maps store names to tables:
 *      - schema(name) → field names from the first record
 *      - scan(name)   → all records, values coerced to SqlValue
 *   3. Execute the parsed AST against the DataSource
 *   4. Return the QueryResult
 *
 * This is the "dumb backend" path — every record is loaded into memory
 * and the engine does the filtering. Smart backends (SQLite) bypass this
 * and execute SQL natively.
 *
 * === Transaction support ===
 *
 * Transactions use the snapshot-and-restore pattern:
 *
 *   1. Deep-clone all maps before the callback runs
 *   2. Run the callback (which calls put/delete on this instance)
 *   3. On success: do nothing (changes are already in place)
 *   4. On error: restore the cloned maps, then re-throw
 *
 * This gives us atomic rollback without any external infrastructure.
 */

import type { Storage, StoreSchema, QueryResult } from "./types.js";
import type { DataSource, Row, SqlValue } from "@coding-adventures/sql-execution-engine";
import { execute } from "@coding-adventures/sql-execution-engine";

export class MemoryStorage implements Storage {
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

  // ── SQL Querying ────────────────────────────────────────────────────────

  /**
   * Execute a SQL SELECT against the in-memory stores.
   *
   * The MemoryStorage acts as a DataSource for the sql-execution-engine.
   * Store names map to table names. Each record becomes a row.
   *
   * Values are coerced to SqlValue (string | number | boolean | null):
   *   - string, number, boolean → passed through
   *   - null, undefined → null
   *   - everything else → JSON.stringify as string
   */
  async query(sql: string): Promise<QueryResult> {
    const dataSource = this.createDataSource();
    return execute(sql, dataSource);
  }

  /**
   * Build a DataSource adapter that maps store names to table scans.
   */
  private createDataSource(): DataSource {
    return {
      schema: (tableName: string): string[] => {
        const store = this.stores.get(tableName);
        if (!store) {
          throw new Error(`Table not found: ${tableName}`);
        }
        // Infer schema from the first record's keys.
        // If the store is empty, return the keyPath as the only column.
        const firstRecord = store.values().next().value;
        if (firstRecord && typeof firstRecord === "object") {
          return Object.keys(firstRecord);
        }
        const keyPath = this.keyPaths.get(tableName);
        return keyPath ? [keyPath] : [];
      },

      scan: (tableName: string): Row[] => {
        const store = this.stores.get(tableName);
        if (!store) {
          throw new Error(`Table not found: ${tableName}`);
        }
        const rows: Row[] = [];
        for (const record of store.values()) {
          const row: Row = {};
          for (const [key, value] of Object.entries(record as Record<string, unknown>)) {
            row[key] = coerceToSqlValue(value);
          }
          rows.push(row);
        }
        return rows;
      },
    };
  }

  // ── Transactions ────────────────────────────────────────────────────────

  /**
   * Execute a function atomically with snapshot-and-restore rollback.
   *
   * Before the callback runs, we deep-clone every store's Map. If the
   * callback throws, we restore the cloned Maps. If it succeeds, the
   * changes are already in place (no commit step needed).
   */
  async transaction<T>(fn: (tx: Storage) => Promise<T>): Promise<T> {
    // Snapshot: deep-clone all store Maps
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const snapshot = new Map<string, Map<string, any>>();
    for (const [name, store] of this.stores) {
      const cloned = new Map<string, unknown>();
      for (const [key, value] of store) {
        // Deep-clone each record via structured clone (JSON round-trip)
        cloned.set(key, JSON.parse(JSON.stringify(value)));
      }
      snapshot.set(name, cloned);
    }

    try {
      // Run the callback — it operates on `this` directly
      const result = await fn(this);
      return result;
    } catch (error) {
      // Rollback: restore the snapshot
      this.stores = snapshot;
      throw error;
    }
  }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/**
 * Coerce a JavaScript value to a SqlValue.
 *
 * The sql-execution-engine expects values to be string | number | boolean | null.
 * Records in storage can contain any JS value (dates as timestamps, objects, etc.).
 * This function maps them to the closest SqlValue representation.
 */
function coerceToSqlValue(value: unknown): SqlValue {
  if (value === null || value === undefined) return null;
  if (typeof value === "string") return value;
  if (typeof value === "number") return value;
  if (typeof value === "boolean") return value;
  // Fall back to string representation for complex types
  return JSON.stringify(value);
}
