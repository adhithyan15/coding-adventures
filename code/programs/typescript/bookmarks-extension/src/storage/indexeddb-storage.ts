/**
 * IndexedDB Storage Backend
 * ==========================
 *
 * The default storage implementation. Stores bookmarks in the browser's
 * built-in IndexedDB, which means:
 * - No server needed — everything is local
 * - No permissions required — IndexedDB is available to all extensions
 * - Fast — no network round-trips
 * - Persistent — survives browser restarts
 * - Essentially unlimited storage (unlike localStorage's 5-10 MB cap)
 *
 * IndexedDB basics
 * -----------------
 * IndexedDB is a transactional, object-oriented database built into
 * every modern browser. It stores JavaScript objects directly (no
 * serialization to strings like localStorage).
 *
 * Key concepts:
 * ```
 * Database ("bookmarks-extension")
 *   └── Object Store ("bookmarks")
 *         ├── keyPath: "id"     ← primary key
 *         ├── index: "url"      ← secondary lookup
 *         └── records: [
 *               { id: "abc", url: "https://...", title: "...", ... },
 *               { id: "def", url: "https://...", title: "...", ... }
 *             ]
 * ```
 *
 * Every read or write happens inside a **transaction**. Transactions
 * ensure that if something fails midway, the database isn't left in
 * a broken state (atomicity).
 *
 * IDB's native API uses callbacks (onsuccess/onerror), which is
 * cumbersome. We wrap each operation in a Promise for clean async/await.
 */

import type {
  Bookmark,
  BookmarkCreateInput,
  BookmarkUpdateInput,
  BookmarkStorage,
} from "./bookmark-storage";

/** Default database name — unique per extension to avoid conflicts */
const DEFAULT_DB_NAME = "bookmarks-extension";

/** Schema version — bump this when you change the object store structure */
const DB_VERSION = 1;

/** Object store name — the "table" that holds our bookmarks */
const STORE_NAME = "bookmarks";

// =========================================================================
// Promise wrapper for IDB requests
// =========================================================================

/**
 * Wrap an IDB request in a Promise.
 *
 * IndexedDB's native API uses event listeners:
 * ```
 * request.onsuccess = () => { /* use request.result *\/ };
 * request.onerror   = () => { /* handle request.error *\/ };
 * ```
 *
 * This is awkward with async/await. This helper converts to:
 * ```
 * const result = await wrapRequest(store.get("key"));
 * ```
 */
function wrapRequest<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

// =========================================================================
// IndexedDB Storage Implementation
// =========================================================================

export class IndexedDBStorage implements BookmarkStorage {
  /**
   * The database connection. Null until initialize() is called.
   *
   * Why nullable? Because opening a database is async, and we can't
   * do async work in a constructor. The initialize() method opens
   * the connection and assigns it here.
   */
  private db: IDBDatabase | null = null;

  /** The database name. Configurable for test isolation. */
  private dbName: string;

  /**
   * @param dbName - Optional database name override. Defaults to
   *   "bookmarks-extension". Tests pass unique names to avoid
   *   cross-test interference in fake-indexeddb.
   */
  constructor(dbName: string = DEFAULT_DB_NAME) {
    this.dbName = dbName;
  }

  /**
   * Open (or create) the IndexedDB database.
   *
   * The `onupgradeneeded` callback runs when:
   * 1. The database doesn't exist yet (first install)
   * 2. The version number is higher than what's stored (schema migration)
   *
   * Inside onupgradeneeded, we create object stores and indexes.
   * This is the ONLY place where you can modify the database schema.
   */
  async initialize(): Promise<void> {
    return new Promise((resolve, reject) => {
      const request = indexedDB.open(this.dbName, DB_VERSION);

      request.onupgradeneeded = () => {
        const db = request.result;

        // Create the bookmarks object store if it doesn't exist.
        // keyPath: "id" means each record's `id` property is its primary key.
        if (!db.objectStoreNames.contains(STORE_NAME)) {
          const store = db.createObjectStore(STORE_NAME, { keyPath: "id" });

          // Create a unique index on URL so we can quickly look up
          // "is this URL already bookmarked?" without scanning all records.
          store.createIndex("url", "url", { unique: true });
        }
      };

      request.onsuccess = () => {
        this.db = request.result;
        resolve();
      };

      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Get the database connection, throwing if not initialized.
   *
   * This is a convenience method that avoids null checks in every
   * other method. Call initialize() before any other method.
   */
  private getDb(): IDBDatabase {
    if (!this.db) {
      throw new Error(
        "Database not initialized. Call initialize() before using storage."
      );
    }
    return this.db;
  }

  /**
   * Retrieve all bookmarks, sorted by most recently updated first.
   *
   * Uses a readonly transaction — multiple reads can happen in parallel
   * without blocking writes.
   */
  async getAll(): Promise<Bookmark[]> {
    const db = this.getDb();
    const tx = db.transaction(STORE_NAME, "readonly");
    const store = tx.objectStore(STORE_NAME);
    const bookmarks = await wrapRequest(store.getAll());

    // Sort by updatedAt descending (most recent first).
    // IndexedDB doesn't support ORDER BY like SQL, so we sort in JS.
    return bookmarks.sort(
      (a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
    );
  }

  /**
   * Find a bookmark by URL using the "url" index.
   *
   * This is O(1) because IndexedDB indexes are B-trees — the same data
   * structure databases use for fast lookups. Without the index, we'd
   * have to scan every record (O(n)).
   */
  async getByUrl(url: string): Promise<Bookmark | null> {
    const db = this.getDb();
    const tx = db.transaction(STORE_NAME, "readonly");
    const store = tx.objectStore(STORE_NAME);
    const index = store.index("url");
    const result = await wrapRequest(index.get(url));
    return result ?? null;
  }

  /**
   * Find a bookmark by its primary key (id).
   */
  async getById(id: string): Promise<Bookmark | null> {
    const db = this.getDb();
    const tx = db.transaction(STORE_NAME, "readonly");
    const store = tx.objectStore(STORE_NAME);
    const result = await wrapRequest(store.get(id));
    return result ?? null;
  }

  /**
   * Create a new bookmark.
   *
   * Generates a UUID for the id and sets both timestamps to now.
   * Uses `store.add()` (not `store.put()`) so it throws if a record
   * with the same id or URL already exists — preventing silent overwrites.
   */
  async save(input: BookmarkCreateInput): Promise<Bookmark> {
    const db = this.getDb();
    const now = new Date().toISOString();

    const bookmark: Bookmark = {
      id: crypto.randomUUID(),
      ...input,
      createdAt: now,
      updatedAt: now,
    };

    const tx = db.transaction(STORE_NAME, "readwrite");
    const store = tx.objectStore(STORE_NAME);
    await wrapRequest(store.add(bookmark));
    return bookmark;
  }

  /**
   * Update an existing bookmark's title and/or note.
   *
   * Fetches the existing record, merges the changes, bumps updatedAt,
   * and writes it back. Throws if the bookmark doesn't exist.
   *
   * Why fetch-then-put instead of just put?
   * Because we need to preserve fields the caller didn't include in
   * the update input (e.g., updating only the note should keep the
   * existing title).
   */
  async update(id: string, input: BookmarkUpdateInput): Promise<Bookmark> {
    const db = this.getDb();
    const tx = db.transaction(STORE_NAME, "readwrite");
    const store = tx.objectStore(STORE_NAME);

    const existing = await wrapRequest(store.get(id));
    if (!existing) {
      throw new Error(`Bookmark not found: ${id}`);
    }

    const updated: Bookmark = {
      ...existing,
      ...input,
      updatedAt: new Date().toISOString(),
    };

    await wrapRequest(store.put(updated));
    return updated;
  }

  /**
   * Delete a bookmark by ID.
   *
   * Idempotent — deleting a non-existent bookmark is not an error.
   * IndexedDB's `store.delete()` already behaves this way (it
   * succeeds silently if the key doesn't exist).
   */
  async delete(id: string): Promise<void> {
    const db = this.getDb();
    const tx = db.transaction(STORE_NAME, "readwrite");
    const store = tx.objectStore(STORE_NAME);
    await wrapRequest(store.delete(id));
  }
}
