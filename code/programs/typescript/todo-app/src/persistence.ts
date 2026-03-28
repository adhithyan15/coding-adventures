/**
 * persistence.ts — IndexedDB persistence middleware.
 *
 * This middleware sits in the dispatch pipeline between actions and the
 * reducer. After the reducer processes each action, the middleware writes
 * the affected records to IndexedDB storage.
 *
 * === Fire-and-forget persistence ===
 *
 * The middleware does NOT await the IndexedDB write. It calls put/delete
 * on the storage and lets the Promise resolve in the background. This means:
 *
 *   1. The UI remains fast — dispatch returns immediately after the reducer.
 *   2. Writes happen asynchronously in the background.
 *   3. If a write fails (extremely rare), data is lost on next page load
 *      but the current session continues working fine.
 *
 * This is a deliberate trade-off: UI responsiveness over write guarantees.
 * For a todo app, losing one edit on a crash is acceptable. For a banking
 * app, you'd want await + error handling + retry.
 *
 * === Selective persistence ===
 *
 * The middleware inspects action.type to decide WHAT to write:
 *
 *   TODO_CREATE         → put the newly created todo
 *   TODO_UPDATE         → put the updated todo
 *   TODO_DELETE         → delete the todo by ID
 *   TODO_TOGGLE_STATUS  → put the updated todo
 *   TODO_SET_STATUS     → put the updated todo
 *   TODO_CLEAR_COMPLETED → delete all completed todos
 *   STATE_LOAD          → no-op (data came FROM storage)
 *
 * Only changed records are written — not the entire state. This minimizes
 * I/O and makes persistence efficient even with thousands of todos.
 */

import type { KVStorage } from "@coding-adventures/indexeddb";
import type { Middleware } from "@coding-adventures/store";
import type { AppState } from "./reducer.js";
import {
  TODO_CREATE,
  TODO_UPDATE,
  TODO_DELETE,
  TODO_TOGGLE_STATUS,
  TODO_SET_STATUS,
  TODO_CLEAR_COMPLETED,
} from "./actions.js";

/**
 * createPersistenceMiddleware — factory that captures the storage reference.
 *
 * The returned middleware function is called on every dispatch. It lets the
 * reducer run first (by calling next()), then inspects the new state to
 * find the affected record and write it to storage.
 *
 * Why a factory? Because middleware needs a reference to the storage instance,
 * which is created asynchronously during app initialization. The factory
 * captures the storage reference in a closure.
 */
export function createPersistenceMiddleware(
  storage: KVStorage,
): Middleware<AppState> {
  return (store, action, next) => {
    // Let the reducer process the action first. After next() returns,
    // store.getState() reflects the NEW state.
    next();

    // Now read the new state and persist the affected record(s).
    const state = store.getState();

    switch (action.type) {
      // ── Create: the new todo is the last item in the array ────────
      case TODO_CREATE: {
        const todo = state.todos[state.todos.length - 1];
        if (todo) {
          storage.put("todos", todo);
        }
        break;
      }

      // ── Update / Toggle / Set status: find by ID and persist ──────
      case TODO_UPDATE:
      case TODO_TOGGLE_STATUS:
      case TODO_SET_STATUS: {
        const todoId = action.todoId as string;
        const todo = state.todos.find((t) => t.id === todoId);
        if (todo) {
          storage.put("todos", todo);
        }
        break;
      }

      // ── Delete: remove from storage by ID ─────────────────────────
      case TODO_DELETE: {
        const todoId = action.todoId as string;
        storage.delete("todos", todoId);
        break;
      }

      // ── Clear completed: delete all done todos from storage ───────
      //
      // We need to know which todos were removed. Compare the IDs in
      // the new state against the old state. But we don't have the old
      // state in middleware (next() already ran). Instead, we track
      // the remaining IDs and delete everything else.
      //
      // Simpler approach: we re-read all records from storage and delete
      // those with status "done". But that would require awaiting.
      //
      // Simplest approach: before calling next(), snapshot the done IDs.
      // But middleware runs AFTER next() in our pattern.
      //
      // Pragmatic approach: Since we're fire-and-forget, we'll re-write
      // ALL remaining todos. This is fine for a todo app (typically < 100
      // items). A more sophisticated approach would use a pre-dispatch hook.
      case TODO_CLEAR_COMPLETED: {
        // Read all from storage, delete the ones not in current state
        storage.getAll("todos").then((stored) => {
          const currentIds = new Set(state.todos.map((t) => t.id));
          for (const record of stored) {
            const storedTodo = record as { id: string };
            if (!currentIds.has(storedTodo.id)) {
              storage.delete("todos", storedTodo.id);
            }
          }
        });
        break;
      }

      // STATE_LOAD: data came from storage, no need to write back.
      default:
        break;
    }
  };
}
