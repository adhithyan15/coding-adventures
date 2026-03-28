# Spec: Graph-Based Projects — todo app

## Status: Accepted

## Context

The todo app stores tasks as a flat `Task[]` array with no hierarchy and no
way to group related work. This spec introduces **Projects** as first-class
persisted entities backed by a **directed acyclic graph (DAG)**. The graph
is the foundation for future constraint propagation (e.g. "task B cannot
start until task A is done") — once every entity is a node, adding new edge
types (`"depends-on"`, `"blocks"`) requires no structural changes.

V1 is intentionally **UI-invisible**: tasks display identically. A default
built-in project is seeded automatically so all existing and new tasks have
a home.

---

## Motivation

```
Flat model (today):          Graph model (after this spec):

tasks: Task[]                projects: Project[]  (IDB "projects")
                             tasks: Task[]        (IDB "todos")
                             edges: GraphEdge[]   (IDB "edges")

                             Project ──"contains"──► Task
                             Project ──"contains"──► Project (subproject)
```

The graph model unlocks, in future specs:
- Project views (filter tasks by project)
- Task dependency edges (`"depends-on"`, `"blocks"`)
- Constraint engine: propagate deadlines, warn on blocking cycles
- Critical path analysis using topological sort

---

## Packages Used

### `@coding-adventures/directed-graph`

`LabeledDirectedGraph` is the core in-memory graph. Methods used:

| Method | Purpose |
|--------|---------|
| `addNode(id)` | Register a project or task node |
| `removeNode(id)` | Deregister on delete |
| `hasNode(id)` | Guard before add/remove |
| `addEdge(from, to, label)` | Add a directed labeled edge |
| `removeEdge(from, to, label)` | Remove a directed labeled edge |
| `hasEdge(from, to, label?)` | Check edge existence |
| `successors(id, label?)` | Get direct children (optionally by label) |
| `predecessors(id, label?)` | Get direct parents (optionally by label) |
| `transitiveClosure(id)` | All nodes reachable from id (for cycle detection) |
| `topologicalSort()` | Linearize for future scheduling |
| `hasCycle()` | Sanity check after bulk load |

The `LabeledDirectedGraph` instance is **never stored in AppState**. It is a
derived, in-memory view rebuilt from the flat `GraphEdge[]` in AppState whenever
graph queries are needed.

### `@coding-adventures/uuid`

`v7()` generates sortable, time-ordered UUIDs for edge `id` fields. This
makes edge records appear in insertion order when iterated from IDB.

---

## Data Model

### `EdgeLabel`

```typescript
/**
 * EdgeLabel — the semantic role of a directed edge.
 *
 * "contains" — the source node (project) owns the target node (task or
 *              subproject). This is the only label in V1.
 *
 * Future labels (not implemented here, reserved for constraint engine):
 *   "depends-on" — source task cannot start until target task is done
 *   "blocks"     — source task prevents target from starting (inverse)
 */
export type EdgeLabel = "contains";
```

### `GraphEdge`

```typescript
/**
 * GraphEdge — a single directed edge in the application graph.
 *
 * id         — v7() UUID. Time-ordered so IDB iteration gives insertion order.
 * fromId     — source node identifier (project id)
 * toId       — target node identifier (task id or project id)
 * label      — semantic role of the edge (see EdgeLabel)
 * createdAt  — Unix timestamp (ms) when edge was added
 *
 * === Storage ===
 *
 * Persisted in IDB "edges" store (keyPath: "id"). Two indexes:
 *   - "fromId" — look up all children of a project
 *   - "toId"   — look up all parents of a task
 *
 * === Immutability ===
 *
 * Edges are not updated, only created or deleted. To re-assign a task to a
 * different project: EDGE_REMOVE old edge, EDGE_ADD new edge.
 */
export interface GraphEdge {
  id: string;
  fromId: string;
  toId: string;
  label: EdgeLabel;
  createdAt: number;
}
```

### `Project`

```typescript
/**
 * Project — a named collection of tasks and/or subprojects.
 *
 * id         — stable UUID, used in edges as fromId
 * name       — display name shown in future project views
 * isBuiltIn  — true for the "Default" project; prevents deletion from UI
 * createdAt  — Unix timestamp (ms)
 * updatedAt  — Unix timestamp (ms), bumped on every PROJECT_UPSERT
 *
 * === V1 constraints ===
 *
 * In V1, a task belongs to exactly one project. The data model allows
 * multi-parent (a task can appear in multiple projects' successors), but
 * the V1 UI and action creators enforce single-parent by always using the
 * default project for new tasks unless a projectId is explicitly passed.
 */
export interface Project {
  id: string;
  name: string;
  isBuiltIn: boolean;
  createdAt: number;
  updatedAt: number;
}
```

### `AppState` (expanded)

```typescript
export interface AppState {
  tasks: Task[];
  views: SavedView[];
  calendars: CalendarSettings[];
  projects: Project[];   // NEW — persisted in IDB "projects"
  edges: GraphEdge[];    // NEW — persisted in IDB "edges"
  activeViewId: string;
}
```

---

## Graph Helpers (`src/graph.ts`)

### `buildGraph`

```typescript
/**
 * buildGraph — reconstruct an in-memory LabeledDirectedGraph from the
 * flat persisted edge list in AppState.
 *
 * All project and task IDs are added as nodes before edges so that
 * `successors` / `predecessors` work even for nodes with no edges yet.
 *
 * Usage:
 *   const g = buildGraph(state.projects, state.tasks, state.edges);
 *   const taskIds = getProjectTaskIds(g, state.tasks, "default");
 */
export function buildGraph(
  projects: Project[],
  tasks: Task[],
  edges: GraphEdge[],
): LabeledDirectedGraph
```

### `wouldCreateCycle`

```typescript
/**
 * wouldCreateCycle — returns true if adding the edge fromId→toId into the
 * given graph would introduce a cycle.
 *
 * Algorithm:
 *   1. Self-loop check: fromId === toId → always a cycle.
 *   2. If toId is not yet a node in the graph, it has no outgoing paths,
 *      so the new edge can't close a loop → false.
 *   3. Use transitiveClosure(toId): the set of all nodes reachable from
 *      toId following existing edges.
 *      If fromId is in that set, then toId already reaches fromId, so
 *      adding fromId→toId would close a loop → true.
 *      Otherwise → false.
 *
 * The reducer uses this before applying EDGE_ADD; if it returns true,
 * the action is a no-op and state is returned unchanged.
 *
 * @param graph — the current in-memory graph (built from state.edges)
 * @param fromId — proposed source node
 * @param toId   — proposed target node
 */
export function wouldCreateCycle(
  graph: LabeledDirectedGraph,
  fromId: string,
  toId: string,
): boolean
```

### Query helpers

```typescript
/**
 * getProjectTaskIds — IDs of tasks that are direct "contains" children of
 * the given project. Filters successors to only IDs present in tasks[].
 */
export function getProjectTaskIds(
  graph: LabeledDirectedGraph,
  tasks: Task[],
  projectId: string,
): string[]

/**
 * getSubprojectIds — IDs of projects that are direct "contains" children of
 * the given project. Filters successors to only IDs present in projects[].
 */
export function getSubprojectIds(
  graph: LabeledDirectedGraph,
  projects: Project[],
  projectId: string,
): string[]

/**
 * getTaskProjectIds — IDs of all projects that directly contain the task.
 * Returns predecessors of taskId via "contains" edges.
 */
export function getTaskProjectIds(
  graph: LabeledDirectedGraph,
  taskId: string,
): string[]

/**
 * newEdgeId — generate a time-sorted v7 UUID string for a new edge's id.
 */
export function newEdgeId(): string
```

---

## Actions

### New action constants

```typescript
export const PROJECT_UPSERT = "PROJECT_UPSERT";
export const EDGE_ADD       = "EDGE_ADD";
export const EDGE_REMOVE    = "EDGE_REMOVE";
```

### New action creators

```typescript
/**
 * projectUpsertAction — create or update a project.
 *
 * If a project with the same id already exists, it is replaced.
 * isBuiltIn projects are seeded at startup; the UI should not allow
 * deletion of isBuiltIn=true projects.
 */
export function projectUpsertAction(project: Project): Action

/**
 * edgeAddAction — add a directed edge between two nodes.
 *
 * The reducer performs a cycle check before applying the edge.
 * If the edge would create a cycle, the action is silently ignored.
 */
export function edgeAddAction(edge: GraphEdge): Action

/**
 * edgeRemoveAction — remove an edge by its id.
 *
 * Does nothing if the edge does not exist (idempotent).
 */
export function edgeRemoveAction(edgeId: string): Action
```

### Modified action creator

```typescript
/**
 * createTaskAction — extended with optional projectId.
 *
 * @param projectId — which project this task belongs to (default: "default").
 *   The reducer atomically creates both the task and a "contains" edge from
 *   projectId → task.id. No second dispatch is needed from the UI.
 */
export function createTaskAction(
  title: string,
  description: string,
  priority: Priority,
  category: string,
  dueDate: string,
  dueTime?: string | null,
  projectId?: string,       // NEW — defaults to PROJECT_ID_DEFAULT
): Action
```

---

## Reducer Changes

### `TASK_CREATE` — atomic edge addition

```
On TASK_CREATE:
  1. Append the new Task to state.tasks (existing logic)
  2. Create a new GraphEdge:
       { id: newEdgeId(), fromId: payload.projectId ?? "default",
         toId: task.id, label: "contains", createdAt: Date.now() }
  3. Append the edge to state.edges
  4. Return { ...state, tasks: [...], edges: [...] }

The cycle check is skipped here because a brand-new task node has no
outgoing edges, so the edge project→task can never create a cycle.
```

### `TASK_DELETE` — cascade edge removal

```
On TASK_DELETE:
  1. Remove the task from state.tasks (existing logic)
  2. Filter out all edges where edge.toId === taskId OR edge.fromId === taskId
  3. Return { ...state, tasks: [...], edges: [...] }
```

### `PROJECT_UPSERT`

```
On PROJECT_UPSERT:
  Replace existing project with same id, or append if new.
  (Same pattern as VIEW_UPSERT / CALENDAR_UPSERT)
```

### `EDGE_ADD`

```
On EDGE_ADD:
  1. Build a temp LabeledDirectedGraph from state.edges
  2. Call wouldCreateCycle(graph, edge.fromId, edge.toId)
  3. If cycle detected: return state unchanged (silent no-op)
  4. Otherwise: return { ...state, edges: [...state.edges, edge] }
```

### `EDGE_REMOVE`

```
On EDGE_REMOVE:
  Filter out edge with matching id.
  return { ...state, edges: state.edges.filter(e => e.id !== edgeId) }
```

---

## Persistence Changes

### `PROJECT_UPSERT`

```typescript
case PROJECT_UPSERT:
  storage.put("projects", action.payload.project).catch(warn);
  break;
```

### `EDGE_ADD`

```typescript
case EDGE_ADD:
  // Only persist if the reducer actually added it
  // (cycle check may have rejected it — re-check by looking at new state)
  const newState = store.getState();
  const edgeExists = newState.edges.some(e => e.id === action.payload.edge.id);
  if (edgeExists) {
    storage.put("edges", action.payload.edge).catch(warn);
  }
  break;
```

### `EDGE_REMOVE`

```typescript
case EDGE_REMOVE:
  storage.delete("edges", action.payload.edgeId).catch(warn);
  break;
```

### `TASK_CREATE` — persist the auto-created edge

```typescript
case TASK_CREATE: {
  // persist the task (existing)
  const newState = store.getState();
  const task = newState.tasks[newState.tasks.length - 1];
  storage.put("todos", task).catch(warn);
  // persist the auto-created edge
  const edge = newState.edges.find(e => e.toId === task.id);
  if (edge) storage.put("edges", edge).catch(warn);
  break;
}
```

### `TASK_DELETE` — cascade edge delete

```typescript
case TASK_DELETE: {
  const taskId = action.payload.taskId;
  storage.delete("todos", taskId).catch(warn);
  // Delete all edges whose toId or fromId was the deleted task.
  // We compare pre-delete vs post-delete edge lists.
  // The pre-delete IDs are captured before calling next(); post-delete
  // is read from store.getState() after. Any ID missing from post is deleted.
  // (Implementation: capture state.edges IDs before next(), diff after.)
  break;
}
```

---

## IDB Schema: v3 → v4

```typescript
// main.tsx — IndexedDBStorage version bump
version: 4,
stores: [
  { name: "todos",     keyPath: "id", indexes: ["status", "priority", "category"] },
  { name: "views",     keyPath: "id" },
  { name: "calendars", keyPath: "id" },
  { name: "events",    keyPath: "id" },      // v3
  { name: "snapshots", keyPath: "id" },      // v3
  { name: "projects",  keyPath: "id" },      // v4 NEW
  { name: "edges",     keyPath: "id",        // v4 NEW
    indexes: ["fromId", "toId"] },
],
```

Note: `IndexedDBStorage` guards each store creation with
`if (!db.objectStoreNames.contains(...))` so existing v3 databases upgrade
cleanly.

---

## Data Migration (v3 → v4)

After loading all entities from IDB, in `main.tsx`:

```typescript
// If no projects exist, this is either a first visit or a v3→v4 upgrade.
// In both cases, seed the default project and assign all existing tasks to it.
if (projects.length === 0) {
  seedDefaultProject(store);
  for (const task of tasks) {
    store.dispatch(edgeAddAction({
      id: newEdgeId(),
      fromId: PROJECT_ID_DEFAULT,
      toId: task.id,
      label: "contains",
      createdAt: Date.now(),
    }));
  }
}
```

This runs through the full middleware stack, so the default project and all
edges are persisted to IDB in the same pass.

---

## Default Project

```typescript
// src/seed.ts
export const PROJECT_ID_DEFAULT = "default";

/**
 * seedDefaultProject — dispatch a PROJECT_UPSERT for the built-in default
 * project. Called during v3→v4 migration and on first visit.
 *
 * The id is stable ("default") so future code can reference it by constant
 * without querying IDB.
 */
export function seedDefaultProject(store: Store<AppState>): void {
  store.dispatch(projectUpsertAction({
    id: PROJECT_ID_DEFAULT,
    name: "Default",
    isBuiltIn: true,
    createdAt: 0,
    updatedAt: 0,
  }));
}
```

---

## BUILD File (transitive dep chain)

The `uuid` package depends on `sha1` and `md5` via `file:` references.
Per the transitive-deps lesson, all must be installed before `todo-app`:

```bash
cd ../../../packages/typescript/sha1 && npm install --quiet && \
cd ../md5 && npm install --quiet && \
cd ../uuid && npm install --quiet && \
cd ../directed-graph && npm install --quiet && \
cd ../../../programs/typescript/todo-app && npm install --quiet && \
npx vitest run
```

---

## UI Changes

**None.** All existing components (TaskList, TaskCard, TaskEditor, FilterBar,
KanbanView, CalendarViewWrapper) remain unchanged. The "Default" project and
the entire edge list are invisible to the user in V1.

---

## Tests

### New: `src/__tests__/graph.test.ts`

| Test | Description |
|------|-------------|
| buildGraph empty | Returns graph with no nodes |
| buildGraph with nodes | All project + task IDs added as nodes |
| buildGraph with edges | Edges present and queryable via successors() |
| wouldCreateCycle self-loop | fromId === toId → true |
| wouldCreateCycle new node | toId not in graph → false |
| wouldCreateCycle no cycle | A→B, check B→C → false |
| wouldCreateCycle direct cycle | A→B, check B→A → true |
| wouldCreateCycle transitive cycle | A→B→C, check C→A → true |
| getProjectTaskIds | Returns task IDs only (not subproject IDs) |
| getSubprojectIds | Returns project IDs only (not task IDs) |
| getTaskProjectIds | Returns parent project IDs |
| newEdgeId | Returns valid UUID string |

### Updated: `src/__tests__/reducer.test.ts`

| Test | Description |
|------|-------------|
| Initial state | projects: [], edges: [] present |
| PROJECT_UPSERT new | Project added to state |
| PROJECT_UPSERT existing | Project replaced by id |
| EDGE_ADD valid | Edge appended |
| EDGE_ADD cycle | Edge rejected, state unchanged |
| EDGE_REMOVE | Edge removed by id |
| TASK_CREATE | Task added AND edge project→task appended atomically |
| TASK_DELETE | Task removed AND all edges with that id cascade-removed |

### Updated: `src/__tests__/actions.test.ts`

- `projectUpsertAction` — returns correct type + payload
- `edgeAddAction` — returns correct type + payload
- `edgeRemoveAction` — returns correct type + payload
- `createTaskAction` with explicit `projectId` — payload contains projectId

### Updated: `src/__tests__/persistence.test.ts`

- `PROJECT_UPSERT` → `storage.put("projects", project)` called
- `EDGE_ADD` (accepted) → `storage.put("edges", edge)` called
- `EDGE_ADD` (cycle rejected) → `storage.put("edges", ...)` NOT called
- `EDGE_REMOVE` → `storage.delete("edges", edgeId)` called
- `TASK_CREATE` → both task and edge persisted
- `TASK_DELETE` → task deleted AND cascade edges deleted

---

## Invariants

1. **DAG**: No edge can be added if it would create a cycle. The reducer
   enforces this via `wouldCreateCycle` on every `EDGE_ADD`.

2. **Referential integrity**: When a task is deleted, all edges referencing it
   (as fromId or toId) are also deleted. When a project is deleted (future),
   all its outgoing edges must also be cleaned up.

3. **Default project always exists**: After any migration or first visit,
   `state.projects` contains at least one project with `id === "default"`.

4. **All tasks have a parent**: After migration, every task has at least one
   incoming "contains" edge. New tasks added via `createTaskAction` always get
   an edge from the specified (or default) project.
