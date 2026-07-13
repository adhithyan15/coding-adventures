# task-app super-app — vision, architecture & phased roadmap

**Status:** planning (spec-first, committed before code)
**Supersedes/extends:** [task-app-overview](task-app-overview.md),
[task-app-architecture](task-app-architecture.md), [task-app-data-model](task-app-data-model.md),
[task-app-scheduling-engine](task-app-scheduling-engine.md),
[task-app-formula-fields](task-app-formula-fields.md).

This document is the umbrella plan for turning the shipped task-app (a to-do app with automatic
CPM scheduling, already live on the web over WASM) into a **super app** — a single tool that
absorbs the best of every mainstream task/project manager (Trello, Asana, Notion/Airtable,
Motion/Reclaim, ClickUp, Todoist, MS Project) on top of one shared Rust engine, with every view
authored once in Mosaic and reusable across the repo's hosts.

It records **what we are building, why, what we reuse, what is genuinely new, and the order we
ship it in** — so the work can be finished in independently-shippable pieces.

---

## 1. Vision

One workspace. Many views of the *same* data. A card you drag on a board is the same task that
appears as a row in a sheet, a bar on a Gantt, a block on a calendar, and a node in a flow graph.
You start as simple as a sticky note and add exactly as much structure as you want — no more.

The differentiator is the **engine underneath**: everything that mainstream apps reimplement in
UI (filtering, sorting, grouping, rollups, formulas, formatting, and — uniquely — real
critical-path scheduling) lives in one pure Rust core, so every host and every view gets it for
free and gets it *identically*.

---

## 2. Locked design principles

These are decided and drive every downstream choice.

### 2.1 Fat engine, dumb UI
All computation lives in `task-core`: filtering, sorting, grouping, column selection, formulas,
rollups, **display formatting**, and scheduling. A **projection takes a view/query config and
returns render-ready data** — strings already formatted, rows already grouped and ordered, cells
already computed. Mosaic components are **thin renderers** that draw the projection's output and
**emit raw intent events** (`onCellEdit`, `onCardDrop`, `onMoveTask`). The UI holds no business
logic and no formatting. This keeps every host consistent and makes new hosts nearly free.

> Consequence: the web app's current TypeScript row-formatting must move *into* engine
> projections. Hosts stop transforming; they only draw and emit.

### 2.2 First-class citizens
- **Projects** — containers that **nest recursively** and **participate in scheduling** (§2.4).
- **Tasks** — the one core entity; every view is a projection of tasks.
- **Subtasks** — task hierarchy (already modeled via `parent`); a subtask is just a child task.
- **Notes** — a first-class entity, standalone *and* attachable to any task/project.

### 2.3 Progressive disclosure — simple by default, complexity opt-in
The single most important product rule.
- A card is just a **name** (optionally a status). A **drag-and-drop board works with zero
  scheduling** — no dates, no dependencies, none of the engine's depth surfaced.
- Capability is **added incrementally**: due date → it's a todo; duration → it can auto-schedule;
  dependency → it sequences; resource → it levels. Every layer is optional with sane defaults.
- Scheduling **only engages when scheduling attributes exist**. An unscheduled task never appears
  on the Gantt and never forces CPM on the user.
- **Configurable per project/task**: a project can be set "board only" and never reveal the
  scheduling machinery; another can be a full CPM plan. Same engine, same data, different exposed
  surface. The model already leans this way — `Task.schedule` is `Option`.

### 2.4 Projects nest and schedule as one network
A project is a schedulable container (a summary-like node) that can contain tasks **and other
projects**, recursively. The scheduler runs **one CPM network over the whole tree**: sub-project
dates/work/cost roll up into the parent, and **dependencies may cross project boundaries**
(a task in project A → a task in project B). This unifies MS-Project master/subprojects and
Primavera EPS. A portfolio is just a tree of projects that all schedule together.

### 2.5 Reusable Mosaic components
Each view is a standalone, reusable `mosaic-pkg-*` component (sheet, board, calendar, notes,
project-nav), usable in *other* apps, not just this one. The app is an assembly of components.

### 2.6 Pluggable storage
The workspace serializes to a **pluggable storage backend**. IndexedDB in the browser now
(with an in-memory fallback), swappable later for SQLite / a sync server / cloud — without the
engine knowing. Persistence is **host-owned**; the engine does zero I/O and only exposes
`snapshot()`/`load()`.

---

## 3. Feature superset (from the app-landscape survey)

Organized by the component that owns it. This is the target surface; §7 sequences it.

**Views (each with independent filters, sorts, grouping, column visibility):**
- **Board / kanban** (Trello-grade): lists/columns, cards, drag-and-drop, swimlanes (group-by),
  WIP limits, card details (description, checklist, due, labels, custom fields, cover, comments,
  attachments), quick-add.
- **Sheet / table** (Notion/Airtable-grade): editable cells, 20+ field types, formulas, rollups
  across relations, lookups, sort/filter/group, column show/hide/reorder, row grouping.
- **Calendar**: month/week/day, events, drag-to-reschedule, event resize, time-blocking,
  auto-schedule placement (Motion/Reclaim-style) around fixed commitments by deadline/priority.
- **Timeline / Gantt**: bars, dependencies, critical path, slack, baselines (engine already emits
  `gantt()`).
- **List / checklist**: fast capture, NLP-ish input, priorities, tags, subtasks (engine already
  emits `todos()`/`checklist()`).
- **Flow graph**: dependency/relation graph (engine already emits `flowchart()`).

**Cross-cutting:**
- Labels/tags, priorities, statuses/workflows, recurring tasks, reminders, attachments, comments,
  custom typed fields, relations between records, saved views, automation rules (Butler-style).
- Notes (rich text) as a first-class entity.
- Projects (nested), portfolio rollups, shared resource pool.

---

## 4. Reuse map (what we build on)

| Need | Reuse | Notes / gaps |
|---|---|---|
| Engine (model, ops, scheduler, projections) | `task-core` (on main) | add workspace, view-layer, notes, more field types |
| Web ABI + snapshot/load | `task-wasm` (on main): `snapshot()`, `load(ptr,len)`, `reset()` | persistence seam ready |
| Pluggable storage interface | `@coding-adventures/storage` (`Storage`: open/close/get/getAll/put/delete/query/transaction) | IndexedDB/SQLite/Drive/S3 named as intended backends |
| Browser storage backend | `@coding-adventures/indexeddb` `IndexedDBStorage` (+ versioning, indexes, `renamedFrom` migration) | `query()`/`transaction()` unimplemented; `MemoryStorage` fallback for private-mode/Node |
| Boot/persist pattern | legacy `checklist-app` `main.tsx` + `persistence.ts` middleware | proven: hydrate on boot, per-record fire-and-forget writes |
| Sheet/table view | `mosaic-pkg-grid` v0.2 (Grid/Cell/Column, editable cells, column widths) | no sort/filter/multi-select/frozen-cols/row-DnD; custom cell renderers deferred to v0.3 |
| Forms / nav / dialogs / lists | `mosaic-pkg-toolkit` v0.11 (Input/Select/Checkbox/Radio/NumberInput/Modal/Tabs/Nav/Navbar/Breadcrumb/ListGroup/…), `mosaic-pkg-dialog` | `Card`/`Container` not shipped (blocked on UI29-2) |
| Notes editor | `mosaic-pkg-note-editor` | adapt from Anki-note domain to generic notes |
| Table primitive (native per backend) | kernel `HostTable` family (UI31): `<table>`/SwiftUI Table/WinUI DataGrid/Qt TableView/Flutter DataTable; a11y + RTL + keyboard-nav guaranteed by the platform widget | editable cells via `HostInput` in a cell |

**Not reused:** Rust `storage-core` (sync trait, filesystem/SQLite, server/native-oriented — does
not fit async browser IndexedDB compiled to wasm). Reserve it for a future native/desktop or
server sync backend where its CAS-revision + lease model earns its keep.

---

## 5. The two hard gaps (they shape the sequence)

### 5.1 No drag-and-drop in the Mosaic kernel — **build it first** (decided)
A full-tree grep found **zero** drag/drop primitives anywhere in the kernel, emitters, layout
compiler, or specs. Trello-grade boards (drag cards, drag swimlanes) and calendar
drag-to-reschedule/resize cannot be built today.

**Decision: build the DnD kernel primitive family first, across all backends**, before the board
and calendar components — so drag works identically everywhere from day one. This is a
UI-kernel expansion in the shape of UI31: a spec (`UIxx-host-drag-drop.md`) defining a
`HostDraggable` / `HostDropTarget` family (pointer/drag lifecycle emits: drag-start, drag-over,
drop, drag-end, with payload + position), lowered natively per backend (HTML5 DnD / pointer events
on web; the native drag APIs on SwiftUI/Compose/Qt/Flutter/WinUI). Accessibility (keyboard-driven
move as an equivalent path) is a first-class requirement, not an afterthought.

### 5.2 No multi-project / workspace container — **add to the engine**
`ProjectState` models exactly one project; `task-wasm` holds one global. "Projects as first-class,
nestable, schedulable" requires a new **`Workspace`** layer in `task-core`: a container of projects
with nesting, cross-project dependencies, portfolio rollups, and a shared resource pool, plus
workspace-level ops and a `task-wasm` surface and snapshot/load. Host-level N-snapshots (one live
project at a time) is a stopgap that cannot do cross-project views — so we build the real thing.

**Other engine gaps (additive):** `table()` and `calendar()` projections don't exist yet (only
`checklist/todos/kanban/gantt/flowchart`); the projection API is mid-migration to take a `&View`;
view-config (filter/sort/group/columns) types; notes entity; labels/priorities/recurring;
more field types; move formatting into projections.

---

## 6. Component & crate decomposition

```
task-core (Rust, pure)                     ← model + ops + scheduler + projections + view-layer + workspace + notes
  └ task-wasm (WASM ABI)                    ← alloc/pack + one export per op/query + snapshot/load
        └ web host (React state)            ← thin controller: query engine → props; emit intent → ops
storage (TS)  @coding-adventures/storage    ← pluggable Storage; IndexedDBStorage + MemoryStorage fallback

Mosaic kernel
  └ HostDraggable / HostDropTarget          ← NEW primitive family (built first), all backends

Reusable Mosaic components (mosaic-pkg-*)
  ├ grid            (reuse)                  → sheet/table view
  ├ toolkit         (reuse)                  → forms, nav, dialogs, lists
  ├ note-editor     (reuse/adapt)            → notes
  ├ board           (NEW)                    → kanban (drag, swimlanes, WIP, card details)
  ├ calendar        (NEW)                    → month/week/day, events, drag-to-reschedule
  └ project-nav     (NEW)                    → nested-project tree, view switcher, complexity config

task-app (Mosaic program)                    ← assembles the components into the super app
```

Each component is a thin renderer over an engine projection and emits raw intent; no component
contains business logic or formatting (§2.1).

---

## 7. Phased roadmap (each phase is independently shippable)

Ordered simplest-first, honoring the two decisions (persistence first; DnD kernel before the
board). Every phase = its own spec section/doc → tests → implementation → CHANGELOG/README →
`/security-review` → PR → `/babysit-pr`. Pull `origin/main` before each.

**Phase 0 — this spec.** Umbrella vision, principles, reuse map, decomposition, sequencing.

**Phase 1 — Web persistence (IndexedDB).** *(start here)*
Wire the web app to `@coding-adventures/storage`: `IndexedDBStorage` with `MemoryStorage`
fallback, hydrate on boot via `engine.load(json)`, persist via `engine.snapshot()` →
`storage.put`. Whole-project snapshot per record to start (per-entity decomposition is a later
optimization). Copy the checklist-app boot/persist pattern. **Ships:** tasks survive reloads;
proves the storage seam. No engine change.

**Phase 2 — Engine: Workspace + nested projects.**
Add `Workspace { projects, … }` to `task-core`; nested-project scheduling (cross-boundary deps +
rollups); shared resource pool; workspace ops (create/rename/delete/move/nest project); `task-wasm`
surface; workspace snapshot/load. **Ships:** projects as first-class, nestable, schedulable.

**Phase 3 — Engine: view/query layer + projections.**
`View`/query config types (filter/sort/group/columns); rich projections returning render-ready
data: `table(view)`, `calendar(range, view)`, enhanced `kanban(view)`; move formatting into
projections. Task attributes: labels/tags, priority, recurring, reminders; more field types.
**Ships:** the "fat engine" that every view will consume.

**Phase 4 — DnD kernel primitives.** *(the gating kernel work)*
`UIxx-host-drag-drop.md` + `HostDraggable`/`HostDropTarget` in the kernel; lower on the web/React
backend first (unblocks board+calendar), then the remaining backends; keyboard-move a11y path.
**Ships:** drag-and-drop available to every host.

**Phase 5 — Sheet component.**
`mosaic-pkg-sheet` on `mosaic-pkg-grid`, wired to `table(view)`: editable cells, engine-driven
sort/filter/group, formulas/rollups, column show/hide. **Ships:** a real spreadsheet of tasks.

**Phase 6 — Board component.**
`mosaic-pkg-board`, wired to `kanban(view)` and the DnD primitives: drag cards across columns,
swimlanes (group-by), WIP limits, quick-add, card details (via toolkit + dialog). **Ships:**
Trello-grade board.

**Phase 7 — Calendar component.**
`mosaic-pkg-calendar`, wired to `calendar(range, view)` + DnD: month/week/day, events,
drag-to-reschedule, resize, time-blocking, auto-schedule placement. **Ships:** robust calendar.

**Phase 8 — Notes component + entity.**
Notes entity in the engine + `mosaic-pkg-notes` (adapted `note-editor`): standalone notes and
per-task/project notes. **Ships:** notes as first-class.

**Phase 9 — App-shell assembly + progressive disclosure.**
`mosaic-pkg-project-nav` (nested-project tree, view switcher) + per-project/task complexity
config (board-only ↔ full CPM). Assemble all components into the super app. **Ships:** the
integrated product with simple-by-default UX.

**Phase 10+ — Polish & reach.**
Recurring/reminders UX, automation rules (Butler-style), resource-leveling UI, portfolio
dashboards, relations/rollups across records, then native shells via `task-capi` (SwiftUI/Compose/
Qt/Flutter/WinUI) — the DnD primitives and pure engine make host fan-out mechanical.

---

## 8. Verification discipline (every phase)
- `cargo clippy -p <pkg> --all-features --all-targets -- -D warnings` **before push** (CI denies
  warnings; local `cargo test` alone does not catch this).
- Engine changes: unit tests vs hand-computed expectations (CPM early/late/float, rollups,
  filter/sort/group determinism, formula recalc).
- Web changes: drive the flow live in a browser, confirm zero console errors.
- `/security-review` before each push; `/babysit-pr` after each PR.
- Spec-sync: if implementation diverges, update the relevant spec and note it in the commit.

---

## 9. Open decisions deferred to their phase
- Storage granularity (whole-project snapshot vs per-entity records) — revisit in Phase 1 if
  write volume warrants; the `Storage` interface makes it swappable.
- SQL-over-IndexedDB (`IndexedDBStorage.query()`/`transaction()` are unimplemented) — add the
  load-all + `sql-execution-engine` bridge (as `MemoryStorage` does) only if a view needs it.
- Cross-project dependency semantics at portfolio scale (Phase 2 detail spec).
- DnD payload/positioning contract and per-backend lowering specifics (Phase 4 detail spec).
