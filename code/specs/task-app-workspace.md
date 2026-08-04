# task-app engine: Workspace + nested, schedulable projects (Phase 2)

**Status:** design (spec-first; committed before implementation)
**Phase:** 2 of [task-app-super-app](task-app-super-app.md)
**Touches:** `task-core` (model, scheduler, ops, projections), `task-wasm` (surface, snapshot/load)

Today `task-core` models **exactly one project**: `ProjectState` is a flat, id-keyed
container (`tasks`, `dependencies`, `resources`, `calendars`, …) and the scheduler runs
over that one project. This phase makes **projects first-class and plural**: multiple
projects in a workspace, projects that **nest recursively**, and — the crux — projects
that **schedule as one network** (dependencies may cross project boundaries; sub-project
dates/work/cost roll up into their parent). This is MS-Project master/subprojects and
Primavera EPS, unified, and it is the foundation everything "projecty" in the super app
builds on.

It must honor the two standing principles from the master spec: **progressive
disclosure** (a single simple project must behave exactly as it does today; nesting and
cross-project scheduling are opt-in) and **fat engine, dumb UI** (all of this lives in
the engine; hosts just render projections).

---

## 1. Model

### 1.1 New container: `Workspace`
```rust
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    /// Every project in the workspace, by id. Nesting is expressed by
    /// ProjectState.parent, NOT by physical containment — relationships stay
    /// by-id, matching the rest of the model (see model.rs header note).
    pub projects: BTreeMap<ProjectId, ProjectState>,
    /// The root ordering of top-level projects (parent == None), for stable display.
    pub roots: Vec<ProjectId>,
    /// Dependencies whose endpoints live in *different* projects. Intra-project
    /// dependencies stay on each ProjectState.dependencies, unchanged.
    pub cross_project_dependencies: Vec<DependencyLink>,
    /// A shared resource pool assignable across projects (for cross-project
    /// leveling later). Per-project ProjectState.resources remain valid for
    /// project-local resources; the pool is additive.
    pub shared_resources: BTreeMap<ResourceId, Resource>,
    pub settings: WorkspaceSettings,
}
```

Add `WorkspaceId` via the existing `id_type!` macro (`ids.rs`).

### 1.2 Nesting on `ProjectState`
Add one field:
```rust
pub parent: Option<ProjectId>,   // None = top-level project
```
A project with `parent = Some(p)` is a **sub-project** of `p`. The nesting forest is
`roots` + each project's `parent`. Cycles in the project forest are rejected by ops
(reuse the `WouldCycle` machinery already used for task hierarchy / dependencies).

`ProjectState` is otherwise unchanged, so **a lone project is exactly today's model** —
critical for backward compatibility and for the "start simple" promise.

### 1.3 Task id uniqueness — **workspace-global**
A `TaskId` is unique across the whole workspace, not just within a project. This is the
single most important simplifying decision: a cross-project dependency is then just a
normal `DependencyLink { predecessor, successor, kind, lag }` whose two `TaskId`s happen
to resolve to tasks in different projects — no `(ProjectId, TaskId)` compound keys, no
new link type. The host already mints unique ids (`t1`, `t2`, …); the engine enforces
global uniqueness in `create_task`. A `task_project: BTreeMap<TaskId, ProjectId>` index
(derived, not stored) answers "which project owns this task?" in O(1) during scheduling.

---

## 2. Scheduling as one network

The current `schedule(project, project_start)` becomes the intra-project special case of
a new workspace-level pass. The generalized algorithm:

1. **Build one directed graph over all tasks in all projects.** Nodes = every scheduling
   task across every `ProjectState` (summaries excluded from the network as today).
   Edges = the union of (a) each project's `dependencies` and (b) the workspace's
   `cross_project_dependencies`. Reuse `directed-graph` for topo order + cycle detection
   over this union — a cross-project cycle is detected exactly like an intra-project one.
2. **Forward/backward pass** identical to today (Early/Late Start/Finish, slack, critical
   path), but each task walks **its own project's calendar** (a task uses its project's
   `project_calendar` unless it has a task/resource calendar override). Cross-project
   edges just constrain successor start by predecessor finish + lag, across calendars.
3. **Rollups, two levels:**
   - *Leaf → summary* within each project — unchanged (`rollup_summaries`).
   - *Project → parent project* — a sub-project rolls up into its parent like a summary
     rolls up its children: parent project span = min child start … max child finish;
     work/cost/percent-complete aggregate up the project forest via `parent`.
4. **Result:** `WorkspaceSchedule { project_start, per_task: BTreeMap<TaskId, TaskDates>,
   per_project: BTreeMap<ProjectId, ProjectRollup>, conflicts: Vec<Conflict>, critical:
   … }`. `ProjectState::schedule()` stays as a thin wrapper that schedules a one-project
   workspace, so existing callers and tests are unaffected.

**Progressive disclosure holds:** a project (or the whole workspace) with no durations /
dependencies produces empty schedule data and simply never appears on a Gantt — the board
and list views work on status alone, untouched by any of this.

**Calendars across projects.** Each project keeps its own calendar set. A cross-project
dependency is evaluated in instant-space (the engine's `Instant` = absolute minutes), so
predecessor-finish → successor-start is well-defined even when the two projects use
different working weeks; the successor's start is then snapped to *its* calendar's next
working time.

---

## 3. Operations (`ops.rs`)

New validated methods on `Workspace`, all returning `Result<(), OpError>` and reusing the
existing `OpError { NotFound, Duplicate, WouldCycle, Invalid }`:
- `create_project(id, name, parent)` / `rename_project` / `delete_project` (delete detaches
  or cascades children — spec: **reject** delete of a project with children unless empty,
  to avoid silent data loss; the host moves children out first).
- `nest_project(child, new_parent)` / `unnest_project(child)` — re-parent, cycle-checked.
- `move_task(task, to_project)` — reassign a task's owning project (updates the derived
  index; intra-project deps that now cross a boundary migrate to
  `cross_project_dependencies`, and vice-versa — handled inside the op so callers don't
  reason about it).
- `link_cross_project_dependency(...)` — same validation as `link_dependency` (no cycle,
  endpoints exist) but records on the workspace.
- Shared-resource ops mirror the per-project resource ops.

Existing per-project ops keep working unchanged when called on a `ProjectState` obtained
from `workspace.projects`.

---

## 4. `task-wasm` surface

Add workspace-level exports mirroring the existing one-project ABI (one export per op /
query, `run_op` pattern):
- `createProject` / `renameProject` / `deleteProject` / `nestProject` / `moveTask` /
  `linkCrossProjectDependency` / shared-resource ops.
- `workspaceSchedule` / `workspaceGantt` (and later `table`/`calendar` at workspace scope).
- **Snapshot/load become workspace-level**: `snapshot()` serializes the whole `Workspace`
  (all projects + nesting + cross-project deps + shared pool); `load()` restores it. The
  web host's persistence record (Phase 1) already stores whichever JSON `snapshot()`
  returns, so the persisted blob upgrades transparently — an old single-project snapshot
  loads as a one-project workspace (migration handled in `load`).

The thread-local `STATE` in `task-wasm` changes from `ProjectState` to `Workspace`
(seeded with one empty project named "project" so first-run behavior is identical).

---

## 5. Backward compatibility & migration
- A fresh workspace has exactly one empty project → the current app behaves identically.
- `ProjectState::schedule()` / `gantt()` / `todos()` / … all remain, operating on a single
  project, so the web host and every existing test keep passing without change.
- `load()` accepts both shapes: a bare `ProjectState` JSON (today's snapshot) is wrapped
  into a one-project `Workspace`; a `Workspace` JSON loads directly. This keeps Phase-1
  persisted data readable.

---

## 6. Tests (`cargo test -p task-core`)
- Cross-project FS/SS/FF/SF + lag vs hand-computed dates (task in project A drives a task
  in project B; different calendars on each).
- Cross-project **cycle** rejection (A→B→A across the boundary).
- Project-forest rollup: a parent project spans min-start…max-finish of its sub-projects;
  three-level nesting rolls up correctly.
- Project-forest cycle rejection (`nest_project` that would create a loop).
- `move_task` migrates a dependency between `dependencies` and
  `cross_project_dependencies` correctly in both directions.
- Backward-compat: a one-project workspace reproduces the existing single-project schedule
  byte-for-byte (same early/late/slack/critical as `ProjectState::schedule`).
- `load()` of an old bare-`ProjectState` snapshot yields an equivalent one-project
  workspace.

---

## 7. Out of scope (later phases)
- Cross-project **resource leveling** / optimization (needs the constraint-VM work).
- Portfolio dashboards & the view/query layer (`table()`/`calendar()`, filters/sorts/
  grouping) — **Phase 3**.
- Per-project **complexity configuration** UI (board-only ↔ full CPM) — the data hook
  (`WorkspaceSettings` / `ProjectSettings`) is added here; the UI lands with the app shell
  (Phase 9).
- Any Mosaic component work — this phase is engine-only.

---

## 8. Sequencing within Phase 2 (small PRs)
1. `WorkspaceId` + `Workspace`/`ProjectState.parent` model + `ProjectState::schedule`
   wrapper (no behavior change) + serde.
2. Workspace scheduler (union graph, cross-project edges, project-forest rollup) + tests.
3. Workspace ops (create/nest/move/link) + tests.
4. `task-wasm` workspace surface + workspace snapshot/load with single-project migration.
5. Point the web host at the workspace snapshot (still one project) — no UX change yet,
   just proves the migration path end-to-end.
