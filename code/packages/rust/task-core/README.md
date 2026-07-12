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

`task-core` is the **pure engine** at the bottom of the task-app stack:

```
task-core            ← you are here: the PURE ENGINE — model + operations API + queries
                       + scheduler + calendar + formula. No I/O, no clock, no state runtime.
  ├ task-capi        C ABI exposing the engine's functions to native shells
  └ task-wasm        WASM ABI exposing the same to web / Electron

  …consumed by per-backend NATIVE host bindings: React state on web, @Observable on
  SwiftUI, Compose State, Qt models — each idiomatic. There is deliberately NO universal
  "facade" / dispatch / props-events contract (that would import React-isms into
  platforms that don't have them). See `code/specs/task-app-architecture.md`.
```

It is **pure and headless**: no I/O, no system clock (the current time is passed in
as `now: u64`), no id generation (ids are minted by the host and passed in), and **no
state-management runtime** — it is a library of pure functions over a value. `serde` is
behind a feature flag, so the model has zero external dependencies by default.

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
- **The operations API** (`ops`): every mutation as a validated `ProjectState` method
  (`create_task`, `reparent`, `link_dependency`, `assign`, `add_calendar_exception`, …)
  returning `Result<(), OpError>` — no command/dispatch. Pure, and the single trust
  boundary enforcing invariants (percent clamp, interval bounds, cycle rejection).
- **View projections** (`projections`): pure `&self` queries that render the primitives
  from the one model — `checklist()` (decision-aware), `todos()`, `kanban(workflow)`,
  `gantt(start)`, `flowchart()` — each returning plain serde-friendly data.

With this the engine is feature-complete as **pure computation**. Landing next: the
`task-capi` / `task-wasm` ABIs (thin, exposing the engine's functions), then the Mosaic
UI wired **natively per backend** (React state on web first).

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
