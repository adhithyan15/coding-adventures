# task-core

The headless engine behind **task-app**: a general task- and project-management
model in Rust, designed for the *hardest* case — Microsoft Project-class scheduling —
so that every simpler tool is a *restriction* of one rich `Task` entity rather than a
separate data model.

- A **checklist item** is a task with a done-flag and no scheduling.
- A **todo** is a task with a deadline.
- A **kanban card** is a task with a workflow status.
- A **Gantt bar** is a fully-scheduled task.
- A **flowchart node** is a task in the relation graph.

One model, many views. See the full design in
[`code/specs/task-app-data-model.md`](../../../specs/task-app-data-model.md) and the
series overview in [`task-app-overview.md`](../../../specs/task-app-overview.md).

## Where this sits in the stack

`task-core` is the bottom of the task-app layer cake, mirroring Engram:

```
task-core            ← you are here: pure model + (soon) CPM scheduler
  └ task-core-wasm   facade: JSON in / JSON out, props + events
        ├ task-capi  C ABI for native shells
        └ task-wasm  WASM ABI for web / Electron
```

It is **pure and headless**: no I/O, no system clock (the current time is passed in
as `now: u64`), and no id generation (ids are minted by the facade and passed in).
`serde` is behind a feature flag, so the model has zero external dependencies by
default. This matches the house style of `engram-core` and `spreadsheet-core`.

Everything a scheduler *derives* — early/late dates, slack, the critical flag,
rollups, formula values — is **computed, never stored as source of truth**.

## Reuse, not rebuild

`task-core` is assembled from existing repo crates rather than duplicating them:

- [`directed-graph`](../directed-graph) — the CPM dependency network: topological
  order, cycle detection, and affected-set recompute.
- [`datetime-core`](../datetime-core) — civil-date arithmetic (weekday, add-days) for
  the working-time calendar model. `task-core`'s `Date` shares its exact
  representation (days since the Unix epoch).

## What's implemented

- **The data model**: `Task` (with an optional scheduling block, typed custom fields,
  and decision-tree support), dependency and generic links, resources, assignments,
  working-time calendars, workflows, baselines, views, and the root `ProjectState`.
  Typed string-backed ids; serde-optional; JSON round-trip tested.
- **The working-time engine** (`calendar`): calendar-aware working-day tests and the
  `next_working` / `add_working` / `sub_working` / `working_between` walks the
  scheduler measures in — weekend/holiday/off-hours aware, bounded, safe on hostile
  input.
- **The CPM scheduler** (`scheduler`): `schedule(project, project_start)` computes
  per-task early/late dates, total/free slack, and the critical path via forward and
  backward passes over `directed-graph`, honouring FS/SS/FF/SF links with lag,
  working-time calendars, the common date constraints, and summary rollups. Rejects
  cyclic networks; surfaces constraint conflicts.
- **Computed fields** (`formula`): a `[field]` bracket-syntax parser → `symbolic-ir`,
  panic-safe `symbolic-vm` evaluation of formula fields, direct-Rust rollups, and a
  `directed-graph` field-dependency order that rejects cyclic formulas. The
  named-variable win over A1-only spreadsheet formulas.

Landing next (per the specs): the `TaskCommand` reducer (in review) and resource
leveling, then Track B (the `task-core-wasm` facade → `task-capi`/`task-wasm` →
Mosaic web app).

```rust
use task_core::{scheduler, Date};
// let result = scheduler::schedule(&project, Date::from_ymd(2026, 7, 13).unwrap())?;
// result.dates[&task_id].critical, .total_slack, .early_start, …
```

## Usage

```rust
use task_core::{ProjectState, ProjectId, Task, TaskId};

// The facade mints ids; here we use fixed strings.
let mut project = ProjectState::empty(ProjectId::from_raw("p1"));
let task = Task::new(TaskId::from_raw("t1"), "Write the spec");
project.tasks.insert(task.id.clone(), task);
```

With the `serde` feature, `ProjectState` (and every entity) serialises to camelCase
JSON with ids as bare strings — the wire contract the host adapters consume.

## Testing

```bash
cargo test -p task-core                 # zero-dependency default model
cargo test -p task-core --all-features  # + serde JSON round-trip
```
