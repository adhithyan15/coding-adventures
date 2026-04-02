/**
 * persistence.ts — Storage persistence middleware.
 *
 * After each action runs through the reducer, this middleware writes the
 * affected records to the Storage backend. It uses a fire-and-forget
 * approach: it calls put/delete without awaiting the Promise. This keeps
 * the UI responsive — dispatch returns immediately after the reducer runs.
 *
 * === Storage abstraction ===
 *
 * This middleware accepts a KVStorage instance — the same interface used
 * by both IndexedDBStorage (browser) and MemoryStorage (tests/fallback).
 * It never knows which backend is active. Tomorrow the backend could be
 * Google Drive, SQLite, or S3 — no changes needed here.
 *
 * === What gets persisted ===
 *
 *   ENTRY_CREATE   → put the new entry
 *   ENTRY_UPDATE   → put the updated entry
 *   ENTRY_DELETE   → delete the entry by id
 *   ENTRIES_LOAD   → no-op (data came FROM storage)
 */

import type { KVStorage } from "@coding-adventures/indexeddb";
import type { Middleware } from "@coding-adventures/store";
import type { AppState } from "./reducer.js";
import {
  ENTRY_CREATE,
  ENTRY_UPDATE,
  ENTRY_DELETE,
} from "./actions.js";

export function createPersistenceMiddleware(
  storage: KVStorage,
): Middleware<AppState> {
  return (store, action, next) => {
    // Run the reducer first — the middleware operates on post-reducer state
    next();

    const state = store.getState();

    switch (action.type) {
      case ENTRY_CREATE: {
        const entry = state.entries[state.entries.length - 1];
        if (entry) storage.put("entries", entry);
        break;
      }

      case ENTRY_UPDATE: {
        const id = action.id as string;
        const entry = state.entries.find((e) => e.id === id);
        if (entry) storage.put("entries", entry);
        break;
      }

      case ENTRY_DELETE: {
        const id = action.id as string;
        storage.delete("entries", id);
        break;
      }

      // ENTRIES_LOAD: data came from storage — nothing to write back
      default:
        break;
    }
  };
}
