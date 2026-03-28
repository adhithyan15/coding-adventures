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
 *   "tasks"     — Task records (renamed from "todos" in IDB v5)
 *   "views"     — SavedView records
 *   "calendars" — CalendarSettings records
 *
 * === Selective persistence ===
 *
 * The middleware inspects action.type to decide WHAT to write:
 *
 *   TASK_CREATE         → put the newly created task into "tasks"
 *   TASK_UPDATE         → put the updated task into "tasks"
 *   TASK_DELETE         → delete the task from "tasks" by ID
 *   TASK_TOGGLE_STATUS  → put the updated task into "tasks"
 *   TASK_SET_STATUS     → put the updated task into "tasks"
 *   TASK_CLEAR_COMPLETED → delete all completed tasks from "tasks"
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
import type { Project } from "./types.js";
import type { GraphEdge } from "./graph.js";
import {
  TASK_CREATE,
  TASK_UPDATE,
  TASK_DELETE,
  TASK_TOGGLE_STATUS,
  TASK_SET_STATUS,
  TASK_CLEAR_COMPLETED,
  VIEW_UPSERT,
  CALENDAR_UPSERT,
  PROJECT_UPSERT,
  EDGE_ADD,
  EDGE_REMOVE,
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

      // ── Create: persist the new task AND the auto-created "contains" edge
      case TASK_CREATE: {
        const task = state.tasks[state.tasks.length - 1];
        if (task) {
          storage.put("tasks", task).catch((err: unknown) => {
            console.warn("[persistence] Failed to persist task:", err);
          });
          // Persist the auto-created project→task edge. It's the edge whose
          // toId matches the new task's id.
          const edge = state.edges.find((e) => e.toId === task.id);
          if (edge) {
            storage.put("edges", edge).catch((err: unknown) => {
              console.warn("[persistence] Failed to persist task edge:", err);
            });
          }
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
          storage.put("tasks", task);
        }
        break;
      }

      // ── Delete: remove task from storage AND cascade-delete its edges ──
      //
      // The reducer already removed the edges from state. We find which edges
      // were deleted by reading all edges from storage and deleting those
      // whose fromId or toId matched the deleted task.
      case TASK_DELETE: {
        const taskId = action.taskId as string;
        storage.delete("tasks", taskId).catch((err: unknown) => {
          console.warn("[persistence] Failed to delete task:", err);
        });
        // Cascade: delete all edges referencing this task from IDB.
        // We can't diff old vs new state here (next() already ran), so
        // we query storage for edges matching the taskId and delete them.
        storage.getAll<GraphEdge>("edges").then((storedEdges) => {
          for (const e of storedEdges) {
            if (e.fromId === taskId || e.toId === taskId) {
              storage.delete("edges", e.id).catch((err: unknown) => {
                console.warn("[persistence] Failed to delete edge:", err);
              });
            }
          }
        }).catch((err: unknown) => {
          console.warn("[persistence] Failed to read edges for cascade delete:", err);
        });
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
        storage.getAll("tasks").then((stored) => {
          const currentIds = new Set(state.tasks.map((t) => t.id));
          for (const record of stored) {
            const storedTask = record as { id: string };
            if (!currentIds.has(storedTask.id)) {
              storage.delete("tasks", storedTask.id);
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
        storage.put("calendars", calendar).catch((err: unknown) => {
          console.warn("[persistence] Failed to persist calendar:", err);
        });
        break;
      }

      // ── Project upsert: persist the project ──────────────────────────
      case PROJECT_UPSERT: {
        const project = action.project as Project;
        storage.put("projects", project).catch((err: unknown) => {
          console.warn("[persistence] Failed to persist project:", err);
        });
        break;
      }

      // ── Edge add: persist the edge if the reducer accepted it ────────
      //
      // The reducer performs a cycle check and may silently reject the edge.
      // We verify it was accepted by checking if it's present in new state.
      case EDGE_ADD: {
        const edge = action.edge as GraphEdge;
        const edgeAccepted = state.edges.some((e) => e.id === edge.id);
        if (edgeAccepted) {
          storage.put("edges", edge).catch((err: unknown) => {
            console.warn("[persistence] Failed to persist edge:", err);
          });
        }
        break;
      }

      // ── Edge remove: delete from storage by ID ───────────────────────
      case EDGE_REMOVE: {
        const edgeId = action.edgeId as string;
        storage.delete("edges", edgeId).catch((err: unknown) => {
          console.warn("[persistence] Failed to delete edge:", err);
        });
        break;
      }

      // STATE_LOAD, VIEW_SET_ACTIVE: data came from storage or is ephemeral.
      default:
        break;
    }
  };
}
