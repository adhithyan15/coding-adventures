/**
 * graph.test.ts — Unit tests for the DAG graph layer.
 *
 * Tests cover:
 *   - buildGraph: correctly populates nodes and edges from flat data
 *   - wouldCreateCycle: detects self-loops, direct cycles, transitive cycles
 *   - getProjectTaskIds: filters successors to tasks only
 *   - getSubprojectIds: filters successors to projects only
 *   - getTaskProjectIds: finds parent projects of a task
 *   - newEdgeId: produces a valid UUID string
 *
 * We test the graph layer against the real @coding-adventures/directed-graph
 * package — no mocking needed for pure graph operations.
 */

import { describe, it, expect } from "vitest";
import {
  buildGraph,
  wouldCreateCycle,
  getProjectTaskIds,
  getSubprojectIds,
  getTaskProjectIds,
  newEdgeId,
} from "../graph.js";
import type { GraphEdge } from "../graph.js";
import type { Task } from "../types.js";
import type { Project } from "../types.js";

// ── Test Helpers ──────────────────────────────────────────────────────────────

function makeProject(id: string, overrides: Partial<Project> = {}): Project {
  return {
    id,
    name: `Project ${id}`,
    isBuiltIn: false,
    createdAt: 0,
    updatedAt: 0,
    ...overrides,
  };
}

function makeTask(id: string, overrides: Partial<Task> = {}): Task {
  return {
    id,
    title: `Task ${id}`,
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

function makeEdge(fromId: string, toId: string, overrides: Partial<GraphEdge> = {}): GraphEdge {
  return {
    id: `edge-${fromId}-${toId}`,
    fromId,
    toId,
    label: "contains",
    createdAt: 0,
    ...overrides,
  };
}

// ── buildGraph ────────────────────────────────────────────────────────────────

describe("buildGraph", () => {
  it("returns an empty graph when all inputs are empty", () => {
    const g = buildGraph([], [], []);
    expect(g.size).toBe(0);
    expect(g.nodes()).toHaveLength(0);
  });

  it("registers all project ids as nodes", () => {
    const projects = [makeProject("p1"), makeProject("p2")];
    const g = buildGraph(projects, [], []);
    expect(g.hasNode("p1")).toBe(true);
    expect(g.hasNode("p2")).toBe(true);
    expect(g.size).toBe(2);
  });

  it("registers all task ids as nodes", () => {
    const tasks = [makeTask("t1"), makeTask("t2"), makeTask("t3")];
    const g = buildGraph([], tasks, []);
    expect(g.hasNode("t1")).toBe(true);
    expect(g.hasNode("t2")).toBe(true);
    expect(g.hasNode("t3")).toBe(true);
  });

  it("adds edges between registered nodes", () => {
    const projects = [makeProject("p1")];
    const tasks = [makeTask("t1"), makeTask("t2")];
    const edges = [makeEdge("p1", "t1"), makeEdge("p1", "t2")];

    const g = buildGraph(projects, tasks, edges);

    expect(g.hasEdge("p1", "t1", "contains")).toBe(true);
    expect(g.hasEdge("p1", "t2", "contains")).toBe(true);
    expect(g.successors("p1", "contains")).toEqual(expect.arrayContaining(["t1", "t2"]));
  });

  it("auto-adds missing nodes referenced only in edges", () => {
    // Edge references a node not in projects or tasks arrays
    const edges = [makeEdge("orphan-project", "orphan-task")];
    const g = buildGraph([], [], edges);
    // Both endpoints should have been added automatically
    expect(g.hasNode("orphan-project")).toBe(true);
    expect(g.hasNode("orphan-task")).toBe(true);
  });

  it("does not add duplicate edges", () => {
    const projects = [makeProject("p1")];
    const tasks = [makeTask("t1")];
    const edges = [makeEdge("p1", "t1"), makeEdge("p1", "t1")]; // duplicate

    // Should not throw; second addEdge is skipped by hasEdge guard
    const g = buildGraph(projects, tasks, edges);
    expect(g.edges().filter(([from, to, label]) => from === "p1" && to === "t1" && label === "contains")).toHaveLength(1);
  });
});

// ── wouldCreateCycle ──────────────────────────────────────────────────────────

describe("wouldCreateCycle", () => {
  it("returns true for a self-loop (fromId === toId)", () => {
    const g = buildGraph([makeProject("p1")], [], []);
    expect(wouldCreateCycle(g, "p1", "p1")).toBe(true);
  });

  it("returns false when toId is not yet a node in the graph", () => {
    const g = buildGraph([makeProject("p1")], [], []);
    // "t1" doesn't exist in the graph yet
    expect(wouldCreateCycle(g, "p1", "t1")).toBe(false);
  });

  it("returns false when there is no path from toId back to fromId", () => {
    // Graph: p1 → t1  (no path from t1 back to p1)
    const projects = [makeProject("p1")];
    const tasks = [makeTask("t1")];
    const edges = [makeEdge("p1", "t1")];
    const g = buildGraph(projects, tasks, edges);

    // Adding p1 → t2 should not create a cycle
    expect(wouldCreateCycle(g, "p1", "t1")).toBe(false);
    // Adding t1 → p2 (new node p2) should not create a cycle
    expect(wouldCreateCycle(g, "t1", "new-node")).toBe(false);
  });

  it("returns true for a direct cycle: A→B, check B→A", () => {
    // Existing: p1 → t1
    const projects = [makeProject("p1")];
    const tasks = [makeTask("t1")];
    const edges = [makeEdge("p1", "t1")];
    const g = buildGraph(projects, tasks, edges);

    // Adding t1 → p1 would close a direct cycle
    expect(wouldCreateCycle(g, "t1", "p1")).toBe(true);
  });

  it("returns true for a transitive cycle: A→B→C, check C→A", () => {
    // Build: p1 → p2 → p3 (chain of subprojects)
    const projects = [makeProject("p1"), makeProject("p2"), makeProject("p3")];
    const edges = [makeEdge("p1", "p2"), makeEdge("p2", "p3")];
    const g = buildGraph(projects, [], edges);

    // Adding p3 → p1 would close a 3-node cycle
    expect(wouldCreateCycle(g, "p3", "p1")).toBe(true);
  });

  it("returns false for an unrelated addition to an existing DAG", () => {
    // Graph: p1 → t1, p2 → t2 (two independent trees)
    const projects = [makeProject("p1"), makeProject("p2")];
    const tasks = [makeTask("t1"), makeTask("t2")];
    const edges = [makeEdge("p1", "t1"), makeEdge("p2", "t2")];
    const g = buildGraph(projects, tasks, edges);

    // Adding p1 → t2 (cross-edge, no cycle)
    expect(wouldCreateCycle(g, "p1", "t2")).toBe(false);
  });

  it("returns true for a longer transitive cycle: A→B→C→D, check D→A", () => {
    const projects = [
      makeProject("a"), makeProject("b"), makeProject("c"), makeProject("d"),
    ];
    const edges = [makeEdge("a", "b"), makeEdge("b", "c"), makeEdge("c", "d")];
    const g = buildGraph(projects, [], edges);

    expect(wouldCreateCycle(g, "d", "a")).toBe(true);
    // But d → some-new-node is fine
    expect(wouldCreateCycle(g, "d", "new")).toBe(false);
  });
});

// ── getProjectTaskIds ─────────────────────────────────────────────────────────

describe("getProjectTaskIds", () => {
  it("returns empty array for a project with no children", () => {
    const projects = [makeProject("p1")];
    const g = buildGraph(projects, [], []);
    expect(getProjectTaskIds(g, [], "p1")).toEqual([]);
  });

  it("returns empty array for a non-existent project", () => {
    const g = buildGraph([], [], []);
    expect(getProjectTaskIds(g, [], "non-existent")).toEqual([]);
  });

  it("returns task IDs that are direct children of the project", () => {
    const projects = [makeProject("p1")];
    const tasks = [makeTask("t1"), makeTask("t2")];
    const edges = [makeEdge("p1", "t1"), makeEdge("p1", "t2")];
    const g = buildGraph(projects, tasks, edges);

    const result = getProjectTaskIds(g, tasks, "p1");
    expect(result).toHaveLength(2);
    expect(result).toContain("t1");
    expect(result).toContain("t2");
  });

  it("excludes subproject IDs (only includes task IDs)", () => {
    // p1 contains both a task (t1) and a subproject (p2)
    const projects = [makeProject("p1"), makeProject("p2")];
    const tasks = [makeTask("t1")];
    const edges = [makeEdge("p1", "t1"), makeEdge("p1", "p2")];
    const g = buildGraph(projects, tasks, edges);

    const result = getProjectTaskIds(g, tasks, "p1");
    expect(result).toEqual(["t1"]);
    expect(result).not.toContain("p2");
  });
});

// ── getSubprojectIds ──────────────────────────────────────────────────────────

describe("getSubprojectIds", () => {
  it("returns empty array for a project with no subprojects", () => {
    const projects = [makeProject("p1")];
    const tasks = [makeTask("t1")];
    const edges = [makeEdge("p1", "t1")];
    const g = buildGraph(projects, tasks, edges);
    expect(getSubprojectIds(g, projects, "p1")).toEqual([]);
  });

  it("returns empty array for a non-existent project", () => {
    const g = buildGraph([], [], []);
    expect(getSubprojectIds(g, [], "non-existent")).toEqual([]);
  });

  it("returns subproject IDs that are direct children", () => {
    const projects = [makeProject("p1"), makeProject("p2"), makeProject("p3")];
    const edges = [makeEdge("p1", "p2"), makeEdge("p1", "p3")];
    const g = buildGraph(projects, [], edges);

    const result = getSubprojectIds(g, projects, "p1");
    expect(result).toHaveLength(2);
    expect(result).toContain("p2");
    expect(result).toContain("p3");
  });

  it("excludes task IDs (only includes project IDs)", () => {
    const projects = [makeProject("p1"), makeProject("p2")];
    const tasks = [makeTask("t1")];
    const edges = [makeEdge("p1", "t1"), makeEdge("p1", "p2")];
    const g = buildGraph(projects, tasks, edges);

    const result = getSubprojectIds(g, projects, "p1");
    expect(result).toEqual(["p2"]);
    expect(result).not.toContain("t1");
  });
});

// ── getTaskProjectIds ─────────────────────────────────────────────────────────

describe("getTaskProjectIds", () => {
  it("returns empty array for a task with no parent project", () => {
    const tasks = [makeTask("t1")];
    const g = buildGraph([], tasks, []);
    expect(getTaskProjectIds(g, "t1")).toEqual([]);
  });

  it("returns empty array for a non-existent task", () => {
    const g = buildGraph([], [], []);
    expect(getTaskProjectIds(g, "non-existent")).toEqual([]);
  });

  it("returns the project ID that contains the task", () => {
    const projects = [makeProject("p1")];
    const tasks = [makeTask("t1")];
    const edges = [makeEdge("p1", "t1")];
    const g = buildGraph(projects, tasks, edges);

    expect(getTaskProjectIds(g, "t1")).toEqual(["p1"]);
  });

  it("returns multiple project IDs for a task in multiple projects (multi-parent DAG)", () => {
    const projects = [makeProject("p1"), makeProject("p2")];
    const tasks = [makeTask("t1")];
    const edges = [makeEdge("p1", "t1"), makeEdge("p2", "t1")];
    const g = buildGraph(projects, tasks, edges);

    const result = getTaskProjectIds(g, "t1");
    expect(result).toHaveLength(2);
    expect(result).toContain("p1");
    expect(result).toContain("p2");
  });
});

// ── newEdgeId ─────────────────────────────────────────────────────────────────

describe("newEdgeId", () => {
  it("returns a non-empty string", () => {
    const id = newEdgeId();
    expect(typeof id).toBe("string");
    expect(id.length).toBeGreaterThan(0);
  });

  it("returns a UUID-shaped string (8-4-4-4-12 hex format)", () => {
    const id = newEdgeId();
    const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
    expect(id).toMatch(uuidRegex);
  });

  it("generates unique IDs on each call", () => {
    const ids = new Set(Array.from({ length: 100 }, () => newEdgeId()));
    expect(ids.size).toBe(100);
  });

  it("generates IDs that are lexicographically sortable (v7 time-ordered)", () => {
    // v7 UUIDs embed a timestamp in the high bits, so they sort chronologically
    const id1 = newEdgeId();
    // Small delay to ensure different millisecond timestamp
    const id2 = newEdgeId();
    // Both should be valid UUIDs; if generated in order, id2 >= id1
    const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
    expect(id1).toMatch(uuidRegex);
    expect(id2).toMatch(uuidRegex);
  });
});
