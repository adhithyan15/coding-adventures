/**
 * actions.test.ts — Unit tests for action creator functions.
 *
 * Action creators are simple factory functions. We test that they
 * produce the correct action shape (type + payload). This ensures
 * the reducer receives properly-formatted actions.
 */

import { describe, it, expect } from "vitest";
import {
  TODO_CREATE,
  TODO_UPDATE,
  TODO_DELETE,
  TODO_TOGGLE_STATUS,
  TODO_SET_STATUS,
  STATE_LOAD,
  TODO_CLEAR_COMPLETED,
  createTodoAction,
  updateTodoAction,
  deleteTodoAction,
  toggleStatusAction,
  setStatusAction,
  stateLoadAction,
  clearCompletedAction,
} from "../actions.js";
import type { TodoItem } from "../types.js";

describe("action creators", () => {
  // ── createTodoAction ────────────────────────────────────────────────────

  describe("createTodoAction", () => {
    it("creates an action with TODO_CREATE type", () => {
      const action = createTodoAction("Title", "Desc", "high", "work", "2026-04-01");
      expect(action.type).toBe(TODO_CREATE);
    });

    it("includes all user-provided fields", () => {
      const action = createTodoAction("Buy groceries", "Milk and eggs", "urgent", "shopping", "2026-03-28");
      expect(action.title).toBe("Buy groceries");
      expect(action.description).toBe("Milk and eggs");
      expect(action.priority).toBe("urgent");
      expect(action.category).toBe("shopping");
      expect(action.dueDate).toBe("2026-03-28");
    });

    it("handles null due date", () => {
      const action = createTodoAction("No deadline", "", "low", "", null);
      expect(action.dueDate).toBeNull();
    });

    it("handles empty description and category", () => {
      const action = createTodoAction("Minimal", "", "medium", "", null);
      expect(action.description).toBe("");
      expect(action.category).toBe("");
    });
  });

  // ── updateTodoAction ────────────────────────────────────────────────────

  describe("updateTodoAction", () => {
    it("creates an action with TODO_UPDATE type", () => {
      const action = updateTodoAction("id-1", { title: "New title" });
      expect(action.type).toBe(TODO_UPDATE);
    });

    it("includes todoId and patch", () => {
      const patch = { title: "Updated", priority: "high" as const };
      const action = updateTodoAction("id-2", patch);
      expect(action.todoId).toBe("id-2");
      expect(action.patch).toEqual(patch);
    });

    it("supports partial patches (single field)", () => {
      const action = updateTodoAction("id-3", { category: "personal" });
      const patch = action.patch as Partial<TodoItem>;
      expect(patch.category).toBe("personal");
      expect(patch.title).toBeUndefined();
    });
  });

  // ── deleteTodoAction ────────────────────────────────────────────────────

  describe("deleteTodoAction", () => {
    it("creates an action with TODO_DELETE type", () => {
      const action = deleteTodoAction("id-4");
      expect(action.type).toBe(TODO_DELETE);
    });

    it("includes the todoId", () => {
      const action = deleteTodoAction("some-uuid");
      expect(action.todoId).toBe("some-uuid");
    });
  });

  // ── toggleStatusAction ──────────────────────────────────────────────────

  describe("toggleStatusAction", () => {
    it("creates an action with TODO_TOGGLE_STATUS type", () => {
      const action = toggleStatusAction("id-5");
      expect(action.type).toBe(TODO_TOGGLE_STATUS);
    });

    it("includes the todoId", () => {
      const action = toggleStatusAction("toggle-me");
      expect(action.todoId).toBe("toggle-me");
    });
  });

  // ── setStatusAction ─────────────────────────────────────────────────────

  describe("setStatusAction", () => {
    it("creates an action with TODO_SET_STATUS type", () => {
      const action = setStatusAction("id-6", "done");
      expect(action.type).toBe(TODO_SET_STATUS);
    });

    it("includes todoId and target status", () => {
      const action = setStatusAction("id-7", "in-progress");
      expect(action.todoId).toBe("id-7");
      expect(action.status).toBe("in-progress");
    });

    it("works with all status values", () => {
      expect(setStatusAction("x", "todo").status).toBe("todo");
      expect(setStatusAction("x", "in-progress").status).toBe("in-progress");
      expect(setStatusAction("x", "done").status).toBe("done");
    });
  });

  // ── stateLoadAction ─────────────────────────────────────────────────────

  describe("stateLoadAction", () => {
    it("creates an action with STATE_LOAD type", () => {
      const action = stateLoadAction([]);
      expect(action.type).toBe(STATE_LOAD);
    });

    it("includes the todos array", () => {
      const todos: TodoItem[] = [
        {
          id: "loaded-1",
          title: "Loaded",
          description: "",
          status: "todo",
          priority: "medium",
          category: "",
          dueDate: null,
          createdAt: 0,
          updatedAt: 0,
          completedAt: null,
          sortOrder: 0,
        },
      ];
      const action = stateLoadAction(todos);
      expect(action.todos).toBe(todos); // same reference (not copied)
    });
  });

  // ── clearCompletedAction ────────────────────────────────────────────────

  describe("clearCompletedAction", () => {
    it("creates an action with TODO_CLEAR_COMPLETED type", () => {
      const action = clearCompletedAction();
      expect(action.type).toBe(TODO_CLEAR_COMPLETED);
    });
  });

  // ── Action type constants ───────────────────────────────────────────────

  describe("action type constants", () => {
    it("all action types are unique strings", () => {
      const types = [
        TODO_CREATE,
        TODO_UPDATE,
        TODO_DELETE,
        TODO_TOGGLE_STATUS,
        TODO_SET_STATUS,
        STATE_LOAD,
        TODO_CLEAR_COMPLETED,
      ];
      const uniqueTypes = new Set(types);
      expect(uniqueTypes.size).toBe(types.length);
    });

    it("all action types are non-empty strings", () => {
      const types = [
        TODO_CREATE,
        TODO_UPDATE,
        TODO_DELETE,
        TODO_TOGGLE_STATUS,
        TODO_SET_STATUS,
        STATE_LOAD,
        TODO_CLEAR_COMPLETED,
      ];
      for (const t of types) {
        expect(t).toBeTypeOf("string");
        expect(t.length).toBeGreaterThan(0);
      }
    });
  });
});
