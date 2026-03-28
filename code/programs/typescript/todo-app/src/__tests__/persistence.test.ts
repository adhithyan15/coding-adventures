/**
 * persistence.test.ts — Unit tests for the IndexedDB persistence middleware.
 *
 * The persistence middleware is the bridge between the in-memory store and
 * IndexedDB. These tests verify that the correct storage operations (put,
 * delete, getAll) are called for each action type.
 *
 * We mock KVStorage to test the middleware in isolation — no actual
 * IndexedDB is needed.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { createPersistenceMiddleware } from "../persistence.js";
import type { AppState } from "../reducer.js";
import type { TodoItem } from "../types.js";
import {
  TODO_CREATE,
  TODO_UPDATE,
  TODO_DELETE,
  TODO_TOGGLE_STATUS,
  TODO_SET_STATUS,
  TODO_CLEAR_COMPLETED,
  STATE_LOAD,
} from "../actions.js";

// ── Mock Storage ────────────────────────────────────────────────────────────

function createMockStorage() {
  return {
    put: vi.fn(() => Promise.resolve()),
    delete: vi.fn(() => Promise.resolve()),
    get: vi.fn(() => Promise.resolve(undefined)),
    getAll: vi.fn(() => Promise.resolve<any[]>([])),
    open: vi.fn(() => Promise.resolve()),
    close: vi.fn(),
  };
}

// ── Mock Store ──────────────────────────────────────────────────────────────

function createMockStore(state: AppState) {
  return {
    getState: () => state,
    dispatch: vi.fn(),
    subscribe: vi.fn(),
    use: vi.fn(),
  };
}

// ── Helpers ─────────────────────────────────────────────────────────────────

function makeTodo(overrides: Partial<TodoItem> = {}): TodoItem {
  return {
    id: "test-id",
    title: "Test",
    description: "",
    status: "todo",
    priority: "medium",
    category: "",
    dueDate: null,
    createdAt: 0,
    updatedAt: 0,
    completedAt: null,
    sortOrder: 0,
    ...overrides,
  };
}

// ── Tests ───────────────────────────────────────────────────────────────────

describe("createPersistenceMiddleware", () => {
  let storage: ReturnType<typeof createMockStorage>;
  let next: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    storage = createMockStorage();
    next = vi.fn();
  });

  it("calls next() for every action (never blocks the reducer)", () => {
    const middleware = createPersistenceMiddleware(storage);
    const store = createMockStore({ todos: [] });

    middleware(store, { type: "ANY_ACTION" }, next);
    expect(next).toHaveBeenCalledOnce();
  });

  describe("TODO_CREATE", () => {
    it("persists the newly created todo (last item in array)", () => {
      const newTodo = makeTodo({ id: "new-1", title: "New" });
      const store = createMockStore({ todos: [newTodo] });
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TODO_CREATE }, next);

      expect(storage.put).toHaveBeenCalledWith("todos", newTodo);
    });

    it("persists the last item when multiple todos exist", () => {
      const old = makeTodo({ id: "old-1" });
      const newest = makeTodo({ id: "newest-1" });
      const store = createMockStore({ todos: [old, newest] });
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TODO_CREATE }, next);

      expect(storage.put).toHaveBeenCalledWith("todos", newest);
    });
  });

  describe("TODO_UPDATE", () => {
    it("persists the updated todo", () => {
      const todo = makeTodo({ id: "upd-1", title: "Updated" });
      const store = createMockStore({ todos: [todo] });
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TODO_UPDATE, todoId: "upd-1" }, next);

      expect(storage.put).toHaveBeenCalledWith("todos", todo);
    });

    it("does nothing if todo not found", () => {
      const store = createMockStore({ todos: [] });
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TODO_UPDATE, todoId: "ghost" }, next);

      expect(storage.put).not.toHaveBeenCalled();
    });
  });

  describe("TODO_TOGGLE_STATUS", () => {
    it("persists the toggled todo", () => {
      const todo = makeTodo({ id: "tog-1", status: "in-progress" });
      const store = createMockStore({ todos: [todo] });
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TODO_TOGGLE_STATUS, todoId: "tog-1" }, next);

      expect(storage.put).toHaveBeenCalledWith("todos", todo);
    });
  });

  describe("TODO_SET_STATUS", () => {
    it("persists the status-changed todo", () => {
      const todo = makeTodo({ id: "set-1", status: "done" });
      const store = createMockStore({ todos: [todo] });
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TODO_SET_STATUS, todoId: "set-1" }, next);

      expect(storage.put).toHaveBeenCalledWith("todos", todo);
    });
  });

  describe("TODO_DELETE", () => {
    it("deletes the todo from storage by ID", () => {
      const store = createMockStore({ todos: [] }); // already removed by reducer
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TODO_DELETE, todoId: "del-1" }, next);

      expect(storage.delete).toHaveBeenCalledWith("todos", "del-1");
    });
  });

  describe("TODO_CLEAR_COMPLETED", () => {
    it("reads all from storage and deletes removed items", async () => {
      // Current state has 2 items (the not-done ones)
      const remaining = [
        makeTodo({ id: "rem-1", status: "todo" }),
        makeTodo({ id: "rem-2", status: "in-progress" }),
      ];
      const store = createMockStore({ todos: remaining });

      // Storage still has 4 items (2 done ones haven't been cleaned yet)
      storage.getAll.mockResolvedValue([
        makeTodo({ id: "rem-1" }),
        makeTodo({ id: "rem-2" }),
        makeTodo({ id: "done-1", status: "done" }),
        makeTodo({ id: "done-2", status: "done" }),
      ]);

      const middleware = createPersistenceMiddleware(storage);
      middleware(store, { type: TODO_CLEAR_COMPLETED }, next);

      // Wait for the async getAll+delete chain
      await vi.waitFor(() => {
        expect(storage.getAll).toHaveBeenCalledWith("todos");
        expect(storage.delete).toHaveBeenCalledWith("todos", "done-1");
        expect(storage.delete).toHaveBeenCalledWith("todos", "done-2");
      });
    });
  });

  describe("STATE_LOAD", () => {
    it("does not write to storage (data came FROM storage)", () => {
      const store = createMockStore({ todos: [makeTodo()] });
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: STATE_LOAD }, next);

      expect(storage.put).not.toHaveBeenCalled();
      expect(storage.delete).not.toHaveBeenCalled();
    });
  });

  describe("unknown actions", () => {
    it("does not write to storage for unknown actions", () => {
      const store = createMockStore({ todos: [] });
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: "CUSTOM_UNRELATED_ACTION" }, next);

      expect(storage.put).not.toHaveBeenCalled();
      expect(storage.delete).not.toHaveBeenCalled();
    });
  });
});
