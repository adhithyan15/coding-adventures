/**
 * Storage Factory
 * ================
 *
 * This is the single place that decides which storage backend to use.
 *
 * Today, it returns IndexedDBStorage (local browser storage).
 * To switch to Google Drive in the future, change one line:
 *
 * ```typescript
 * // Before:
 * return new IndexedDBStorage();
 *
 * // After:
 * return new GoogleDriveStorage();
 * ```
 *
 * Consumer code (the popup, tests) imports `createStorage()` from here
 * and never needs to know which concrete class it got back.
 */

import { IndexedDBStorage } from "./indexeddb-storage";
import type { BookmarkStorage } from "./bookmark-storage";

// Re-export types so consumers can import everything from one place
export type { Bookmark, BookmarkCreateInput, BookmarkUpdateInput, BookmarkStorage } from "./bookmark-storage";

/**
 * Create and return the active storage backend.
 *
 * Call initialize() on the returned storage before using it:
 * ```typescript
 * const storage = createStorage();
 * await storage.initialize();
 * await storage.save({ url: "...", title: "...", note: "..." });
 * ```
 */
export function createStorage(): BookmarkStorage {
  return new IndexedDBStorage();
}
