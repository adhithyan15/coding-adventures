/**
 * storage.ts — Settable storage singleton.
 *
 * The storage object is created asynchronously in main.tsx (because
 * IndexedDB.open() is async). This module holds a reference that can be
 * set by main.tsx during startup and then read by any component that needs
 * to query the event log or other stores directly.
 *
 * === Pattern: Initialized Singleton ===
 *
 * The alternative would be React Context, which requires:
 *   1. A <StorageProvider> wrapping the app root.
 *   2. A useStorage() hook call in every component.
 *   3. A ContextType definition.
 *
 * For a singleton that is always available after app startup, that's
 * unnecessary overhead. The initialized singleton pattern is simpler:
 *   - main.tsx calls initStorage(s) once, before React mounts.
 *   - Any module calls getStorage() to get the instance.
 *
 * === Safety ===
 *
 * getStorage() throws if called before initStorage(). In production this
 * should never happen because main.tsx calls initStorage before rendering
 * the React tree. In tests, provide a mock storage via initStorage().
 */

import type { KVStorage } from "@coding-adventures/indexeddb";

let _storage: KVStorage | null = null;

/**
 * initStorage — called once by main.tsx after the storage is opened.
 *
 * Must be called before React mounts, so that any component using
 * getStorage() during its initial render has access to the instance.
 */
export function initStorage(s: KVStorage): void {
  _storage = s;
}

/**
 * getStorage — returns the initialized storage instance.
 *
 * @throws Error if called before initStorage() has been called.
 */
export function getStorage(): KVStorage {
  if (!_storage) throw new Error("Storage has not been initialized. Call initStorage() first.");
  return _storage;
}
