/**
 * reducer.test.ts — Comprehensive unit tests for the todo app reducer.
 *
 * Tests every action type and edge case. The reducer is the core of the
 * app — if it's correct, the app is correct. Pure functions are easy to
 * test: input → output, no mocks needed.
 */

import { describe, it, expect, vi } from "vitest";
import { webcrypto } from "node:crypto";
import { reducer } from "../reducer.js";
import type { AppState } from "../reducer.js";
import type { Task, Project } from "../types.js";
import type { GraphEdge } from "../graph.js";
import {
  createTaskAction,
  updateTaskAction,
  deleteTaskAction,
  toggleStatusAction,
  setStatusAction,
  stateLoadAction,
  clearCompletedAction,
  projectUpsertAction,
  edgeAddAction,
  edgeRemoveAction,
  // Legacy aliases still work — test that they do
  createTodoAction,
  updateTodoAction,
  deleteTodoAction,
} from "../actions.js";

// ── Test Helpers ────────────────────────────────────────────────────────────

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "test-id-1",
    title: "Test Todo",
    description: "Test description",
    status: "todo",
    priority: "medium",
    category: "test",
    dueDate: null,
    dueTime: null,
    createdAt: 1000,
    updatedAt: 1000,
    completedAt: null,
    sortOrder: 1000,
    ...overrides,
  };
}

/**
 * makeState — builds a full AppState with sensible defaults for views,
 * calendars, projects, edges, and activeViewId so tests only need to
 * specify tasks.
 */
function makeState(tasks: Task[] = []): AppState {
  return { tasks, views: [], calendars: [], projects: [], edges: [], activeViewId: "" };
}

// ── Test Suites ─────────────────────────────────────────────────────────────

describe("reducer", () => {
  // ── TASK_CREATE ──────────────────────────────────────────────────────────

  describe("TASK_CREATE", () => {
    it("adds a new task to an empty list", () => {
      // Mock crypto.randomUUID for deterministic testing
      const mockUUID = "mock-uuid-123";
      vi.stubGlobal("crypto", { randomUUID: () => mockUUID, getRandomValues: (b: Uint8Array) => webcrypto.getRandomValues(b), subtle: webcrypto.subtle });

      const state = makeState();
      const action = createTaskAction("Buy milk", "From the store", "high", "shopping", "2026-04-01");
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(1);

      const task = newState.tasks[0]!;
      expect(task.id).toBe(mockUUID);
      expect(task.title).toBe("Buy milk");
      expect(task.description).toBe("From the store");
      expect(task.status).toBe("todo");
      expect(task.priority).toBe("high");
      expect(task.category).toBe("shopping");
      expect(task.dueDate).toBe("2026-04-01");
      expect(task.completedAt).toBeNull();
      expect(task.createdAt).toBeTypeOf("number");
      expect(task.updatedAt).toBeTypeOf("number");
      expect(task.sortOrder).toBeTypeOf("number");

      vi.unstubAllGlobals();
    });

    it("appends to existing list", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "uuid-2", getRandomValues: (b: Uint8Array) => webcrypto.getRandomValues(b), subtle: webcrypto.subtle });

      const existing = makeTask({ id: "existing-1" });
      const state = makeState([existing]);
      const action = createTaskAction("Second task", "", "low", "", null);
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(2);
      expect(newState.tasks[0]!.id).toBe("existing-1");
      expect(newState.tasks[1]!.id).toBe("uuid-2");

      vi.unstubAllGlobals();
    });

    it("defaults to medium priority when empty", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "uuid-3", getRandomValues: (b: Uint8Array) => webcrypto.getRandomValues(b), subtle: webcrypto.subtle });

      const state = makeState();
      // Include id field — now that createTaskAction pre-generates the UUID,
      // raw action objects used in tests must also carry it. The reducer uses
      // action.id if present, falling back to crypto.randomUUID() otherwise.
      const action = { type: "TASK_CREATE", id: "uuid-3", title: "Task", description: "", priority: "", category: "", dueDate: null, dueTime: null };
      const newState = reducer(state, action);

      expect(newState.tasks[0]!.priority).toBe("medium");

      vi.unstubAllGlobals();
    });

    it("does not mutate the original state", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "uuid-4", getRandomValues: (b: Uint8Array) => webcrypto.getRandomValues(b), subtle: webcrypto.subtle });

      const state = makeState();
      const action = createTaskAction("Immutable test", "", "low", "", null);
      const newState = reducer(state, action);

      expect(state.tasks).toHaveLength(0);
      expect(newState.tasks).toHaveLength(1);
      expect(state).not.toBe(newState);

      vi.unstubAllGlobals();
    });

    it("legacy createTodoAction alias produces the same result", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "legacy-uuid", getRandomValues: (b: Uint8Array) => webcrypto.getRandomValues(b), subtle: webcrypto.subtle });

      const state = makeState();
      const action = createTodoAction("Legacy task", "desc", "high", "cat", "2026-04-01");
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(1);
      expect(newState.tasks[0]!.title).toBe("Legacy task");

      vi.unstubAllGlobals();
    });
  });

  // ── TASK_UPDATE ──────────────────────────────────────────────────────────

  describe("TASK_UPDATE", () => {
    it("updates title of an existing task", () => {
      const task = makeTask({ id: "u-1" });
      const state = makeState([task]);
      const action = updateTaskAction("u-1", { title: "Updated title" });
      const newState = reducer(state, action);

      expect(newState.tasks[0]!.title).toBe("Updated title");
      expect(newState.tasks[0]!.description).toBe("Test description"); // unchanged
    });

    it("updates multiple fields at once", () => {
      const task = makeTask({ id: "u-2" });
      const state = makeState([task]);
      const action = updateTaskAction("u-2", {
        title: "New title",
        priority: "urgent",
        category: "work",
        dueDate: "2026-12-31",
      });
      const newState = reducer(state, action);

      expect(newState.tasks[0]!.title).toBe("New title");
      expect(newState.tasks[0]!.priority).toBe("urgent");
      expect(newState.tasks[0]!.category).toBe("work");
      expect(newState.tasks[0]!.dueDate).toBe("2026-12-31");
    });

    it("bumps updatedAt timestamp", () => {
      const task = makeTask({ id: "u-3", updatedAt: 1000 });
      const state = makeState([task]);
      const action = updateTaskAction("u-3", { title: "Changed" });
      const newState = reducer(state, action);

      expect(newState.tasks[0]!.updatedAt).toBeGreaterThan(1000);
    });

    it("leaves non-matching tasks unchanged", () => {
      const task1 = makeTask({ id: "u-4a", title: "Keep me" });
      const task2 = makeTask({ id: "u-4b", title: "Change me" });
      const state = makeState([task1, task2]);
      const action = updateTaskAction("u-4b", { title: "Changed" });
      const newState = reducer(state, action);

      expect(newState.tasks[0]!.title).toBe("Keep me");
      expect(newState.tasks[1]!.title).toBe("Changed");
    });

    it("does nothing for non-existent ID", () => {
      const task = makeTask({ id: "u-5" });
      const state = makeState([task]);
      const action = updateTaskAction("non-existent", { title: "Ghost" });
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(1);
      expect(newState.tasks[0]!.title).toBe("Test Todo"); // unchanged
    });

    it("legacy updateTodoAction alias produces the same result", () => {
      const task = makeTask({ id: "legacy-u-1" });
      const state = makeState([task]);
      const action = updateTodoAction("legacy-u-1", { title: "Legacy update" });
      const newState = reducer(state, action);

      expect(newState.tasks[0]!.title).toBe("Legacy update");
    });
  });

  // ── TASK_DELETE ──────────────────────────────────────────────────────────

  describe("TASK_DELETE", () => {
    it("removes a task by ID", () => {
      const task = makeTask({ id: "d-1" });
      const state = makeState([task]);
      const action = deleteTaskAction("d-1");
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(0);
    });

    it("removes only the matching task", () => {
      const task1 = makeTask({ id: "d-2a" });
      const task2 = makeTask({ id: "d-2b" });
      const task3 = makeTask({ id: "d-2c" });
      const state = makeState([task1, task2, task3]);
      const action = deleteTaskAction("d-2b");
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(2);
      expect(newState.tasks.map((t) => t.id)).toEqual(["d-2a", "d-2c"]);
    });

    it("does nothing for non-existent ID", () => {
      const task = makeTask({ id: "d-3" });
      const state = makeState([task]);
      const action = deleteTaskAction("ghost");
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(1);
    });

    it("handles deleting from empty list", () => {
      const state = makeState();
      const action = deleteTaskAction("any-id");
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(0);
    });

    it("legacy deleteTodoAction alias produces the same result", () => {
      const task = makeTask({ id: "legacy-d-1" });
      const state = makeState([task]);
      const action = deleteTodoAction("legacy-d-1");
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(0);
    });
  });

  // ── TASK_TOGGLE_STATUS ───────────────────────────────────────────────────

  describe("TASK_TOGGLE_STATUS", () => {
    it("cycles todo → in-progress", () => {
      const task = makeTask({ id: "t-1", status: "todo" });
      const state = makeState([task]);
      const action = toggleStatusAction("t-1");
      const newState = reducer(state, action);

      expect(newState.tasks[0]!.status).toBe("in-progress");
      expect(newState.tasks[0]!.completedAt).toBeNull();
    });

    it("cycles in-progress → done", () => {
      const task = makeTask({ id: "t-2", status: "in-progress" });
      const state = makeState([task]);
      const action = toggleStatusAction("t-2");
      const newState = reducer(state, action);

      expect(newState.tasks[0]!.status).toBe("done");
      expect(newState.tasks[0]!.completedAt).toBeTypeOf("number");
    });

    it("cycles done → todo", () => {
      const task = makeTask({ id: "t-3", status: "done", completedAt: 5000 });
      const state = makeState([task]);
      const action = toggleStatusAction("t-3");
      const newState = reducer(state, action);

      expect(newState.tasks[0]!.status).toBe("todo");
      expect(newState.tasks[0]!.completedAt).toBeNull();
    });

    it("full cycle: todo → in-progress → done → todo", () => {
      const task = makeTask({ id: "t-4", status: "todo" });
      let state = makeState([task]);

      state = reducer(state, toggleStatusAction("t-4"));
      expect(state.tasks[0]!.status).toBe("in-progress");

      state = reducer(state, toggleStatusAction("t-4"));
      expect(state.tasks[0]!.status).toBe("done");

      state = reducer(state, toggleStatusAction("t-4"));
      expect(state.tasks[0]!.status).toBe("todo");
    });

    it("bumps updatedAt on toggle", () => {
      const task = makeTask({ id: "t-5", updatedAt: 1000 });
      const state = makeState([task]);
      const action = toggleStatusAction("t-5");
      const newState = reducer(state, action);

      expect(newState.tasks[0]!.updatedAt).toBeGreaterThan(1000);
    });
  });

  // ── TASK_SET_STATUS ──────────────────────────────────────────────────────

  describe("TASK_SET_STATUS", () => {
    it("sets status to done with completedAt", () => {
      const task = makeTask({ id: "s-1", status: "todo" });
      const state = makeState([task]);
      const action = setStatusAction("s-1", "done");
      const newState = reducer(state, action);

      expect(newState.tasks[0]!.status).toBe("done");
      expect(newState.tasks[0]!.completedAt).toBeTypeOf("number");
    });

    it("clears completedAt when moving away from done", () => {
      const task = makeTask({ id: "s-2", status: "done", completedAt: 5000 });
      const state = makeState([task]);
      const action = setStatusAction("s-2", "in-progress");
      const newState = reducer(state, action);

      expect(newState.tasks[0]!.status).toBe("in-progress");
      expect(newState.tasks[0]!.completedAt).toBeNull();
    });

    it("can set same status (idempotent)", () => {
      const task = makeTask({ id: "s-3", status: "in-progress" });
      const state = makeState([task]);
      const action = setStatusAction("s-3", "in-progress");
      const newState = reducer(state, action);

      expect(newState.tasks[0]!.status).toBe("in-progress");
    });
  });

  // ── STATE_LOAD ───────────────────────────────────────────────────────────

  describe("STATE_LOAD", () => {
    it("replaces all tasks with loaded data", () => {
      const existing = makeTask({ id: "old-1" });
      const state = makeState([existing]);

      const loadedTasks = [
        makeTask({ id: "loaded-1", title: "Loaded 1" }),
        makeTask({ id: "loaded-2", title: "Loaded 2" }),
      ];
      const action = stateLoadAction(loadedTasks, [], [], "");
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(2);
      expect(newState.tasks[0]!.id).toBe("loaded-1");
      expect(newState.tasks[1]!.id).toBe("loaded-2");
    });

    it("handles loading empty array", () => {
      const existing = makeTask({ id: "old-2" });
      const state = makeState([existing]);
      const action = stateLoadAction([], [], [], "");
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(0);
    });

    it("hydrates views and calendars from loaded state", () => {
      const state = makeState();
      const action = stateLoadAction([], [], [], "all-tasks");
      const newState = reducer(state, action);

      expect(newState.activeViewId).toBe("all-tasks");
      expect(newState.views).toHaveLength(0);
      expect(newState.calendars).toHaveLength(0);
    });
  });

  // ── TASK_CLEAR_COMPLETED ─────────────────────────────────────────────────

  describe("TASK_CLEAR_COMPLETED", () => {
    it("removes all done tasks", () => {
      const tasks = [
        makeTask({ id: "cc-1", status: "todo" }),
        makeTask({ id: "cc-2", status: "done" }),
        makeTask({ id: "cc-3", status: "in-progress" }),
        makeTask({ id: "cc-4", status: "done" }),
      ];
      const state = makeState(tasks);
      const action = clearCompletedAction();
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(2);
      expect(newState.tasks.map((t) => t.id)).toEqual(["cc-1", "cc-3"]);
    });

    it("does nothing when no completed tasks", () => {
      const tasks = [
        makeTask({ id: "cc-5", status: "todo" }),
        makeTask({ id: "cc-6", status: "in-progress" }),
      ];
      const state = makeState(tasks);
      const action = clearCompletedAction();
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(2);
    });

    it("handles all completed (empties the list)", () => {
      const tasks = [
        makeTask({ id: "cc-7", status: "done" }),
        makeTask({ id: "cc-8", status: "done" }),
      ];
      const state = makeState(tasks);
      const action = clearCompletedAction();
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(0);
    });
  });

  // ── PROJECT_UPSERT ───────────────────────────────────────────────────────

  describe("PROJECT_UPSERT", () => {
    const makeProject = (overrides: Partial<Project> = {}): Project => ({
      id: "p1",
      name: "Test Project",
      isBuiltIn: false,
      createdAt: 0,
      updatedAt: 0,
      ...overrides,
    });

    it("adds a new project when none exists with that id", () => {
      const state = makeState();
      const project = makeProject({ id: "new-project" });
      const newState = reducer(state, projectUpsertAction(project));
      expect(newState.projects).toHaveLength(1);
      expect(newState.projects[0]!.id).toBe("new-project");
    });

    it("replaces an existing project with the same id", () => {
      const existing = makeProject({ id: "p1", name: "Old Name" });
      const state = { ...makeState(), projects: [existing] };
      const updated = makeProject({ id: "p1", name: "New Name" });
      const newState = reducer(state, projectUpsertAction(updated));
      expect(newState.projects).toHaveLength(1);
      expect(newState.projects[0]!.name).toBe("New Name");
    });

    it("does not mutate existing state", () => {
      const state = { ...makeState(), projects: [makeProject({ id: "p1" })] };
      Object.freeze(state.projects);
      expect(() =>
        reducer(state, projectUpsertAction(makeProject({ id: "p2" }))),
      ).not.toThrow();
    });
  });

  // ── EDGE_ADD ─────────────────────────────────────────────────────────────

  describe("EDGE_ADD", () => {
    const makeEdge = (fromId: string, toId: string, id = "e1"): GraphEdge => ({
      id,
      fromId,
      toId,
      label: "contains",
      createdAt: 0,
    });

    it("adds a valid edge to state.edges", () => {
      const project: Project = { id: "p1", name: "P1", isBuiltIn: false, createdAt: 0, updatedAt: 0 };
      const task = makeTask({ id: "t1" });
      const state = { ...makeState([task]), projects: [project] };
      const edge = makeEdge("p1", "t1");
      const newState = reducer(state, edgeAddAction(edge));
      expect(newState.edges).toHaveLength(1);
      expect(newState.edges[0]!.id).toBe("e1");
    });

    it("is a no-op when the edge would create a cycle", () => {
      // p1 → t1 exists; trying to add t1 → p1 should be rejected
      const project: Project = { id: "p1", name: "P1", isBuiltIn: false, createdAt: 0, updatedAt: 0 };
      const task = makeTask({ id: "t1" });
      const existingEdge = makeEdge("p1", "t1", "existing");
      const state = { ...makeState([task]), projects: [project], edges: [existingEdge] };
      const cycleEdge = makeEdge("t1", "p1", "cycle-attempt");
      const newState = reducer(state, edgeAddAction(cycleEdge));
      expect(newState.edges).toHaveLength(1); // unchanged
      expect(newState).toBe(state) // same reference
    });

    it("is a no-op for self-loops", () => {
      const project: Project = { id: "p1", name: "P1", isBuiltIn: false, createdAt: 0, updatedAt: 0 };
      const state = { ...makeState(), projects: [project] };
      const selfLoop = makeEdge("p1", "p1", "self");
      const newState = reducer(state, edgeAddAction(selfLoop));
      expect(newState.edges).toHaveLength(0);
    });

    it("is a no-op for duplicate edges", () => {
      const project: Project = { id: "p1", name: "P1", isBuiltIn: false, createdAt: 0, updatedAt: 0 };
      const task = makeTask({ id: "t1" });
      const existingEdge = makeEdge("p1", "t1", "e1");
      const state = { ...makeState([task]), projects: [project], edges: [existingEdge] };
      const duplicate = makeEdge("p1", "t1", "e2"); // same from/to, different id
      const newState = reducer(state, edgeAddAction(duplicate));
      expect(newState.edges).toHaveLength(1); // still just one
    });
  });

  // ── EDGE_REMOVE ──────────────────────────────────────────────────────────

  describe("EDGE_REMOVE", () => {
    const makeEdge = (id: string): GraphEdge => ({
      id,
      fromId: "p1",
      toId: "t1",
      label: "contains",
      createdAt: 0,
    });

    it("removes an edge by id", () => {
      const edge = makeEdge("e1");
      const state = { ...makeState(), edges: [edge] };
      const newState = reducer(state, edgeRemoveAction("e1"));
      expect(newState.edges).toHaveLength(0);
    });

    it("is a no-op when edge id does not exist", () => {
      const edge = makeEdge("e1");
      const state = { ...makeState(), edges: [edge] };
      const newState = reducer(state, edgeRemoveAction("non-existent"));
      expect(newState.edges).toHaveLength(1);
    });

    it("only removes the targeted edge, not others", () => {
      const edges: GraphEdge[] = [
        { id: "e1", fromId: "p1", toId: "t1", label: "contains", createdAt: 0 },
        { id: "e2", fromId: "p1", toId: "t2", label: "contains", createdAt: 0 },
        { id: "e3", fromId: "p1", toId: "t3", label: "contains", createdAt: 0 },
      ];
      const state = { ...makeState(), edges };
      const newState = reducer(state, edgeRemoveAction("e2"));
      expect(newState.edges).toHaveLength(2);
      expect(newState.edges.map((e) => e.id)).toEqual(["e1", "e3"]);
    });
  });

  // ── TASK_CREATE (atomic edge) ─────────────────────────────────────────────

  describe("TASK_CREATE atomic edge", () => {
    it("creates a 'contains' edge from default project to the new task", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "task-uuid", getRandomValues: (b: Uint8Array) => webcrypto.getRandomValues(b), subtle: webcrypto.subtle });
      const state = makeState();
      const action = createTaskAction("Test", "", "low", "", null);
      const newState = reducer(state, action);

      // Should have one edge
      expect(newState.edges).toHaveLength(1);
      const edge = newState.edges[0]!;
      expect(edge.fromId).toBe("default");
      expect(edge.toId).toBe("task-uuid");
      expect(edge.label).toBe("contains");

      vi.unstubAllGlobals();
    });

    it("creates edge from explicit projectId when provided", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "task-2", getRandomValues: (b: Uint8Array) => webcrypto.getRandomValues(b), subtle: webcrypto.subtle });
      const state = makeState();
      const action = createTaskAction("T", "", "low", "", null, null, "my-project");
      const newState = reducer(state, action);

      expect(newState.edges[0]!.fromId).toBe("my-project");
      expect(newState.edges[0]!.toId).toBe("task-2");

      vi.unstubAllGlobals();
    });

    it("task and edge are both appended atomically", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "atomic-id", getRandomValues: (b: Uint8Array) => webcrypto.getRandomValues(b), subtle: webcrypto.subtle });
      const state = makeState();
      const action = createTaskAction("A", "", "low", "", null);
      const newState = reducer(state, action);

      expect(newState.tasks).toHaveLength(1);
      expect(newState.edges).toHaveLength(1);
      expect(newState.tasks[0]!.id).toBe("atomic-id");
      expect(newState.edges[0]!.toId).toBe("atomic-id");

      vi.unstubAllGlobals();
    });
  });

  // ── TASK_DELETE cascade ───────────────────────────────────────────────────

  describe("TASK_DELETE cascade", () => {
    it("removes all edges where toId matches the deleted task", () => {
      const task = makeTask({ id: "del-task" });
      const edges: GraphEdge[] = [
        { id: "e1", fromId: "p1", toId: "del-task", label: "contains", createdAt: 0 },
        { id: "e2", fromId: "p2", toId: "del-task", label: "contains", createdAt: 0 },
        { id: "e3", fromId: "p1", toId: "other-task", label: "contains", createdAt: 0 },
      ];
      const state = { ...makeState([task]), edges };
      const newState = reducer(state, deleteTaskAction("del-task"));

      expect(newState.tasks).toHaveLength(0);
      expect(newState.edges).toHaveLength(1); // e3 survives
      expect(newState.edges[0]!.id).toBe("e3");
    });

    it("removes edges where fromId matches (if task was a project too)", () => {
      const task = makeTask({ id: "del-task" });
      const edges: GraphEdge[] = [
        { id: "e1", fromId: "del-task", toId: "child", label: "contains", createdAt: 0 },
        { id: "e2", fromId: "parent", toId: "del-task", label: "contains", createdAt: 0 },
        { id: "e3", fromId: "other", toId: "other2", label: "contains", createdAt: 0 },
      ];
      const state = { ...makeState([task]), edges };
      const newState = reducer(state, deleteTaskAction("del-task"));

      expect(newState.edges).toHaveLength(1); // only e3 survives
      expect(newState.edges[0]!.id).toBe("e3");
    });
  });

  // ── Unknown actions ──────────────────────────────────────────────────────

  describe("unknown actions", () => {
    it("returns state unchanged for unknown action types", () => {
      const state = makeState([makeTask()]);
      const action = { type: "UNKNOWN_ACTION" };
      const newState = reducer(state, action);

      expect(newState).toBe(state); // exact same reference
    });
  });

  // ── Immutability checks ──────────────────────────────────────────────────

  describe("immutability", () => {
    it("never mutates the input state object", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "imm-uuid", getRandomValues: (b: Uint8Array) => webcrypto.getRandomValues(b), subtle: webcrypto.subtle });

      const original = makeTask({ id: "imm-1" });
      const state = makeState([original]);

      // Freeze the state to catch any mutations
      Object.freeze(state);
      Object.freeze(state.tasks);

      // All these should work without throwing (no mutations)
      expect(() => reducer(state, createTaskAction("New", "", "low", "", null))).not.toThrow();
      expect(() => reducer(state, updateTaskAction("imm-1", { title: "Changed" }))).not.toThrow();
      expect(() => reducer(state, deleteTaskAction("imm-1"))).not.toThrow();
      expect(() => reducer(state, toggleStatusAction("imm-1"))).not.toThrow();
      expect(() => reducer(state, setStatusAction("imm-1", "done"))).not.toThrow();
      expect(() => reducer(state, clearCompletedAction())).not.toThrow();

      vi.unstubAllGlobals();
    });

    it("returns a new state reference on every mutation", () => {
      vi.stubGlobal("crypto", { randomUUID: () => "ref-uuid", getRandomValues: (b: Uint8Array) => webcrypto.getRandomValues(b), subtle: webcrypto.subtle });

      const state = makeState([makeTask({ id: "ref-1" })]);

      const s1 = reducer(state, createTaskAction("X", "", "low", "", null));
      const s2 = reducer(state, updateTaskAction("ref-1", { title: "Y" }));
      const s3 = reducer(state, deleteTaskAction("ref-1"));
      const s4 = reducer(state, toggleStatusAction("ref-1"));

      expect(s1).not.toBe(state);
      expect(s2).not.toBe(state);
      expect(s3).not.toBe(state);
      expect(s4).not.toBe(state);

      vi.unstubAllGlobals();
    });
  });
});
