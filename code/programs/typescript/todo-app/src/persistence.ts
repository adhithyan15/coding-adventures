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
 * === Store layout ===
 *
 * The IndexedDB database has three stores:
 *   "todos"     — Task records (store name is "todos" for backward compat)
 *   "views"     — SavedView records
 *   "calendars" — CalendarSettings records
 *
 * === Selective persistence ===
 *
 * The middleware inspects action.type to decide WHAT to write:
 *
 *   TASK_CREATE         → put the newly created task into "todos"
 *   TASK_UPDATE         → put the updated task into "todos"
 *   TASK_DELETE         → delete the task from "todos" by ID
 *   TASK_TOGGLE_STATUS  → put the updated task into "todos"
 *   TASK_SET_STATUS     → put the updated task into "todos"
 *   TASK_CLEAR_COMPLETED → delete all completed tasks from "todos"
 *   VIEW_UPSERT         → put the view into "views"
 *   CALENDAR_UPSERT     → put the calendar into "calendars"
 *   VIEW_SET_ACTIVE     → no-op (activeViewId is ephemeral; reconstructed from URL)
 *   STATE_LOAD          → no-op (data came FROM storage)
 *
 * Only changed records are written — not the entire state. This minimizes
 * I/O and makes persistence efficient even with thousands of tasks.
 */

import type { KVStorage } from "@coding-adventures/indexeddb";
import type { Middleware } from "@coding-adventures/store";
import type { AppState } from "./reducer.js";
import type { SavedView } from "./views.js";
import type { CalendarSettings } from "./calendar-settings.js";
import {
  TASK_CREATE,
  TASK_UPDATE,
  TASK_DELETE,
  TASK_TOGGLE_STATUS,
  TASK_SET_STATUS,
  TASK_CLEAR_COMPLETED,
  VIEW_UPSERT,
  CALENDAR_UPSERT,
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

      // ── Create: the new task is the last item in the array ──────────
      case TASK_CREATE: {
        const task = state.tasks[state.tasks.length - 1];
        if (task) {
          storage.put("todos", task);
        }
        break;
      }

      // ── Update / Toggle / Set status: find by ID and persist ────────
      case TASK_UPDATE:
      case TASK_TOGGLE_STATUS:
      case TASK_SET_STATUS: {
        const taskId = action.taskId as string;
        const task = state.tasks.find((t) => t.id === taskId);
        if (task) {
          storage.put("todos", task);
        }
        break;
      }

      // ── Delete: remove from storage by ID ───────────────────────────
      case TASK_DELETE: {
        const taskId = action.taskId as string;
        storage.delete("todos", taskId);
        break;
      }

      // ── Clear completed: delete all done tasks from storage ─────────
      //
      // We need to know which tasks were removed. Compare the IDs in
      // the new state against the old state. But we don't have the old
      // state in middleware (next() already ran).
      //
      // Pragmatic approach: re-read all records from storage and delete
      // those not present in the current state. Fine for a small list.
      case TASK_CLEAR_COMPLETED: {
        storage.getAll("todos").then((stored) => {
          const currentIds = new Set(state.tasks.map((t) => t.id));
          for (const record of stored) {
            const storedTask = record as { id: string };
            if (!currentIds.has(storedTask.id)) {
              storage.delete("todos", storedTask.id);
            }
          }
        });
        break;
      }

      // ── View upsert: persist the view ────────────────────────────────
      case VIEW_UPSERT: {
        const view = action.view as SavedView;
        storage.put("views", view);
        break;
      }

      // ── Calendar upsert: persist the calendar ────────────────────────
      case CALENDAR_UPSERT: {
        const calendar = action.calendar as CalendarSettings;
        storage.put("calendars", calendar);
        break;
      }

      // STATE_LOAD, VIEW_SET_ACTIVE: data came from storage or is ephemeral.
      default:
        break;
    }
  };
}
