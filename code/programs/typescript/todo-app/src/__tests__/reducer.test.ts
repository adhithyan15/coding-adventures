/**
 * reducer.test.ts — Comprehensive unit tests for the todo app reducer.
 *
 * Tests every action type and edge case. The reducer is the core of the
 * app — if it's correct, the app is correct. Pure functions are easy to
 * test: input → output, no mocks needed.
 */

import { describe, it, expect, vi } from "vitest";
import { reducer } from "../reducer.js";
import type { AppState } from "../reducer.js";
import type { TodoItem } from "../types.js";
import {
  createTodoAction,
  updateTodoAction,
  deleteTodoAction,
  toggleStatusAction,
  setStatusAction,
  stateLoadAction,
  clearCompletedAction,
} from "../actions.js";

// ── Test Helpers ────────────────────────────────────────────────────────────

function makeTodo(overrides: Partial<TodoItem> = {}): TodoItem {
  return {
    id: "test-id-1",
    title: "Test Todo",
    description: "Test description",
    status: "todo",
    priority: "medium",
    category: "test",
    dueDate: null,
    createdAt: 1000,
    updatedAt: 1000,
    completedAt: null,
    sortOrder: 1000,
    ...overrides,
  };
}

function makeState(todos: TodoItem[] = []): AppState {
  return { todos };
}

// ── Test Suites ─────────────────────────────────────────────────────────────

describe("reducer", () => {
  // ── TODO_CREATE ─────────────────────────────────────────────────────────

  describe("TODO_CREATE", () => {
    it("adds a new todo to an empty list", () => {
      // Mock crypto.randomUUID for deterministic testing
      const mockUUID = "mock-uuid-123";
      vi.stubGlobal("crypto", { randomUUID: () => mockUUID });

      const state = makeState();
      const action = createTodoAction("Buy milk", "From the store", "high", "shopping", "2026-04-01");
      const newState = reducer(state, action);

      expect(newState.todos).toHaveLength(1);

      const todo = newState.todos[0]!;
      expect(todo.id).toBe(mockUUID);
      expect(todo.title).toBe("Buy milk");
      expect(todo.description).toBe("From the store");
      expect(todo.status).toBe("todo");
      expect(todo.priority).toBe("high");
      expect(todo.category).toBe("shopping");
      expect(todo.dueDate).toBe("2026-04-01");
      expect(todo.completedAt).toBeNull();
      expect(todo.createdAt).toBeTypeOf("number");
      expect(todo.updatedAt).toBeTypeOf("number");
      expect(todo.sortOrder).toBeTypeOf("number");

      vi.unstubAllGlobals();
    });

    it("appends to existing list", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "uuid-2" });

      const existing = makeTodo({ id: "existing-1" });
      const state = makeState([existing]);
      const action = createTodoAction("Second todo", "", "low", "", null);
      const newState = reducer(state, action);

      expect(newState.todos).toHaveLength(2);
      expect(newState.todos[0]!.id).toBe("existing-1");
      expect(newState.todos[1]!.id).toBe("uuid-2");

      vi.unstubAllGlobals();
    });

    it("defaults to medium priority when empty", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "uuid-3" });

      const state = makeState();
      const action = { type: "TODO_CREATE", title: "Task", description: "", priority: "", category: "", dueDate: null };
      const newState = reducer(state, action);

      expect(newState.todos[0]!.priority).toBe("medium");

      vi.unstubAllGlobals();
    });

    it("does not mutate the original state", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "uuid-4" });

      const state = makeState();
      const action = createTodoAction("Immutable test", "", "low", "", null);
      const newState = reducer(state, action);

      expect(state.todos).toHaveLength(0);
      expect(newState.todos).toHaveLength(1);
      expect(state).not.toBe(newState);

      vi.unstubAllGlobals();
    });
  });

  // ── TODO_UPDATE ─────────────────────────────────────────────────────────

  describe("TODO_UPDATE", () => {
    it("updates title of an existing todo", () => {
      const todo = makeTodo({ id: "u-1" });
      const state = makeState([todo]);
      const action = updateTodoAction("u-1", { title: "Updated title" });
      const newState = reducer(state, action);

      expect(newState.todos[0]!.title).toBe("Updated title");
      expect(newState.todos[0]!.description).toBe("Test description"); // unchanged
    });

    it("updates multiple fields at once", () => {
      const todo = makeTodo({ id: "u-2" });
      const state = makeState([todo]);
      const action = updateTodoAction("u-2", {
        title: "New title",
        priority: "urgent",
        category: "work",
        dueDate: "2026-12-31",
      });
      const newState = reducer(state, action);

      expect(newState.todos[0]!.title).toBe("New title");
      expect(newState.todos[0]!.priority).toBe("urgent");
      expect(newState.todos[0]!.category).toBe("work");
      expect(newState.todos[0]!.dueDate).toBe("2026-12-31");
    });

    it("bumps updatedAt timestamp", () => {
      const todo = makeTodo({ id: "u-3", updatedAt: 1000 });
      const state = makeState([todo]);
      const action = updateTodoAction("u-3", { title: "Changed" });
      const newState = reducer(state, action);

      expect(newState.todos[0]!.updatedAt).toBeGreaterThan(1000);
    });

    it("leaves non-matching todos unchanged", () => {
      const todo1 = makeTodo({ id: "u-4a", title: "Keep me" });
      const todo2 = makeTodo({ id: "u-4b", title: "Change me" });
      const state = makeState([todo1, todo2]);
      const action = updateTodoAction("u-4b", { title: "Changed" });
      const newState = reducer(state, action);

      expect(newState.todos[0]!.title).toBe("Keep me");
      expect(newState.todos[1]!.title).toBe("Changed");
    });

    it("does nothing for non-existent ID", () => {
      const todo = makeTodo({ id: "u-5" });
      const state = makeState([todo]);
      const action = updateTodoAction("non-existent", { title: "Ghost" });
      const newState = reducer(state, action);

      expect(newState.todos).toHaveLength(1);
      expect(newState.todos[0]!.title).toBe("Test Todo"); // unchanged
    });
  });

  // ── TODO_DELETE ─────────────────────────────────────────────────────────

  describe("TODO_DELETE", () => {
    it("removes a todo by ID", () => {
      const todo = makeTodo({ id: "d-1" });
      const state = makeState([todo]);
      const action = deleteTodoAction("d-1");
      const newState = reducer(state, action);

      expect(newState.todos).toHaveLength(0);
    });

    it("removes only the matching todo", () => {
      const todo1 = makeTodo({ id: "d-2a" });
      const todo2 = makeTodo({ id: "d-2b" });
      const todo3 = makeTodo({ id: "d-2c" });
      const state = makeState([todo1, todo2, todo3]);
      const action = deleteTodoAction("d-2b");
      const newState = reducer(state, action);

      expect(newState.todos).toHaveLength(2);
      expect(newState.todos.map((t) => t.id)).toEqual(["d-2a", "d-2c"]);
    });

    it("does nothing for non-existent ID", () => {
      const todo = makeTodo({ id: "d-3" });
      const state = makeState([todo]);
      const action = deleteTodoAction("ghost");
      const newState = reducer(state, action);

      expect(newState.todos).toHaveLength(1);
    });

    it("handles deleting from empty list", () => {
      const state = makeState();
      const action = deleteTodoAction("any-id");
      const newState = reducer(state, action);

      expect(newState.todos).toHaveLength(0);
    });
  });

  // ── TODO_TOGGLE_STATUS ──────────────────────────────────────────────────

  describe("TODO_TOGGLE_STATUS", () => {
    it("cycles todo → in-progress", () => {
      const todo = makeTodo({ id: "t-1", status: "todo" });
      const state = makeState([todo]);
      const action = toggleStatusAction("t-1");
      const newState = reducer(state, action);

      expect(newState.todos[0]!.status).toBe("in-progress");
      expect(newState.todos[0]!.completedAt).toBeNull();
    });

    it("cycles in-progress → done", () => {
      const todo = makeTodo({ id: "t-2", status: "in-progress" });
      const state = makeState([todo]);
      const action = toggleStatusAction("t-2");
      const newState = reducer(state, action);

      expect(newState.todos[0]!.status).toBe("done");
      expect(newState.todos[0]!.completedAt).toBeTypeOf("number");
    });

    it("cycles done → todo", () => {
      const todo = makeTodo({ id: "t-3", status: "done", completedAt: 5000 });
      const state = makeState([todo]);
      const action = toggleStatusAction("t-3");
      const newState = reducer(state, action);

      expect(newState.todos[0]!.status).toBe("todo");
      expect(newState.todos[0]!.completedAt).toBeNull();
    });

    it("full cycle: todo → in-progress → done → todo", () => {
      const todo = makeTodo({ id: "t-4", status: "todo" });
      let state = makeState([todo]);

      state = reducer(state, toggleStatusAction("t-4"));
      expect(state.todos[0]!.status).toBe("in-progress");

      state = reducer(state, toggleStatusAction("t-4"));
      expect(state.todos[0]!.status).toBe("done");

      state = reducer(state, toggleStatusAction("t-4"));
      expect(state.todos[0]!.status).toBe("todo");
    });

    it("bumps updatedAt on toggle", () => {
      const todo = makeTodo({ id: "t-5", updatedAt: 1000 });
      const state = makeState([todo]);
      const action = toggleStatusAction("t-5");
      const newState = reducer(state, action);

      expect(newState.todos[0]!.updatedAt).toBeGreaterThan(1000);
    });
  });

  // ── TODO_SET_STATUS ─────────────────────────────────────────────────────

  describe("TODO_SET_STATUS", () => {
    it("sets status to done with completedAt", () => {
      const todo = makeTodo({ id: "s-1", status: "todo" });
      const state = makeState([todo]);
      const action = setStatusAction("s-1", "done");
      const newState = reducer(state, action);

      expect(newState.todos[0]!.status).toBe("done");
      expect(newState.todos[0]!.completedAt).toBeTypeOf("number");
    });

    it("clears completedAt when moving away from done", () => {
      const todo = makeTodo({ id: "s-2", status: "done", completedAt: 5000 });
      const state = makeState([todo]);
      const action = setStatusAction("s-2", "in-progress");
      const newState = reducer(state, action);

      expect(newState.todos[0]!.status).toBe("in-progress");
      expect(newState.todos[0]!.completedAt).toBeNull();
    });

    it("can set same status (idempotent)", () => {
      const todo = makeTodo({ id: "s-3", status: "in-progress" });
      const state = makeState([todo]);
      const action = setStatusAction("s-3", "in-progress");
      const newState = reducer(state, action);

      expect(newState.todos[0]!.status).toBe("in-progress");
    });
  });

  // ── STATE_LOAD ──────────────────────────────────────────────────────────

  describe("STATE_LOAD", () => {
    it("replaces all todos with loaded data", () => {
      const existing = makeTodo({ id: "old-1" });
      const state = makeState([existing]);

      const loadedTodos = [
        makeTodo({ id: "loaded-1", title: "Loaded 1" }),
        makeTodo({ id: "loaded-2", title: "Loaded 2" }),
      ];
      const action = stateLoadAction(loadedTodos);
      const newState = reducer(state, action);

      expect(newState.todos).toHaveLength(2);
      expect(newState.todos[0]!.id).toBe("loaded-1");
      expect(newState.todos[1]!.id).toBe("loaded-2");
    });

    it("handles loading empty array", () => {
      const existing = makeTodo({ id: "old-2" });
      const state = makeState([existing]);
      const action = stateLoadAction([]);
      const newState = reducer(state, action);

      expect(newState.todos).toHaveLength(0);
    });
  });

  // ── TODO_CLEAR_COMPLETED ────────────────────────────────────────────────

  describe("TODO_CLEAR_COMPLETED", () => {
    it("removes all done todos", () => {
      const todos = [
        makeTodo({ id: "cc-1", status: "todo" }),
        makeTodo({ id: "cc-2", status: "done" }),
        makeTodo({ id: "cc-3", status: "in-progress" }),
        makeTodo({ id: "cc-4", status: "done" }),
      ];
      const state = makeState(todos);
      const action = clearCompletedAction();
      const newState = reducer(state, action);

      expect(newState.todos).toHaveLength(2);
      expect(newState.todos.map((t) => t.id)).toEqual(["cc-1", "cc-3"]);
    });

    it("does nothing when no completed todos", () => {
      const todos = [
        makeTodo({ id: "cc-5", status: "todo" }),
        makeTodo({ id: "cc-6", status: "in-progress" }),
      ];
      const state = makeState(todos);
      const action = clearCompletedAction();
      const newState = reducer(state, action);

      expect(newState.todos).toHaveLength(2);
    });

    it("handles all completed (empties the list)", () => {
      const todos = [
        makeTodo({ id: "cc-7", status: "done" }),
        makeTodo({ id: "cc-8", status: "done" }),
      ];
      const state = makeState(todos);
      const action = clearCompletedAction();
      const newState = reducer(state, action);

      expect(newState.todos).toHaveLength(0);
    });
  });

  // ── Unknown actions ─────────────────────────────────────────────────────

  describe("unknown actions", () => {
    it("returns state unchanged for unknown action types", () => {
      const state = makeState([makeTodo()]);
      const action = { type: "UNKNOWN_ACTION" };
      const newState = reducer(state, action);

      expect(newState).toBe(state); // exact same reference
    });
  });

  // ── Immutability checks ─────────────────────────────────────────────────

  describe("immutability", () => {
    it("never mutates the input state object", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "imm-uuid" });

      const original = makeTodo({ id: "imm-1" });
      const state = makeState([original]);

      // Freeze the state to catch any mutations
      Object.freeze(state);
      Object.freeze(state.todos);

      // All these should work without throwing (no mutations)
      expect(() => reducer(state, createTodoAction("New", "", "low", "", null))).not.toThrow();
      expect(() => reducer(state, updateTodoAction("imm-1", { title: "Changed" }))).not.toThrow();
      expect(() => reducer(state, deleteTodoAction("imm-1"))).not.toThrow();
      expect(() => reducer(state, toggleStatusAction("imm-1"))).not.toThrow();
      expect(() => reducer(state, setStatusAction("imm-1", "done"))).not.toThrow();
      expect(() => reducer(state, clearCompletedAction())).not.toThrow();

      vi.unstubAllGlobals();
    });

    it("returns a new state reference on every mutation", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "ref-uuid" });

      const state = makeState([makeTodo({ id: "ref-1" })]);

      const s1 = reducer(state, createTodoAction("X", "", "low", "", null));
      const s2 = reducer(state, updateTodoAction("ref-1", { title: "Y" }));
      const s3 = reducer(state, deleteTodoAction("ref-1"));
      const s4 = reducer(state, toggleStatusAction("ref-1"));

      expect(s1).not.toBe(state);
      expect(s2).not.toBe(state);
      expect(s3).not.toBe(state);
      expect(s4).not.toBe(state);

      vi.unstubAllGlobals();
    });
  });
});
