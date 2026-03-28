/**
 * actions.ts — Action types and action creator functions.
 *
 * In the Flux architecture, ACTIONS are the only way to change state.
 * An action is a plain object with a `type` field (a string constant)
 * and zero or more payload fields carrying the data needed to perform
 * the state transition.
 *
 * === Why constants instead of raw strings? ===
 *
 * Each action type is a named constant. This provides two benefits:
 *   1. Typos become compile errors (not silent no-ops)
 *   2. The reducer's switch statement gets exhaustive checking
 *
 * === Why action creators instead of inline objects? ===
 *
 * Action creators encapsulate the shape of each action. If the payload
 * changes, only the creator and reducer need updating — not every
 * component that dispatches the action. They also provide autocomplete.
 */

import type { Action } from "@coding-adventures/store";
import type { TodoItem, TodoStatus, Priority } from "./types.js";

// ── Action type constants ──────────────────────────────────────────────────
//
// Naming convention: NOUN_VERB (e.g., TODO_CREATE, not CREATE_TODO).
// This groups related actions together alphabetically.

/** Create a new todo item with title, description, priority, category, dueDate. */
export const TODO_CREATE = "TODO_CREATE";

/** Update one or more fields of an existing todo. */
export const TODO_UPDATE = "TODO_UPDATE";

/** Remove a todo permanently. No soft deletes — gone is gone. */
export const TODO_DELETE = "TODO_DELETE";

/** Cycle a todo's status: todo → in-progress → done → todo. */
export const TODO_TOGGLE_STATUS = "TODO_TOGGLE_STATUS";

/** Set a todo's status to a specific value (used by Kanban drag). */
export const TODO_SET_STATUS = "TODO_SET_STATUS";

/**
 * Bulk-load all todos from IndexedDB on startup.
 *
 * Dispatched once during initialization to hydrate the store with
 * previously persisted data. Replaces the entire todos array.
 */
export const STATE_LOAD = "STATE_LOAD";

/** Clear all completed todos at once. */
export const TODO_CLEAR_COMPLETED = "TODO_CLEAR_COMPLETED";

// ── Action creator functions ──────────────────────────────────────────────

/**
 * createTodoAction — builds an action to create a new todo.
 *
 * The reducer assigns the id, createdAt, updatedAt, completedAt, and
 * sortOrder fields. The caller only provides user-editable fields.
 */
export function createTodoAction(
  title: string,
  description: string,
  priority: Priority,
  category: string,
  dueDate: string | null,
): Action {
  return { type: TODO_CREATE, title, description, priority, category, dueDate };
}

/**
 * updateTodoAction — builds an action to update an existing todo.
 *
 * Uses a "patch" pattern: only the fields present in the patch object
 * are updated. Missing fields retain their current value.
 */
export function updateTodoAction(
  todoId: string,
  patch: Partial<Pick<TodoItem, "title" | "description" | "priority" | "category" | "dueDate">>,
): Action {
  return { type: TODO_UPDATE, todoId, patch };
}

/** deleteTodoAction — removes a todo by ID. */
export function deleteTodoAction(todoId: string): Action {
  return { type: TODO_DELETE, todoId };
}

/**
 * toggleStatusAction — cycles the status: todo → in-progress → done → todo.
 *
 * This is the most common status change — triggered by clicking the
 * checkbox or status icon on a todo card.
 */
export function toggleStatusAction(todoId: string): Action {
  return { type: TODO_TOGGLE_STATUS, todoId };
}

/**
 * setStatusAction — sets a specific status value.
 *
 * Used when the user explicitly picks a status from a dropdown or
 * drags a card between Kanban columns.
 */
export function setStatusAction(todoId: string, status: TodoStatus): Action {
  return { type: TODO_SET_STATUS, todoId, status };
}

/** stateLoadAction — hydrates the store from IndexedDB on startup. */
export function stateLoadAction(todos: TodoItem[]): Action {
  return { type: STATE_LOAD, todos };
}

/** clearCompletedAction — removes all todos with status "done". */
export function clearCompletedAction(): Action {
  return { type: TODO_CLEAR_COMPLETED };
}
