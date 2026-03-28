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
 * object with `{ ...state, todos: [...] }` creates a new reference,
 * which React sees as "state changed, re-render."
 *
 * === State shape ===
 *
 * The entire app state is a single object:
 *   { todos: TodoItem[] }
 *
 * That's it. Filters, search, sort — those are local component state, not
 * global store state. Only PERSISTED data lives in the store. This keeps
 * the store lean and the reducer simple.
 */

import type { Action } from "@coding-adventures/store";
import type { TodoItem, TodoStatus } from "./types.js";
import {
  TODO_CREATE,
  TODO_UPDATE,
  TODO_DELETE,
  TODO_TOGGLE_STATUS,
  TODO_SET_STATUS,
  STATE_LOAD,
  TODO_CLEAR_COMPLETED,
} from "./actions.js";

// ── State ─────────────────────────────────────────────────────────────────

export interface AppState {
  todos: TodoItem[];
}

// ── Status cycle ──────────────────────────────────────────────────────────
//
// The toggle action cycles through statuses in order:
//   todo → in-progress → done → todo
//
// This function encapsulates that cycle so the reducer doesn't need
// a multi-branch if/else.

function nextStatus(current: TodoStatus): TodoStatus {
  switch (current) {
    case "todo": return "in-progress";
    case "in-progress": return "done";
    case "done": return "todo";
  }
}

// ── Reducer ───────────────────────────────────────────────────────────────

export function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    // ── Create ──────────────────────────────────────────────────────────
    //
    // Generate a unique ID via crypto.randomUUID() (available in all modern
    // browsers and Node 19+). Set timestamps to now. SortOrder uses Date.now()
    // so new items appear at the bottom of the list.
    case TODO_CREATE: {
      const now = Date.now();
      const newTodo: TodoItem = {
        id: crypto.randomUUID(),
        title: action.title as string,
        description: (action.description as string) || "",
        status: "todo",
        priority: (action.priority as TodoItem["priority"]) || "medium",
        category: (action.category as string) || "",
        dueDate: (action.dueDate as string | null) || null,
        createdAt: now,
        updatedAt: now,
        completedAt: null,
        sortOrder: now,
      };
      return { ...state, todos: [...state.todos, newTodo] };
    }

    // ── Update ──────────────────────────────────────────────────────────
    //
    // Find the todo by ID, apply the patch (partial update), bump updatedAt.
    // If the ID doesn't match, the item passes through unchanged.
    case TODO_UPDATE: {
      const todoId = action.todoId as string;
      const patch = action.patch as Partial<TodoItem>;
      return {
        ...state,
        todos: state.todos.map((todo) =>
          todo.id === todoId
            ? { ...todo, ...patch, updatedAt: Date.now() }
            : todo,
        ),
      };
    }

    // ── Delete ──────────────────────────────────────────────────────────
    //
    // Remove by ID. filter() returns a new array, satisfying immutability.
    case TODO_DELETE: {
      const todoId = action.todoId as string;
      return {
        ...state,
        todos: state.todos.filter((todo) => todo.id !== todoId),
      };
    }

    // ── Toggle status ───────────────────────────────────────────────────
    //
    // Cycle: todo → in-progress → done → todo.
    // When transitioning TO "done", set completedAt. When leaving "done",
    // clear completedAt.
    case TODO_TOGGLE_STATUS: {
      const todoId = action.todoId as string;
      return {
        ...state,
        todos: state.todos.map((todo) => {
          if (todo.id !== todoId) return todo;
          const newStatus = nextStatus(todo.status);
          return {
            ...todo,
            status: newStatus,
            completedAt: newStatus === "done" ? Date.now() : null,
            updatedAt: Date.now(),
          };
        }),
      };
    }

    // ── Set status ──────────────────────────────────────────────────────
    //
    // Explicit status set (not a cycle). Used by dropdown or drag-and-drop.
    case TODO_SET_STATUS: {
      const todoId = action.todoId as string;
      const status = action.status as TodoStatus;
      return {
        ...state,
        todos: state.todos.map((todo) => {
          if (todo.id !== todoId) return todo;
          return {
            ...todo,
            status,
            completedAt: status === "done" ? Date.now() : null,
            updatedAt: Date.now(),
          };
        }),
      };
    }

    // ── Load from persistence ───────────────────────────────────────────
    //
    // Replace the entire todos array with data from IndexedDB. This is
    // called once on startup. No merge logic — IndexedDB is the source
    // of truth.
    case STATE_LOAD: {
      const todos = action.todos as TodoItem[];
      return { ...state, todos };
    }

    // ── Clear completed ─────────────────────────────────────────────────
    //
    // Remove all todos with status "done". A cleanup operation.
    case TODO_CLEAR_COMPLETED: {
      return {
        ...state,
        todos: state.todos.filter((todo) => todo.status !== "done"),
      };
    }

    // ── Unknown action ──────────────────────────────────────────────────
    //
    // Return state unchanged. This is important: unknown actions should
    // be a no-op, not an error. Middleware may dispatch internal actions
    // that the reducer doesn't need to handle.
    default:
      return state;
  }
}
