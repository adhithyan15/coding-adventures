# task-app — A General Task & Project Engine on Mosaic

> **Spec series.** This is the overview. The companion specs are:
> - [`task-app-data-model.md`](task-app-data-model.md) — the entity model (the crown jewel)
> - [`task-app-scheduling-engine.md`](task-app-scheduling-engine.md) — CPM, calendars, leveling
> - [`task-app-constraint-vm-enhancement.md`](task-app-constraint-vm-enhancement.md) — optimization for the constraint VM
> - [`task-app-formula-fields.md`](task-app-formula-fields.md) — computed/rollup fields via `symbolic-vm`
> - [`task-app-architecture.md`](task-app-architecture.md) — core → facade → capi/wasm → Mosaic hosts

## Overview

**task-app** is a general-purpose task- and project-management platform. Its heart is a
single **Rust core engine** — as flexible as Microsoft Project — that every native app *and*
the web app consume identically: the browser and Electron through **WebAssembly**, native
shells (Qt, SwiftUI, Compose, Flutter, WinUI/XAML) through a **C ABI**. One engine, every
platform, identical behavior.

The user interface is authored once in **Mosaic** (this repo's compile-time UI language) and
emitted to all nine host platforms, exactly as the **Engram** app already does. task-app is,
in effect, "Engram's architecture applied to project management."

This project supersedes the exploratory TypeScript/Electron
[`checklist-app`](../programs/typescript/checklist-app) (which remains in place, retrievable via
git). Where checklist-app modeled one narrow thing (decision-tree checklists) in TypeScript,
task-app models the **general case** in Rust and treats checklists as one of many *views*.

## The core thesis: one model, many views

Most task tools pick a shape — a flat list, a kanban board, a Gantt chart — and marry the data
model to it. task-app inverts this. There is **one entity, the Task**, rich enough to be a strict
superset of every shape, and each familiar tool is a **projection** of that one model:

| Familiar tool | Is just a Task with… | Rendered as |
|---|---|---|
| Checklist item | a done-flag, no scheduling | decision-tree list |
| Todo | a due-date / deadline | flat list |
| Kanban card | a workflow `status` | column board |
| Gantt bar | full scheduling (duration, dependencies, calendar) | timeline |
| Flowchart node | membership in the dependency/relation graph | node-edge canvas |
| Spreadsheet row | typed custom fields incl. formulas | table/grid |

Because the model is the union, you never migrate data to "upgrade" a checklist into a project
plan — you just turn on more of the same Task's capabilities and pick a different view.

## Design principles

1. **Model the hardest case first.** We design for Microsoft Project-class scheduling (durations,
   work, four dependency types with lag, eight date constraints, working-time calendars, resources,
   assignments, resource leveling, critical path, baselines). Everything simpler falls out as a
   restriction of the general model. See [`task-app-data-model.md`](task-app-data-model.md).

2. **Reuse the repo's engines; do not build micro-packages.** The scheduling graph, date math,
   formula evaluation, and constraint solving already exist as mature crates. task-app is
   assembled from them. New code is confined to the *domain* (the task model + scheduler) and the
   FFI/UI seams. (Reuse map below.)

3. **Pure, headless, deterministic core.** `task-core` has no I/O, no clock (time is passed in as
   `now: u64`), and `serde` behind a feature flag — the house pattern of `engram-core` and
   `spreadsheet-core`. The same bytes in produce the same bytes out on every platform.

4. **One props/event contract, authored once.** The facade computes a flat slot map (props) and
   handles a typed event envelope; the Mosaic `.mil` interface mirrors it; every host adapter is a
   thin translator. Behavior is defined exactly once, in Rust.

5. **Views are projections, never separate stores.** Checklist/todo/kanban/gantt/flowchart/table
   are computed read-models over the one `ProjectState`, like `computeStats` in the legacy app.

## Reuse map (assembled from existing crates)

| Need | Reused crate | Notes |
|---|---|---|
| Dependency DAG: topo sort, cycle detection, affected-set recalc, parallel levels | `directed-graph` (on `graph`) | powers both CPM ordering and formula recalc |
| Date math, weekday/month arithmetic, day-count, injectable clock | `datetime-core` + `wall-clock` | working-day calendars layer on top |
| Exact/rational/decimal quantities | `numeric-tower` | where exactness matters (cost, unit math) |
| Named-variable formula / computed / rollup fields | `symbolic-vm` + `symbolic-ir` (+ `cas-substitution`) | `StrictBackend`; the win over A1-only spreadsheet-core |
| Feasibility validation + optimization (makespan, leveling) | `constraint-*` stack (enhanced) | see the enhancement spec |
| Persistence | `storage-core` + a backend | copy the `memory-store` record shape |
| Time-sortable IDs | `uuid` (v7) | |
| FFI/WASM/host architecture | Engram pattern | `engram-core` → `engram-core-wasm` → `engram-capi`+`engram-wasm` |

**Genuinely new code** (no gratuitous micro-crates): the `task-core` domain model + CPM scheduler;
working-day/holiday calendars and time-interval types folded *into* `task-core`; the
`task-core-wasm` facade; the `task-capi`/`task-wasm` ABI siblings; optimization enhancements inside
the existing `constraint-*` crates; and the Mosaic UI packages + host adapters.

## Component map

```
task-core            pure domain model + CPM scheduler (no I/O, no clock, forbid(unsafe))
  └ task-core-wasm   facade: TaskSession, dispatch(), get_props(), handle_event()
        ├ task-capi  C ABI (native shells) + task-host-cli sidecar
        └ task-wasm  linear-memory WASM ABI (browser/Electron) + JS loader

code/programs/mosaic/task-app         Mosaic product package (.mil/.mll/.msl) + host adapters
code/packages/mosaic/mosaic-pkg-*     reusable UI components (checklist-runner, todo-list,
                                      task-board, gantt-view, flowchart-view, task-detail, …)
```

The two ABI crates are **siblings over the facade**, adding only marshalling — all props/event
logic lives once in `task-core-wasm`. Porting to a new platform is a new thin adapter, never a
change to core logic. Full detail in [`task-app-architecture.md`](task-app-architecture.md).

## Scope & phasing

The **data model is comprehensive from day one** (so no schema migration later), and the **full
engine** — CPM scheduling *plus* resources, assignments, and resource leveling — is in scope, not
deferred. Implementation proceeds in phases:

- **Phase 0 — Specs** (this series), committed before code, per repo mandate.
- **Phase 1 — Two parallel tracks.** *Track A:* `task-core` model + CPM scheduler + calendars +
  formula fields + resources; begin constraint-VM optimization. *Track B:* facade + `task-capi` +
  `task-wasm` + Mosaic `task-app` + **web & Electron** adapters + **checklist & todo** projections,
  proving the full core→facade→wasm→Mosaic loop.
- **Phase 2 — Convergence.** Full engine through the facade; kanban/gantt/flowchart projections;
  feasibility/leveling surfaced in the UI.
- **Phase 3 — Host fan-out.** The remaining native shells (Qt, SwiftUI, Compose, Flutter, XAML) via
  `task-capi`, matching Engram's nine-host reach.
- **Phase 4 — Polish.** Baselines/variance, resource-leveling UI, `storage-core` persistence,
  per-package CHANGELOG/README, security review, spec-sync.

Each phase is a series of small, focused PRs on feature branches, pulling `origin/main` first.

## Out of scope (noted for later)

- A pure **Win32 / non-XAML Windows** Mosaic emit backend + native host — a separate Mosaic-backend
  effort, independent of this app.
- Portfolio/multi-project rollups, earned-value management, and Monte-Carlo risk analysis
  (Primavera-tier features) — the data model leaves room for them; they are not v1 work.
