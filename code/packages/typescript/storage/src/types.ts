/**
 * types.ts — The unified storage contract and schema definitions.
 *
 * === Design Philosophy ===
 *
 * Every storage system — IndexedDB, SQLite, Google Drive, S3, a flat file
 * on disk, a REST API — is fundamentally a blob store:
 *
 *   namespace + key → blob
 *
 * IndexedDB calls namespaces "object stores". SQLite calls them "tables".
 * Google Drive calls them "folders". S3 calls them "prefixes". But they're
 * all the same idea: a named collection of keyed records.
 *
 * This interface abstracts over all of them with four capabilities:
 *
 *   1. Connection    — open() / close()
 *   2. CRUD          — get() / getAll() / put() / delete()
 *   3. SQL Querying  — query() with real SQL, parsed and executed
 *   4. Transactions  — atomic batches with rollback on failure
 *
 * Each backend implements these four capabilities using its native
 * primitives. SQLite passes SQL straight to its engine. IndexedDB loads
 * all records and filters in memory via the sql-execution-engine. Google
 * Drive deserializes a file and does the same. Same interface, different
 * performance profiles.
 *
 * === Relationship to sql-execution-engine ===
 *
 * The query() method bridges Storage to the sql-execution-engine package.
 * The storage backend acts as a DataSource:
 *
 *   schema(tableName) → return field names for the named store
 *   scan(tableName)   → return all records from the store
 *
 * The sql-execution-engine handles WHERE, JOIN, GROUP BY, ORDER BY,
 * LIMIT/OFFSET — all in memory. Smart backends (SQLite) can bypass the
 * in-memory engine and execute SQL natively.
 */

import type { QueryResult, SqlValue } from "@coding-adventures/sql-execution-engine";

// Re-export for consumers
export type { QueryResult, SqlValue };

// ── Schema Definitions ─────────────────────────────────────────────────────

/**
 * A generic record in storage. Every record is an object with string keys.
 */
export interface StorageRecord {
  [key: string]: unknown;
}

/**
 * IndexSchema — a secondary key for lookups beyond the primary key.
 *
 * Not all backends support indexes natively. IndexedDB does. SQLite does
 * (via CREATE INDEX). Google Drive doesn't — the index is ignored.
 * Backends SHOULD honor indexes when possible and MAY ignore them when not.
 */
export interface IndexSchema {
  name: string;
  keyPath: string;
  unique?: boolean;
}

/**
 * StoreSchema — declares one collection (object store / table / folder).
 *
 * The keyPath identifies which field in each record serves as the primary
 * key. In IndexedDB this is set at store creation. In SQLite it maps to
 * the PRIMARY KEY column. In a flat file it's the filename or a JSON field.
 */
export interface StoreSchema {
  name: string;
  keyPath: string;

  /**
   * renamedFrom — the OLD store name that should be migrated into this store.
   *
   * When this field is set AND the old store still exists in the database,
   * the backend should migrate all records from the old store to the new one
   * and then remove the old store. This happens atomically during open().
   *
   * Backends that don't support atomic migrations (e.g., flat files) should
   * copy records first, verify the copy, then delete the old store.
   */
  renamedFrom?: string;

  indexes?: IndexSchema[];
}

/**
 * StorageConfig — the full database/collection schema.
 *
 * This is passed to the storage constructor. It tells the backend:
 *   - what to name the database/collection
 *   - which version to expect (for migration triggers)
 *   - which stores/tables to create
 */
export interface StorageConfig {
  dbName: string;
  version: number;
  stores: StoreSchema[];
}

// ── The Unified Storage Interface ──────────────────────────────────────────

/**
 * Storage — the universal storage contract.
 *
 * Every backend implements this single interface. The four capabilities
 * map to the four things every application needs from storage:
 *
 *   1. open/close  — lifecycle management
 *   2. CRUD        — read and write individual records by key
 *   3. query       — structured retrieval with filtering, sorting, grouping
 *   4. transaction — atomic multi-operation batches with rollback
 *
 * === Backend Implementation Guide ===
 *
 * Minimal backend (Google Drive, flat file):
 *   - CRUD: serialize/deserialize records as JSON
 *   - query: load all records, delegate to sql-execution-engine
 *   - transaction: clone state before, restore on error
 *
 * Native backend (SQLite):
 *   - CRUD: INSERT/SELECT/DELETE statements
 *   - query: pass SQL directly to the engine
 *   - transaction: BEGIN/COMMIT/ROLLBACK
 *
 * Browser backend (IndexedDB):
 *   - CRUD: IDB get/put/delete with Promise wrappers
 *   - query: getAll() then delegate to sql-execution-engine
 *   - transaction: IDB readwrite transaction (auto-abort on error)
 */
export interface Storage {
  // ── Connection ──────────────────────────────────────────────────────────

  /**
   * Open the storage connection and create/migrate stores as needed.
   *
   * Must be called before any other method. Implementations should:
   *   - Create stores that don't exist yet
   *   - Run migrations (renamedFrom) if applicable
   *   - Open connections, authenticate, etc.
   */
  open(): Promise<void>;

  /**
   * Close the storage connection and release resources.
   *
   * After close(), calling any other method is an error.
   */
  close(): void;

  // ── CRUD ────────────────────────────────────────────────────────────────

  /**
   * Retrieve a single record by key.
   *
   * @param store - The store/table/collection name.
   * @param key   - The primary key value.
   * @returns The record, or undefined if not found.
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  get<T = any>(store: string, key: string): Promise<T | undefined>;

  /**
   * Retrieve all records from a store.
   *
   * @param store - The store/table/collection name.
   * @returns All records in the store (order is backend-dependent).
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  getAll<T = any>(store: string): Promise<T[]>;

  /**
   * Create or update a record (upsert).
   *
   * The key is extracted from the record using the store's keyPath.
   * If a record with that key exists, it is replaced. Otherwise, a
   * new record is created.
   *
   * @param store  - The store/table/collection name.
   * @param record - The record to store. Must contain the keyPath field.
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  put(store: string, record: any): Promise<void>;

  /**
   * Delete a record by key.
   *
   * If no record exists with the given key, this is a no-op.
   *
   * @param store - The store/table/collection name.
   * @param key   - The primary key value to delete.
   */
  delete(store: string, key: string): Promise<void>;

  // ── SQL Querying ────────────────────────────────────────────────────────

  /**
   * Execute a SQL query against the stored data.
   *
   * The SQL string is parsed by the sql-parser and executed against
   * the storage backend acting as a DataSource. Smart backends (SQLite)
   * can bypass the in-memory engine and execute SQL natively.
   *
   * Currently supports SELECT queries only (the sql-execution-engine
   * is read-only). For mutations, use the CRUD methods.
   *
   * @param sql - A SQL SELECT statement.
   * @returns The query result with column names and rows.
   *
   * @example
   * ```typescript
   * const result = await storage.query(
   *   "SELECT front, back FROM cards WHERE deckId = 'abc' ORDER BY createdAt"
   * );
   * console.log(result.columns); // ["front", "back"]
   * console.log(result.rows);    // [{ front: "Q1", back: "A1" }, ...]
   * ```
   */
  query(sql: string): Promise<QueryResult>;

  // ── Transactions ────────────────────────────────────────────────────────

  /**
   * Execute a function atomically — all operations succeed or all roll back.
   *
   * The callback receives a Storage interface scoped to the transaction.
   * All CRUD operations inside the callback are part of the transaction.
   *
   * If the callback throws, all operations are rolled back:
   *   - IndexedDB: the IDB transaction auto-aborts
   *   - SQLite: ROLLBACK
   *   - MemoryStorage: restore the pre-transaction snapshot
   *   - Google Drive: discard the journal file
   *
   * If the callback returns normally, all operations are committed:
   *   - IndexedDB: the IDB transaction auto-commits
   *   - SQLite: COMMIT
   *   - MemoryStorage: the changes are already in place
   *   - Google Drive: apply the journal
   *
   * @param fn - An async function that performs operations on the transaction.
   * @returns The return value of the callback.
   *
   * @example
   * ```typescript
   * await storage.transaction(async (tx) => {
   *   await tx.delete("decks", deckId);
   *   await tx.delete("cards", cardId1);
   *   await tx.delete("cards", cardId2);
   *   // if any throw, everything rolls back
   * });
   * ```
   */
  transaction<T>(fn: (tx: Storage) => Promise<T>): Promise<T>;
}
