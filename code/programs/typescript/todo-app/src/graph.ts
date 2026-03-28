/**
 * graph.ts — Application-level graph layer built on LabeledDirectedGraph.
 *
 * The app maintains a directed acyclic graph (DAG) where every entity
 * (Project, Task) is a node and directed edges express relationships:
 *
 *   "contains"  — a project node owns a task or subproject node
 *
 * Future edge labels (not in V1, reserved for the constraint engine):
 *   "depends-on" — task A cannot start until task B is done
 *   "blocks"     — task A prevents task B from starting (inverse of depends-on)
 *
 * === Two-layer architecture ===
 *
 * Layer 1 — Persistent storage (IDB "edges" store, AppState.edges):
 *   Plain JSON-serializable GraphEdge objects. The source of truth.
 *   These travel through the store / reducer / IndexedDB like any other
 *   domain entity.
 *
 * Layer 2 — In-memory graph (LabeledDirectedGraph):
 *   A queryable in-memory representation rebuilt from Layer 1 whenever
 *   graph queries are needed. Never stored in AppState.
 *
 *   Rebuild with: const g = buildGraph(projects, tasks, edges)
 *   Then query: g.successors("project-id", "contains")
 *
 * === Why not store LabeledDirectedGraph in AppState? ===
 *
 * AppState must be a plain serializable object so it can be persisted to
 * IndexedDB and diffed by React (Object.is reference comparison). A class
 * instance with mutable internal state would break both.
 *
 * === Cycle prevention ===
 *
 * The DAG invariant is enforced at write time. Before the EDGE_ADD reducer
 * case appends an edge, it calls wouldCreateCycle(). If the result is true,
 * the action is a silent no-op — the edge is not added and state is unchanged.
 *
 * The cycle check uses transitiveClosure(toId): if the target node can
 * already reach the source node via existing edges, adding source→target
 * would close a loop.
 *
 * === UUIDs ===
 *
 * Edge ids use v7() from @coding-adventures/uuid. Version 7 UUIDs are
 * time-ordered (Unix timestamp in the high bits), which means:
 *   - Edge records appear in insertion order when iterated from IDB.
 *   - Sorting by id gives chronological order for free.
 *   - Distributed creation (future multi-device sync) avoids collisions.
 */

import { LabeledDirectedGraph } from "@coding-adventures/directed-graph";
import { v7 } from "@coding-adventures/uuid";
import type { Task } from "./types.js";
import type { Project } from "./types.js";

// ── Types ──────────────────────────────────────────────────────────────────

/**
 * EdgeLabel — the semantic role of a directed edge in the application graph.
 *
 * "contains" is the only label in V1. It models the "project owns task/subproject"
 * relationship. Future labels will model task-level constraints (the constraint
 * engine) and resource assignments.
 *
 * Extending this type (e.g. | "depends-on" | "blocks") is the intended way
 * to add new relationship types. The LabeledDirectedGraph supports filtering
 * successors/predecessors by label, so adding a new label is non-breaking.
 */
export type EdgeLabel = "contains";

/**
 * GraphEdge — a single directed edge persisted in IDB "edges".
 *
 * === Fields ===
 *
 * id         — v7() UUID. Time-ordered so IDB cursor iteration gives
 *              insertion order and sorting by id gives chronological order.
 *
 * fromId     — source node id. For "contains" edges this is a Project id.
 *              For future "depends-on" edges this will be a Task id.
 *
 * toId       — target node id. For "contains" edges this is a Task id or
 *              a subproject's Project id.
 *
 * label      — semantic role. Determines what the edge means and how
 *              graph queries interpret it.
 *
 * createdAt  — Unix timestamp (ms) when the edge was first created.
 *
 * === Immutability ===
 *
 * Edges are never updated, only created or deleted. To reassign a task
 * from one project to another:
 *   1. Dispatch EDGE_REMOVE for the old edge (by edgeId).
 *   2. Dispatch EDGE_ADD for the new edge.
 *
 * === IDB indexes ===
 *
 * The "edges" store has two secondary indexes:
 *   "fromId" — fast lookup of all children of a given project
 *   "toId"   — fast lookup of all parents of a given task
 */
export interface GraphEdge {
  id: string;
  fromId: string;
  toId: string;
  label: EdgeLabel;
  createdAt: number;
}

// ── In-memory graph construction ──────────────────────────────────────────

/**
 * buildGraph — reconstruct an in-memory LabeledDirectedGraph from the flat
 * persisted edge list in AppState.
 *
 * Every project and task id is registered as a node before edges are added.
 * This ensures that `successors(projectId)` returns an empty array (not
 * a NodeNotFoundError) for projects that exist but have no children yet.
 *
 * Algorithm:
 *   1. Add a node for each Project id.
 *   2. Add a node for each Task id.
 *   3. For each GraphEdge, add the corresponding labeled edge.
 *      Guard against duplicate fromId/toId pairs with the same label by
 *      checking hasEdge first (LabeledDirectedGraph ignores duplicate adds,
 *      but explicit guarding is clearer).
 *
 * Performance note: O(P + T + E) where P = project count, T = task count,
 * E = edge count. For a typical todo app (< 10,000 records) this is
 * instantaneous. If the dataset grows, consider caching the graph instance
 * with useMemo(() => buildGraph(...), [projects, tasks, edges]).
 *
 * @param projects — all projects from AppState
 * @param tasks    — all tasks from AppState
 * @param edges    — all edges from AppState
 */
export function buildGraph(
  projects: Project[],
  tasks: Task[],
  edges: GraphEdge[],
): LabeledDirectedGraph {
  const g = new LabeledDirectedGraph();

  // Register all node IDs so queries work even for isolated nodes
  for (const p of projects) g.addNode(p.id);
  for (const t of tasks) g.addNode(t.id);

  // Add edges — ensure both endpoints exist as nodes before adding
  // (handles edge records that reference nodes not yet loaded)
  for (const e of edges) {
    if (!g.hasNode(e.fromId)) g.addNode(e.fromId);
    if (!g.hasNode(e.toId)) g.addNode(e.toId);
    if (!g.hasEdge(e.fromId, e.toId, e.label)) {
      g.addEdge(e.fromId, e.toId, e.label);
    }
  }

  return g;
}

// ── DAG invariant ──────────────────────────────────────────────────────────

/**
 * wouldCreateCycle — returns true if adding the edge fromId→toId into the
 * given graph would introduce a directed cycle.
 *
 * === Algorithm ===
 *
 * A directed cycle exists when toId can reach fromId through the existing
 * graph. If it can, adding fromId→toId closes the loop:
 *
 *   fromId → toId → ... → fromId   ← the new edge closes this
 *
 * We detect this by computing transitiveClosure(toId) — the set of all
 * nodes reachable from toId following existing directed edges — and checking
 * if fromId is in that set.
 *
 * Special cases:
 *   - Self-loop (fromId === toId): always a cycle.
 *   - toId not a node yet: no outgoing paths, so no cycle possible.
 *
 * Time complexity: O(V + E) via DFS inside transitiveClosure.
 *
 * @param graph  — current in-memory graph (built from state.edges)
 * @param fromId — proposed source node
 * @param toId   — proposed target node
 */
export function wouldCreateCycle(
  graph: LabeledDirectedGraph,
  fromId: string,
  toId: string,
): boolean {
  // Self-loops are always cycles
  if (fromId === toId) return true;

  // If toId has no outgoing paths, it can't reach fromId
  if (!graph.hasNode(toId)) return false;

  try {
    // transitiveClosure(toId) = all nodes reachable FROM toId.
    // If fromId is in that set, then toId can already reach fromId, so
    // adding fromId→toId would close a directed cycle.
    return graph.transitiveClosure(toId).has(fromId);
  } catch {
    // NodeNotFoundError or unexpected error — conservatively allow the edge.
    return false;
  }
}

// ── Query helpers ──────────────────────────────────────────────────────────

/**
 * getProjectTaskIds — returns the IDs of tasks that are direct "contains"
 * children of the given project.
 *
 * Filters successors to only IDs that appear in the tasks array. This
 * distinguishes task children from subproject children (both are successors
 * of the project node, but tasks exist in state.tasks and subprojects exist
 * in state.projects).
 *
 * Returns an empty array if the project node does not exist.
 *
 * @param graph     — in-memory graph from buildGraph()
 * @param tasks     — current state.tasks
 * @param projectId — the project to query
 */
export function getProjectTaskIds(
  graph: LabeledDirectedGraph,
  tasks: Task[],
  projectId: string,
): string[] {
  if (!graph.hasNode(projectId)) return [];
  const taskIds = new Set(tasks.map((t) => t.id));
  return graph.successors(projectId, "contains").filter((id) => taskIds.has(id));
}

/**
 * getSubprojectIds — returns the IDs of subprojects that are direct "contains"
 * children of the given project.
 *
 * Filters successors to only IDs that appear in the projects array,
 * excluding the project itself to prevent accidental self-reference.
 *
 * Returns an empty array if the project node does not exist.
 *
 * @param graph     — in-memory graph from buildGraph()
 * @param projects  — current state.projects
 * @param projectId — the project to query
 */
export function getSubprojectIds(
  graph: LabeledDirectedGraph,
  projects: Project[],
  projectId: string,
): string[] {
  if (!graph.hasNode(projectId)) return [];
  const projectIds = new Set(projects.map((p) => p.id));
  return graph
    .successors(projectId, "contains")
    .filter((id) => id !== projectId && projectIds.has(id));
}

/**
 * getTaskProjectIds — returns the IDs of all projects that directly contain
 * the given task (via "contains" edges where the task is the target).
 *
 * In V1 this is always a single-element array (tasks have exactly one
 * parent), but the data model supports multi-parent for future use.
 *
 * Returns an empty array if the task node does not exist.
 *
 * @param graph  — in-memory graph from buildGraph()
 * @param taskId — the task to query
 */
export function getTaskProjectIds(
  graph: LabeledDirectedGraph,
  taskId: string,
): string[] {
  if (!graph.hasNode(taskId)) return [];
  return graph.predecessors(taskId, "contains");
}

// ── Edge ID generation ─────────────────────────────────────────────────────

/**
 * newEdgeId — generate a time-sorted v7 UUID string for a new edge's id.
 *
 * v7 UUIDs embed a Unix millisecond timestamp in the high 48 bits, making
 * them naturally sortable by creation time. This means:
 *   - IDB cursor iteration yields edges in insertion order.
 *   - String sorting by id gives chronological order.
 *   - Future distributed edge creation (multi-device sync) avoids collisions
 *     because the random component provides 74 bits of entropy per millisecond.
 */
export function newEdgeId(): string {
  return v7().toString();
}
