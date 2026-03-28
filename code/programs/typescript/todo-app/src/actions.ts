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
 *
 * === Naming convention ===
 *
 * NOUN_VERB (e.g., TASK_CREATE, VIEW_UPSERT). This groups related actions
 * together alphabetically and makes the namespace clear.
 *
 * Task actions use TASK_* (previously TODO_*).
 * View actions use VIEW_*.
 * Calendar actions use CALENDAR_*.
 */

import type { Action } from "@coding-adventures/store";
import type { Task, TaskStatus, Priority } from "./types.js";
import type { SavedView } from "./views.js";
import type { CalendarSettings } from "./calendar-settings.js";

// ── Task action type constants ─────────────────────────────────────────────

/** Create a new task with title, description, priority, category, dueDate, dueTime. */
export const TASK_CREATE = "TASK_CREATE";

/** Update one or more fields of an existing task. */
export const TASK_UPDATE = "TASK_UPDATE";

/** Remove a task permanently. No soft deletes — gone is gone. */
export const TASK_DELETE = "TASK_DELETE";

/** Cycle a task's status: todo → in-progress → done → todo. */
export const TASK_TOGGLE_STATUS = "TASK_TOGGLE_STATUS";

/** Set a task's status to a specific value (used by dropdown or Kanban drag). */
export const TASK_SET_STATUS = "TASK_SET_STATUS";

/** Clear all completed tasks at once. */
export const TASK_CLEAR_COMPLETED = "TASK_CLEAR_COMPLETED";

// ── View action type constants ─────────────────────────────────────────────

/**
 * Upsert a view into the store (add if new, replace if id exists).
 * Used both for seeding built-in views and for future user-created views.
 */
export const VIEW_UPSERT = "VIEW_UPSERT";

/** Set the currently active view by id. */
export const VIEW_SET_ACTIVE = "VIEW_SET_ACTIVE";

// ── Calendar action type constants ─────────────────────────────────────────

/**
 * Upsert a CalendarSettings into the store.
 * Used for seeding the Gregorian default and future user calendars.
 */
export const CALENDAR_UPSERT = "CALENDAR_UPSERT";

// ── Bootstrap action ──────────────────────────────────────────────────────

/**
 * Bulk-load the full app state from IndexedDB on startup.
 *
 * Dispatched once during initialization to hydrate the store with
 * previously persisted data. Replaces tasks, views, calendars, and
 * sets the activeViewId.
 */
export const STATE_LOAD = "STATE_LOAD";

// ── Task action creator functions ──────────────────────────────────────────

/**
 * createTaskAction — builds an action to create a new task.
 *
 * The reducer assigns id, createdAt, updatedAt, completedAt, sortOrder.
 * The caller only provides user-editable fields.
 */
export function createTaskAction(
  title: string,
  description: string,
  priority: Priority,
  category: string,
  dueDate: string | null,
  dueTime: string | null = null,
): Action {
  return {
    type: TASK_CREATE,
    title,
    description,
    priority,
    category,
    dueDate,
    dueTime,
  };
}

/**
 * updateTaskAction — builds an action to update an existing task.
 *
 * Uses a "patch" pattern: only the fields present in the patch object
 * are updated. Missing fields retain their current value.
 */
export function updateTaskAction(
  taskId: string,
  patch: Partial<
    Pick<Task, "title" | "description" | "priority" | "category" | "dueDate" | "dueTime">
  >,
): Action {
  return { type: TASK_UPDATE, taskId, patch };
}

/** deleteTaskAction — removes a task by ID. */
export function deleteTaskAction(taskId: string): Action {
  return { type: TASK_DELETE, taskId };
}

/**
 * toggleStatusAction — cycles the status: todo → in-progress → done → todo.
 *
 * This is the most common status change — triggered by clicking the
 * checkbox or status icon on a task card.
 */
export function toggleStatusAction(taskId: string): Action {
  return { type: TASK_TOGGLE_STATUS, taskId };
}

/**
 * setStatusAction — sets a specific status value.
 *
 * Used when the user explicitly picks a status from a dropdown or
 * drags a card between Kanban columns.
 */
export function setStatusAction(taskId: string, status: TaskStatus): Action {
  return { type: TASK_SET_STATUS, taskId, status };
}

/** clearCompletedAction — removes all tasks with status "done". */
export function clearCompletedAction(): Action {
  return { type: TASK_CLEAR_COMPLETED };
}

// ── View action creator functions ──────────────────────────────────────────

/**
 * upsertViewAction — adds or replaces a SavedView in the store.
 *
 * If a view with the same id already exists, it is replaced entirely.
 * This is used both during initial seeding and future user edits.
 */
export function upsertViewAction(view: SavedView): Action {
  return { type: VIEW_UPSERT, view };
}

/**
 * setActiveViewAction — sets the currently selected view tab.
 *
 * The active view id is stored in app state so that navigating back
 * to #/ restores the last-used view rather than always defaulting
 * to the first tab.
 */
export function setActiveViewAction(viewId: string): Action {
  return { type: VIEW_SET_ACTIVE, viewId };
}

// ── Calendar action creator functions ─────────────────────────────────────

/**
 * upsertCalendarAction — adds or replaces a CalendarSettings in the store.
 *
 * Used during seeding and future user calendar creation.
 */
export function upsertCalendarAction(calendar: CalendarSettings): Action {
  return { type: CALENDAR_UPSERT, calendar };
}

// ── Bootstrap action creator ──────────────────────────────────────────────

/**
 * stateLoadAction — hydrates the store from IndexedDB on startup.
 *
 * Called once in main.tsx after loading tasks, views, and calendars.
 * activeViewId is set to the first view's id if no preference is stored.
 */
export function stateLoadAction(
  tasks: Task[],
  views: SavedView[],
  calendars: CalendarSettings[],
  activeViewId: string,
): Action {
  return { type: STATE_LOAD, tasks, views, calendars, activeViewId };
}

// ── Legacy aliases (for backward compatibility during the transition) ──────
//
// These are kept temporarily so any test or component that still uses the
// old TODO_* names will compile. Remove in the next major cleanup.

/** @deprecated Use TASK_CREATE */
export const TODO_CREATE = TASK_CREATE;
/** @deprecated Use TASK_UPDATE */
export const TODO_UPDATE = TASK_UPDATE;
/** @deprecated Use TASK_DELETE */
export const TODO_DELETE = TASK_DELETE;
/** @deprecated Use TASK_TOGGLE_STATUS */
export const TODO_TOGGLE_STATUS = TASK_TOGGLE_STATUS;
/** @deprecated Use TASK_SET_STATUS */
export const TODO_SET_STATUS = TASK_SET_STATUS;
/** @deprecated Use TASK_CLEAR_COMPLETED */
export const TODO_CLEAR_COMPLETED = TASK_CLEAR_COMPLETED;

/** @deprecated Use createTaskAction */
export function createTodoAction(
  title: string,
  description: string,
  priority: Priority,
  category: string,
  dueDate: string | null,
): Action {
  return createTaskAction(title, description, priority, category, dueDate, null);
}

/** @deprecated Use updateTaskAction */
export function updateTodoAction(
  todoId: string,
  patch: Partial<Pick<Task, "title" | "description" | "priority" | "category" | "dueDate">>,
): Action {
  return updateTaskAction(todoId, patch);
}

/** @deprecated Use deleteTaskAction */
export function deleteTodoAction(todoId: string): Action {
  return deleteTaskAction(todoId);
}

/** @deprecated Use stateLoadAction with full signature */
export function stateLoadLegacyAction(tasks: Task[]): Action {
  return { type: STATE_LOAD, tasks, views: [], calendars: [], activeViewId: "" };
}
