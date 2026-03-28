/**
 * persistence.test.ts — Unit tests for the IndexedDB persistence middleware.
 *
 * The persistence middleware is the bridge between the in-memory store and
 * IndexedDB. These tests verify that the correct storage operations (put,
 * delete, getAll) are called for each action type.
 *
 * We mock KVStorage to test the middleware in isolation — no actual
 * IndexedDB is needed.
 *
 * Note on action field names:
 *   The new API uses `taskId` (not `todoId`). The middleware reads
 *   `action.taskId` to find the affected record. Tests must use `taskId`
 *   in action payloads to match what the middleware expects.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { createPersistenceMiddleware } from "../persistence.js";
import type { AppState } from "../reducer.js";
import type { Task, Project } from "../types.js";
import type { GraphEdge } from "../graph.js";
import {
  TASK_CREATE,
  TASK_UPDATE,
  TASK_DELETE,
  TASK_TOGGLE_STATUS,
  TASK_SET_STATUS,
  TASK_CLEAR_COMPLETED,
  STATE_LOAD,
  VIEW_UPSERT,
  CALENDAR_UPSERT,
  PROJECT_UPSERT,
  EDGE_ADD,
  EDGE_REMOVE,
} from "../actions.js";

// ── Mock Storage ─────────────────────────────────────────────────────────────

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

// ── Mock Store ───────────────────────────────────────────────────────────────

function createMockStore(state: AppState) {
  return {
    getState: () => state,
    dispatch: vi.fn(),
    subscribe: vi.fn(),
    use: vi.fn(),
  };
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "test-id",
    title: "Test",
    description: "",
    status: "todo",
    priority: "medium",
    category: "",
    dueDate: null,
    dueTime: null,
    createdAt: 0,
    updatedAt: 0,
    completedAt: null,
    sortOrder: 0,
    ...overrides,
  };
}

/**
 * makeState — builds a full AppState for the mock store.
 * The middleware reads `state.tasks` and `state.edges` so this must match
 * the real AppState shape including projects and edges.
 */
function makeState(tasks: Task[] = [], edges: GraphEdge[] = []): AppState {
  return { tasks, views: [], calendars: [], projects: [], edges, activeViewId: "" };
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe("createPersistenceMiddleware", () => {
  let storage: ReturnType<typeof createMockStorage>;
  let next: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    storage = createMockStorage();
    next = vi.fn();
  });

  it("calls next() for every action (never blocks the reducer)", () => {
    const middleware = createPersistenceMiddleware(storage);
    const store = createMockStore(makeState());

    middleware(store, { type: "ANY_ACTION" }, next);
    expect(next).toHaveBeenCalledOnce();
  });

  describe("TASK_CREATE", () => {
    it("persists the newly created task (last item in array)", () => {
      const newTask = makeTask({ id: "new-1", title: "New" });
      const store = createMockStore(makeState([newTask]));
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TASK_CREATE }, next);

      expect(storage.put).toHaveBeenCalledWith("todos", newTask);
    });

    it("persists the last item when multiple tasks exist", () => {
      const old = makeTask({ id: "old-1" });
      const newest = makeTask({ id: "newest-1" });
      const store = createMockStore(makeState([old, newest]));
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TASK_CREATE }, next);

      expect(storage.put).toHaveBeenCalledWith("todos", newest);
    });
  });

  describe("TASK_UPDATE", () => {
    it("persists the updated task", () => {
      const task = makeTask({ id: "upd-1", title: "Updated" });
      const store = createMockStore(makeState([task]));
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TASK_UPDATE, taskId: "upd-1" }, next);

      expect(storage.put).toHaveBeenCalledWith("todos", task);
    });

    it("does nothing if task not found", () => {
      const store = createMockStore(makeState([]));
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TASK_UPDATE, taskId: "ghost" }, next);

      expect(storage.put).not.toHaveBeenCalled();
    });
  });

  describe("TASK_TOGGLE_STATUS", () => {
    it("persists the toggled task", () => {
      const task = makeTask({ id: "tog-1", status: "in-progress" });
      const store = createMockStore(makeState([task]));
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TASK_TOGGLE_STATUS, taskId: "tog-1" }, next);

      expect(storage.put).toHaveBeenCalledWith("todos", task);
    });
  });

  describe("TASK_SET_STATUS", () => {
    it("persists the status-changed task", () => {
      const task = makeTask({ id: "set-1", status: "done" });
      const store = createMockStore(makeState([task]));
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TASK_SET_STATUS, taskId: "set-1" }, next);

      expect(storage.put).toHaveBeenCalledWith("todos", task);
    });
  });

  describe("TASK_DELETE", () => {
    it("deletes the task from storage by ID", () => {
      const store = createMockStore(makeState([])); // already removed by reducer
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TASK_DELETE, taskId: "del-1" }, next);

      expect(storage.delete).toHaveBeenCalledWith("todos", "del-1");
    });
  });

  describe("TASK_CLEAR_COMPLETED", () => {
    it("reads all from storage and deletes removed items", async () => {
      // Current state has 2 items (the not-done ones, after reducer ran)
      const remaining = [
        makeTask({ id: "rem-1", status: "todo" }),
        makeTask({ id: "rem-2", status: "in-progress" }),
      ];
      const store = createMockStore(makeState(remaining));

      // Storage still has 4 items (2 done ones haven't been cleaned yet)
      storage.getAll.mockResolvedValue([
        makeTask({ id: "rem-1" }),
        makeTask({ id: "rem-2" }),
        makeTask({ id: "done-1", status: "done" }),
        makeTask({ id: "done-2", status: "done" }),
      ]);

      const middleware = createPersistenceMiddleware(storage);
      middleware(store, { type: TASK_CLEAR_COMPLETED }, next);

      // Wait for the async getAll+delete chain
      await vi.waitFor(() => {
        expect(storage.getAll).toHaveBeenCalledWith("todos");
        expect(storage.delete).toHaveBeenCalledWith("todos", "done-1");
        expect(storage.delete).toHaveBeenCalledWith("todos", "done-2");
      });
    });
  });

  describe("VIEW_UPSERT", () => {
    it("persists the view to the views store", () => {
      const view = {
        id: "v-1",
        name: "My View",
        config: { type: "list" as const, filter: { statusFilter: null, priorityFilter: null, categoryFilter: "", searchQuery: "" }, sortField: "createdAt" as const, sortDirection: "desc" as const },
        sortOrder: 0,
        isBuiltIn: false,
        createdAt: 0,
        updatedAt: 0,
      };
      const store = createMockStore(makeState());
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: VIEW_UPSERT, view }, next);

      expect(storage.put).toHaveBeenCalledWith("views", view);
    });
  });

  describe("CALENDAR_UPSERT", () => {
    it("persists the calendar to the calendars store", () => {
      const calendar = {
        id: "gregorian",
        name: "Gregorian",
        weekStartsOn: 0 as const,
        weeklySchedule: {},
        dateOverrides: [],
        timezone: "UTC",
        isBuiltIn: true,
        createdAt: 0,
        updatedAt: 0,
      };
      const store = createMockStore(makeState());
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: CALENDAR_UPSERT, calendar }, next);

      expect(storage.put).toHaveBeenCalledWith("calendars", calendar);
    });
  });

  describe("STATE_LOAD", () => {
    it("does not write to storage (data came FROM storage)", () => {
      const store = createMockStore(makeState([makeTask()]));
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: STATE_LOAD }, next);

      expect(storage.put).not.toHaveBeenCalled();
      expect(storage.delete).not.toHaveBeenCalled();
    });
  });

  describe("PROJECT_UPSERT", () => {
    it("persists the project to the projects store", () => {
      const project: Project = {
        id: "default",
        name: "Default",
        isBuiltIn: true,
        createdAt: 0,
        updatedAt: 0,
      };
      const store = createMockStore(makeState());
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: PROJECT_UPSERT, project }, next);

      expect(storage.put).toHaveBeenCalledWith("projects", project);
    });
  });

  describe("EDGE_ADD", () => {
    const makeEdge = (id = "e1"): GraphEdge => ({
      id,
      fromId: "p1",
      toId: "t1",
      label: "contains",
      createdAt: 0,
    });

    it("persists the edge when the reducer accepted it (edge present in state)", () => {
      const edge = makeEdge();
      // State has the edge (reducer accepted it)
      const store = createMockStore(makeState([], [edge]));
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: EDGE_ADD, edge }, next);

      expect(storage.put).toHaveBeenCalledWith("edges", edge);
    });

    it("does NOT persist the edge when the reducer rejected it (cycle or duplicate)", () => {
      const edge = makeEdge();
      // State does NOT have the edge (reducer rejected the cycle)
      const store = createMockStore(makeState([], []));
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: EDGE_ADD, edge }, next);

      expect(storage.put).not.toHaveBeenCalledWith("edges", edge);
    });
  });

  describe("EDGE_REMOVE", () => {
    it("deletes the edge from storage by id", () => {
      const store = createMockStore(makeState());
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: EDGE_REMOVE, edgeId: "e1" }, next);

      expect(storage.delete).toHaveBeenCalledWith("edges", "e1");
    });
  });

  describe("TASK_CREATE persists edge", () => {
    it("persists both the task and its auto-created edge", () => {
      const task = makeTask({ id: "new-task" });
      const edge: GraphEdge = { id: "e-auto", fromId: "default", toId: "new-task", label: "contains", createdAt: 0 };
      const store = createMockStore(makeState([task], [edge]));
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: TASK_CREATE, id: "new-task" }, next);

      expect(storage.put).toHaveBeenCalledWith("todos", task);
      expect(storage.put).toHaveBeenCalledWith("edges", edge);
    });
  });

  describe("TASK_DELETE cascade edges", () => {
    it("deletes the task and all edges referencing it from storage", async () => {
      const storedEdges: GraphEdge[] = [
        { id: "e1", fromId: "p1", toId: "del-task", label: "contains", createdAt: 0 },
        { id: "e2", fromId: "p2", toId: "del-task", label: "contains", createdAt: 0 },
        { id: "e3", fromId: "p1", toId: "other", label: "contains", createdAt: 0 },
      ];
      storage.getAll.mockResolvedValue(storedEdges as any);

      const store = createMockStore(makeState());
      const middleware = createPersistenceMiddleware(storage);
      middleware(store, { type: TASK_DELETE, taskId: "del-task" }, next);

      expect(storage.delete).toHaveBeenCalledWith("todos", "del-task");

      // Cascade: edges e1 and e2 should be deleted, e3 should not
      await vi.waitFor(() => {
        expect(storage.delete).toHaveBeenCalledWith("edges", "e1");
        expect(storage.delete).toHaveBeenCalledWith("edges", "e2");
      });
      expect(storage.delete).not.toHaveBeenCalledWith("edges", "e3");
    });
  });

  describe("unknown actions", () => {
    it("does not write to storage for unknown actions", () => {
      const store = createMockStore(makeState());
      const middleware = createPersistenceMiddleware(storage);

      middleware(store, { type: "CUSTOM_UNRELATED_ACTION" }, next);

      expect(storage.put).not.toHaveBeenCalled();
      expect(storage.delete).not.toHaveBeenCalled();
    });
  });
});
