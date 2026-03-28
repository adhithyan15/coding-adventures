/**
 * actions.test.ts — Unit tests for action creator functions.
 *
 * Action creators are simple factory functions. We test that they
 * produce the correct action shape (type + payload). This ensures
 * the reducer receives properly-formatted actions.
 *
 * We test both the new TASK_* API and the legacy TODO_* aliases to
 * ensure backward compatibility during the transition period.
 */

import { describe, it, expect } from "vitest";
import {
  // New TASK_* constants
  TASK_CREATE,
  TASK_UPDATE,
  TASK_DELETE,
  TASK_TOGGLE_STATUS,
  TASK_SET_STATUS,
  TASK_CLEAR_COMPLETED,
  STATE_LOAD,
  // New action creators
  createTaskAction,
  updateTaskAction,
  deleteTaskAction,
  toggleStatusAction,
  setStatusAction,
  stateLoadAction,
  clearCompletedAction,
  upsertViewAction,
  setActiveViewAction,
  upsertCalendarAction,
  VIEW_UPSERT,
  VIEW_SET_ACTIVE,
  CALENDAR_UPSERT,
  // Graph / project action creators
  PROJECT_UPSERT,
  EDGE_ADD,
  EDGE_REMOVE,
  projectUpsertAction,
  edgeAddAction,
  edgeRemoveAction,
  // Legacy TODO_* aliases — must still work and resolve to the same values
  TODO_CREATE,
  TODO_UPDATE,
  TODO_DELETE,
  TODO_TOGGLE_STATUS,
  TODO_SET_STATUS,
  TODO_CLEAR_COMPLETED,
  createTodoAction,
  updateTodoAction,
  deleteTodoAction,
} from "../actions.js";
import type { Task, Project } from "../types.js";
import type { GraphEdge } from "../graph.js";

describe("action creators", () => {
  // ── createTaskAction ─────────────────────────────────────────────────────

  describe("createTaskAction", () => {
    it("creates an action with TASK_CREATE type", () => {
      const action = createTaskAction("Title", "Desc", "high", "work", "2026-04-01");
      expect(action.type).toBe(TASK_CREATE);
    });

    it("includes all user-provided fields", () => {
      const action = createTaskAction("Buy groceries", "Milk and eggs", "urgent", "shopping", "2026-03-28");
      expect(action.title).toBe("Buy groceries");
      expect(action.description).toBe("Milk and eggs");
      expect(action.priority).toBe("urgent");
      expect(action.category).toBe("shopping");
      expect(action.dueDate).toBe("2026-03-28");
    });

    it("handles null due date", () => {
      const action = createTaskAction("No deadline", "", "low", "", null);
      expect(action.dueDate).toBeNull();
    });

    it("handles empty description and category", () => {
      const action = createTaskAction("Minimal", "", "medium", "", null);
      expect(action.description).toBe("");
      expect(action.category).toBe("");
    });

    it("includes dueTime when provided", () => {
      const action = createTaskAction("Morning standup", "", "low", "", "2026-04-01", "09:00");
      expect(action.dueTime).toBe("09:00");
    });

    it("defaults dueTime to null when omitted", () => {
      const action = createTaskAction("No time", "", "low", "", null);
      expect(action.dueTime).toBeNull();
    });

    it("defaults projectId to 'default' when omitted", () => {
      const action = createTaskAction("Task", "", "medium", "", null);
      expect(action.projectId).toBe("default");
    });

    it("includes an explicit projectId when provided", () => {
      const action = createTaskAction("Task", "", "medium", "", null, null, "proj-42");
      expect(action.projectId).toBe("proj-42");
    });
  });

  // ── updateTaskAction ─────────────────────────────────────────────────────

  describe("updateTaskAction", () => {
    it("creates an action with TASK_UPDATE type", () => {
      const action = updateTaskAction("id-1", { title: "New title" });
      expect(action.type).toBe(TASK_UPDATE);
    });

    it("includes taskId and patch", () => {
      const patch = { title: "Updated", priority: "high" as const };
      const action = updateTaskAction("id-2", patch);
      expect(action.taskId).toBe("id-2");
      expect(action.patch).toEqual(patch);
    });

    it("supports partial patches (single field)", () => {
      const action = updateTaskAction("id-3", { category: "personal" });
      const patch = action.patch as Partial<Task>;
      expect(patch.category).toBe("personal");
      expect(patch.title).toBeUndefined();
    });
  });

  // ── deleteTaskAction ─────────────────────────────────────────────────────

  describe("deleteTaskAction", () => {
    it("creates an action with TASK_DELETE type", () => {
      const action = deleteTaskAction("id-4");
      expect(action.type).toBe(TASK_DELETE);
    });

    it("includes the taskId", () => {
      const action = deleteTaskAction("some-uuid");
      expect(action.taskId).toBe("some-uuid");
    });
  });

  // ── toggleStatusAction ───────────────────────────────────────────────────

  describe("toggleStatusAction", () => {
    it("creates an action with TASK_TOGGLE_STATUS type", () => {
      const action = toggleStatusAction("id-5");
      expect(action.type).toBe(TASK_TOGGLE_STATUS);
    });

    it("includes the taskId", () => {
      const action = toggleStatusAction("toggle-me");
      expect(action.taskId).toBe("toggle-me");
    });
  });

  // ── setStatusAction ──────────────────────────────────────────────────────

  describe("setStatusAction", () => {
    it("creates an action with TASK_SET_STATUS type", () => {
      const action = setStatusAction("id-6", "done");
      expect(action.type).toBe(TASK_SET_STATUS);
    });

    it("includes taskId and target status", () => {
      const action = setStatusAction("id-7", "in-progress");
      expect(action.taskId).toBe("id-7");
      expect(action.status).toBe("in-progress");
    });

    it("works with all status values", () => {
      expect(setStatusAction("x", "todo").status).toBe("todo");
      expect(setStatusAction("x", "in-progress").status).toBe("in-progress");
      expect(setStatusAction("x", "done").status).toBe("done");
    });
  });

  // ── stateLoadAction ──────────────────────────────────────────────────────

  describe("stateLoadAction", () => {
    it("creates an action with STATE_LOAD type", () => {
      const action = stateLoadAction([], [], [], "");
      expect(action.type).toBe(STATE_LOAD);
    });

    it("includes tasks, views, calendars, and activeViewId", () => {
      const tasks: Task[] = [
        {
          id: "loaded-1",
          title: "Loaded",
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
        },
      ];
      const action = stateLoadAction(tasks, [], [], "all-tasks");
      expect(action.tasks).toBe(tasks); // same reference (not copied)
      expect(action.views).toEqual([]);
      expect(action.calendars).toEqual([]);
      expect(action.activeViewId).toBe("all-tasks");
    });

    it("handles empty arrays for all fields", () => {
      const action = stateLoadAction([], [], [], "");
      expect(action.tasks).toHaveLength(0);
      expect(action.views).toHaveLength(0);
      expect(action.calendars).toHaveLength(0);
      expect(action.activeViewId).toBe("");
    });
  });

  // ── clearCompletedAction ─────────────────────────────────────────────────

  describe("clearCompletedAction", () => {
    it("creates an action with TASK_CLEAR_COMPLETED type", () => {
      const action = clearCompletedAction();
      expect(action.type).toBe(TASK_CLEAR_COMPLETED);
    });
  });

  // ── upsertViewAction ─────────────────────────────────────────────────────

  describe("upsertViewAction", () => {
    it("creates an action with VIEW_UPSERT type", () => {
      const view = {
        id: "v-1",
        name: "My View",
        config: { type: "list" as const, filter: { statusFilter: null, priorityFilter: null, categoryFilter: "", searchQuery: "" }, sortField: "createdAt" as const, sortDirection: "desc" as const },
        sortOrder: 0,
        isBuiltIn: false,
        createdAt: 0,
        updatedAt: 0,
      };
      const action = upsertViewAction(view);
      expect(action.type).toBe(VIEW_UPSERT);
      expect(action.view).toBe(view);
    });
  });

  // ── setActiveViewAction ──────────────────────────────────────────────────

  describe("setActiveViewAction", () => {
    it("creates an action with VIEW_SET_ACTIVE type", () => {
      const action = setActiveViewAction("all-tasks");
      expect(action.type).toBe(VIEW_SET_ACTIVE);
      expect(action.viewId).toBe("all-tasks");
    });
  });

  // ── upsertCalendarAction ─────────────────────────────────────────────────

  describe("upsertCalendarAction", () => {
    it("creates an action with CALENDAR_UPSERT type", () => {
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
      const action = upsertCalendarAction(calendar);
      expect(action.type).toBe(CALENDAR_UPSERT);
      expect(action.calendar).toBe(calendar);
    });
  });

  // ── projectUpsertAction ──────────────────────────────────────────────────

  describe("projectUpsertAction", () => {
    it("creates an action with PROJECT_UPSERT type", () => {
      const project: Project = { id: "p1", name: "My Project", isBuiltIn: false, createdAt: 0, updatedAt: 0 };
      const action = projectUpsertAction(project);
      expect(action.type).toBe(PROJECT_UPSERT);
    });

    it("includes the project payload by reference", () => {
      const project: Project = { id: "default", name: "Default", isBuiltIn: true, createdAt: 0, updatedAt: 0 };
      const action = projectUpsertAction(project);
      expect(action.project).toBe(project); // same reference, not a copy
    });

    it("works for a built-in project (isBuiltIn: true)", () => {
      const project: Project = { id: "default", name: "Default", isBuiltIn: true, createdAt: 0, updatedAt: 0 };
      const action = projectUpsertAction(project);
      expect((action.project as Project).isBuiltIn).toBe(true);
    });

    it("works for a user-created project (isBuiltIn: false)", () => {
      const project: Project = { id: "user-p1", name: "Work", isBuiltIn: false, createdAt: 1000, updatedAt: 2000 };
      const action = projectUpsertAction(project);
      expect((action.project as Project).name).toBe("Work");
      expect((action.project as Project).isBuiltIn).toBe(false);
    });
  });

  // ── edgeAddAction ────────────────────────────────────────────────────────

  describe("edgeAddAction", () => {
    it("creates an action with EDGE_ADD type", () => {
      const edge: GraphEdge = { id: "e1", fromId: "p1", toId: "t1", label: "contains", createdAt: 0 };
      const action = edgeAddAction(edge);
      expect(action.type).toBe(EDGE_ADD);
    });

    it("includes the edge payload by reference", () => {
      const edge: GraphEdge = { id: "e1", fromId: "default", toId: "task-42", label: "contains", createdAt: 0 };
      const action = edgeAddAction(edge);
      expect(action.edge).toBe(edge);
    });

    it("preserves all edge fields", () => {
      const edge: GraphEdge = { id: "e-abc", fromId: "proj-1", toId: "task-5", label: "contains", createdAt: 9999 };
      const action = edgeAddAction(edge);
      const payload = action.edge as GraphEdge;
      expect(payload.id).toBe("e-abc");
      expect(payload.fromId).toBe("proj-1");
      expect(payload.toId).toBe("task-5");
      expect(payload.label).toBe("contains");
      expect(payload.createdAt).toBe(9999);
    });
  });

  // ── edgeRemoveAction ─────────────────────────────────────────────────────

  describe("edgeRemoveAction", () => {
    it("creates an action with EDGE_REMOVE type", () => {
      const action = edgeRemoveAction("e1");
      expect(action.type).toBe(EDGE_REMOVE);
    });

    it("includes the edgeId", () => {
      const action = edgeRemoveAction("my-edge-uuid");
      expect(action.edgeId).toBe("my-edge-uuid");
    });

    it("works with any string edge ID", () => {
      const ids = ["e1", "e-abc-123", "00000000-0000-7000-0000-000000000001"];
      for (const id of ids) {
        expect(edgeRemoveAction(id).edgeId).toBe(id);
      }
    });
  });

  // ── Legacy TODO_* alias constants ────────────────────────────────────────

  describe("legacy TODO_* alias constants", () => {
    it("TODO_CREATE equals TASK_CREATE", () => {
      expect(TODO_CREATE).toBe(TASK_CREATE);
    });

    it("TODO_UPDATE equals TASK_UPDATE", () => {
      expect(TODO_UPDATE).toBe(TASK_UPDATE);
    });

    it("TODO_DELETE equals TASK_DELETE", () => {
      expect(TODO_DELETE).toBe(TASK_DELETE);
    });

    it("TODO_TOGGLE_STATUS equals TASK_TOGGLE_STATUS", () => {
      expect(TODO_TOGGLE_STATUS).toBe(TASK_TOGGLE_STATUS);
    });

    it("TODO_SET_STATUS equals TASK_SET_STATUS", () => {
      expect(TODO_SET_STATUS).toBe(TASK_SET_STATUS);
    });

    it("TODO_CLEAR_COMPLETED equals TASK_CLEAR_COMPLETED", () => {
      expect(TODO_CLEAR_COMPLETED).toBe(TASK_CLEAR_COMPLETED);
    });
  });

  // ── Legacy createTodoAction / updateTodoAction / deleteTodoAction ────────

  describe("legacy action creator aliases", () => {
    it("createTodoAction produces TASK_CREATE action", () => {
      const action = createTodoAction("Title", "Desc", "high", "work", "2026-04-01");
      expect(action.type).toBe(TASK_CREATE);
      expect(action.title).toBe("Title");
    });

    it("updateTodoAction produces TASK_UPDATE action with taskId field", () => {
      const action = updateTodoAction("old-id", { title: "New" });
      expect(action.type).toBe(TASK_UPDATE);
      // updateTodoAction calls updateTaskAction which uses taskId
      expect(action.taskId).toBe("old-id");
    });

    it("deleteTodoAction produces TASK_DELETE action with taskId field", () => {
      const action = deleteTodoAction("del-id");
      expect(action.type).toBe(TASK_DELETE);
      expect(action.taskId).toBe("del-id");
    });
  });

  // ── Action type constants ─────────────────────────────────────────────────

  describe("action type constants", () => {
    it("all action type constants are unique strings", () => {
      const types = [
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
        PROJECT_UPSERT,
        EDGE_ADD,
        EDGE_REMOVE,
      ];
      const uniqueTypes = new Set(types);
      expect(uniqueTypes.size).toBe(types.length);
    });

    it("all action types are non-empty strings", () => {
      const types = [
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
        PROJECT_UPSERT,
        EDGE_ADD,
        EDGE_REMOVE,
      ];
      for (const t of types) {
        expect(t).toBeTypeOf("string");
        expect(t.length).toBeGreaterThan(0);
      }
    });
  });
});
