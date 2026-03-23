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
 * on the storage and ignores the returned Promise. This means:
 *
 *   1. The UI remains fast — dispatch returns immediately after the reducer.
 *   2. Writes happen asynchronously in the background.
 *   3. If a write fails (rare), data is lost on next page load but the
 *      current session continues working fine.
 *
 * This is a deliberate trade-off: UI responsiveness over write guarantees.
 * For a checklist app, losing one edit on a crash is acceptable. For a
 * banking app, you'd want await + error handling.
 *
 * === What gets persisted? ===
 *
 * The middleware inspects action.type to decide what to write:
 *
 *   TEMPLATE_CREATE, TEMPLATE_UPDATE → put the affected template
 *   TEMPLATE_DELETE                  → delete the template by ID
 *   INSTANCE_*                       → put the affected instance
 *   STATE_LOAD                       → no-op (data came FROM storage)
 *
 * Only changed records are written — not the entire state. This minimizes
 * I/O and makes persistence efficient even with hundreds of templates.
 */

import type { KVStorage } from "@coding-adventures/indexeddb";
import type { Middleware } from "@coding-adventures/store";
import type { AppState } from "./reducer.js";
import {
  TEMPLATE_CREATE,
  TEMPLATE_UPDATE,
  TEMPLATE_DELETE,
  INSTANCE_CREATE,
  INSTANCE_CHECK,
  INSTANCE_UNCHECK,
  INSTANCE_ANSWER,
  INSTANCE_COMPLETE,
  INSTANCE_ABANDON,
  TODO_CREATE,
  TODO_UPDATE,
  TODO_DELETE,
  TODO_TOGGLE,
} from "./actions.js";

/**
 * createPersistenceMiddleware — factory that captures the storage reference.
 *
 * The returned middleware function is called on every dispatch. It lets the
 * reducer run first (by calling next()), then inspects the new state to
 * find the affected record and write it to storage.
 */
export function createPersistenceMiddleware(
  storage: KVStorage,
): Middleware<AppState> {
  return (store, action, next) => {
    // Let the reducer process the action first
    next();

    // Now read the new state and persist the affected record
    const state = store.getState();

    switch (action.type) {
      case TEMPLATE_CREATE: {
        // The newly created template is the last one in the array
        const template = state.templates[state.templates.length - 1];
        if (template) {
          storage.put("templates", template);
        }
        break;
      }

      case TEMPLATE_UPDATE: {
        const id = action.id as string;
        const template = state.templates.find((t) => t.id === id);
        if (template) {
          storage.put("templates", template);
        }
        break;
      }

      case TEMPLATE_DELETE: {
        const id = action.id as string;
        storage.delete("templates", id);
        break;
      }

      case INSTANCE_CREATE: {
        // The newly created instance is the last one in the array
        const instance = state.instances[state.instances.length - 1];
        if (instance) {
          storage.put("instances", instance);
        }
        break;
      }

      case INSTANCE_CHECK:
      case INSTANCE_UNCHECK:
      case INSTANCE_ANSWER:
      case INSTANCE_COMPLETE:
      case INSTANCE_ABANDON: {
        const instanceId = action.instanceId as string;
        const instance = state.instances.find((i) => i.id === instanceId);
        if (instance) {
          storage.put("instances", instance);
        }
        break;
      }

      // ── Todo persistence ──────────────────────────────────────────────
      case TODO_CREATE: {
        const todo = state.todos[state.todos.length - 1];
        if (todo) {
          storage.put("todos", todo);
        }
        break;
      }

      case TODO_UPDATE:
      case TODO_TOGGLE: {
        const todoId = action.todoId as string;
        const todo = state.todos.find((t) => t.id === todoId);
        if (todo) {
          storage.put("todos", todo);
        }
        break;
      }

      case TODO_DELETE: {
        storage.delete("todos", action.todoId as string);
        break;
      }

      // STATE_LOAD: data came from storage, no need to write back
      default:
        break;
    }
  };
}
