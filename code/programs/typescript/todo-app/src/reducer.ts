/**
 * reducer.ts — Pure state reducer for the todo app.
 *
 * A reducer is a pure function: (state, action) → newState. It takes the
 * current application state and an action describing what happened, then
 * returns a NEW state object reflecting the change. The reducer never
 * mutates the existing state — it creates fresh objects via spread syntax.
 *
 * === Why immutability? ===
 *
 * React (via useSyncExternalStore) compares state snapshots by reference
 * (Object.is). If the reducer mutated the existing state object, React
 * wouldn't detect the change and wouldn't re-render. Returning a new
 * object with `{ ...state, tasks: [...] }` creates a new reference,
 * which React sees as "state changed, re-render."
 *
 * === State shape ===
 *
 * The entire app state is a single object:
 *   {
 *     tasks: Task[];               — persisted task records
 *     views: SavedView[];          — persisted named views
 *     calendars: CalendarSettings[]; — persisted calendar configs
 *     activeViewId: string;        — which view tab is currently open
 *   }
 *
 * ONLY PERSISTED data lives in the store. Ephemeral UI state (search input
 * contents, hover state, etc.) stays in component-local useState.
 */

import type { Action } from "@coding-adventures/store";
import type { Task, TaskStatus } from "./types.js";
import type { SavedView } from "./views.js";
import type { CalendarSettings } from "./calendar-settings.js";
import {
  TASK_CREATE,
  TASK_UPDATE,
  TASK_DELETE,
  TASK_TOGGLE_STATUS,
  TASK_SET_STATUS,
  STATE_LOAD,
  TASK_CLEAR_COMPLETED,
  VIEW_UPSERT,
  VIEW_SET_ACTIVE,
  CALENDAR_UPSERT,
} from "./actions.js";

// ── State ─────────────────────────────────────────────────────────────────

export interface AppState {
  tasks: Task[];
  views: SavedView[];
  calendars: CalendarSettings[];
  /**
   * The id of the currently displayed view.
   * Set on first load to the "All Tasks" view id, then updated whenever
   * the user clicks a different view tab.
   */
  activeViewId: string;
}

// ── Status cycle ──────────────────────────────────────────────────────────
//
// The toggle action cycles through statuses in order:
//   todo → in-progress → done → todo
//
// This function encapsulates that cycle so the reducer doesn't need
// a multi-branch if/else.

function nextStatus(current: TaskStatus): TaskStatus {
  switch (current) {
    case "todo":        return "in-progress";
    case "in-progress": return "done";
    case "done":        return "todo";
  }
}

// ── Reducer ───────────────────────────────────────────────────────────────

export function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {

    // ── TASK_CREATE ──────────────────────────────────────────────────────
    //
    // Generate a unique ID via crypto.randomUUID(). Set timestamps to now.
    // SortOrder uses Date.now() so new items appear at the bottom of lists.
    case TASK_CREATE: {
      const now = Date.now();
      const newTask: Task = {
        id: crypto.randomUUID(),
        title: action.title as string,
        description: (action.description as string) || "",
        status: "todo",
        priority: (action.priority as Task["priority"]) || "medium",
        category: (action.category as string) || "",
        dueDate: (action.dueDate as string | null) || null,
        dueTime: (action.dueTime as string | null) || null,
        createdAt: now,
        updatedAt: now,
        completedAt: null,
        sortOrder: now,
      };
      return { ...state, tasks: [...state.tasks, newTask] };
    }

    // ── TASK_UPDATE ──────────────────────────────────────────────────────
    //
    // Find the task by ID, apply the patch (partial update), bump updatedAt.
    // If the ID doesn't match, the item passes through unchanged.
    case TASK_UPDATE: {
      const taskId = action.taskId as string;
      const patch = action.patch as Partial<Task>;
      return {
        ...state,
        tasks: state.tasks.map((task) =>
          task.id === taskId
            ? { ...task, ...patch, updatedAt: Date.now() }
            : task,
        ),
      };
    }

    // ── TASK_DELETE ──────────────────────────────────────────────────────
    //
    // Remove by ID. filter() returns a new array, satisfying immutability.
    case TASK_DELETE: {
      const taskId = action.taskId as string;
      return {
        ...state,
        tasks: state.tasks.filter((task) => task.id !== taskId),
      };
    }

    // ── TASK_TOGGLE_STATUS ───────────────────────────────────────────────
    //
    // Cycle: todo → in-progress → done → todo.
    // When transitioning TO "done", set completedAt. When leaving "done",
    // clear completedAt.
    case TASK_TOGGLE_STATUS: {
      const taskId = action.taskId as string;
      return {
        ...state,
        tasks: state.tasks.map((task) => {
          if (task.id !== taskId) return task;
          const newStatus = nextStatus(task.status);
          return {
            ...task,
            status: newStatus,
            completedAt: newStatus === "done" ? Date.now() : null,
            updatedAt: Date.now(),
          };
        }),
      };
    }

    // ── TASK_SET_STATUS ──────────────────────────────────────────────────
    //
    // Explicit status set (not a cycle). Used by dropdown or drag-and-drop.
    case TASK_SET_STATUS: {
      const taskId = action.taskId as string;
      const status = action.status as TaskStatus;
      return {
        ...state,
        tasks: state.tasks.map((task) => {
          if (task.id !== taskId) return task;
          return {
            ...task,
            status,
            completedAt: status === "done" ? Date.now() : null,
            updatedAt: Date.now(),
          };
        }),
      };
    }

    // ── TASK_CLEAR_COMPLETED ─────────────────────────────────────────────
    //
    // Remove all tasks with status "done". A cleanup operation.
    case TASK_CLEAR_COMPLETED: {
      return {
        ...state,
        tasks: state.tasks.filter((task) => task.status !== "done"),
      };
    }

    // ── VIEW_UPSERT ──────────────────────────────────────────────────────
    //
    // Add a new view or replace an existing one (matched by id).
    // Views are sorted by sortOrder for display.
    case VIEW_UPSERT: {
      const view = action.view as SavedView;
      const existing = state.views.findIndex((v) => v.id === view.id);
      const newViews =
        existing >= 0
          ? state.views.map((v) => (v.id === view.id ? view : v))
          : [...state.views, view];
      return { ...state, views: newViews };
    }

    // ── VIEW_SET_ACTIVE ──────────────────────────────────────────────────
    //
    // Update the currently displayed view. Components read activeViewId
    // to know which view tab is selected.
    case VIEW_SET_ACTIVE: {
      const viewId = action.viewId as string;
      return { ...state, activeViewId: viewId };
    }

    // ── CALENDAR_UPSERT ──────────────────────────────────────────────────
    //
    // Add a new CalendarSettings or replace an existing one (matched by id).
    case CALENDAR_UPSERT: {
      const calendar = action.calendar as CalendarSettings;
      const existing = state.calendars.findIndex((c) => c.id === calendar.id);
      const newCalendars =
        existing >= 0
          ? state.calendars.map((c) => (c.id === calendar.id ? calendar : c))
          : [...state.calendars, calendar];
      return { ...state, calendars: newCalendars };
    }

    // ── STATE_LOAD ───────────────────────────────────────────────────────
    //
    // Replace the entire state with data from IndexedDB. Called once on
    // startup. No merge logic — IndexedDB is the source of truth.
    case STATE_LOAD: {
      const tasks = action.tasks as Task[];
      const views = (action.views as SavedView[]) || [];
      const calendars = (action.calendars as CalendarSettings[]) || [];
      const activeViewId = (action.activeViewId as string) || "";
      return { tasks, views, calendars, activeViewId };
    }

    // ── Unknown action ───────────────────────────────────────────────────
    //
    // Return state unchanged. Unknown actions must be no-ops because
    // middleware may dispatch internal actions the reducer doesn't handle.
    default:
      return state;
  }
}
