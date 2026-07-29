# Changelog

All notable changes to `task-core` are documented here.

## [0.1.0] - Unreleased

### Added

- **`overdue` built-in column** (Phase 3 PR-6) — `Bool`: the task has a deadline, its
  computed finish falls after it, and it isn't done. Derived in the engine rather than in
  each host, so "show me what's slipping" is a single filter and every UI agrees on the
  answer. (The web host previously computed this in JavaScript.)

- **Labels and priority — two new first-class view dimensions** (Phase 3 PR-5 of
  `code/specs/task-app-view-layer.md`).
  - `Label { id, name, color }` + `LabelId`, registered per project
    (`ProjectState::labels`) and referenced from `Task::labels`, so renaming or
    recolouring a label updates every task at once. `Priority { Low, Normal, High,
    Urgent }` on `Task::priority`, with `rank()`/`label()`/`from_rank()`.
  - Ops: `upsert_label`, `delete_label` (removes the label from every task, so no task is
    left pointing at a deleted label), `set_task_labels` (rejects unknown label/task ids
    and de-duplicates), and `set_priority`.
  - View built-ins: **`priority` resolves to its numeric *rank*, not its name**, so a
    sort orders by urgency (Low→Urgent) instead of alphabetically (High→Low→Normal→
    Urgent); `format_cell` turns the rank back into the display name. **`labels` resolves
    to the label *names*** (comma-joined via the project registry) so filtering and
    searching see what the user sees rather than opaque ids.
  - Both fields are serde-defaulted and skipped when empty, so **pre-label snapshots
    still deserialize** unchanged.

### Fixed

- **Blank values now sort last in *both* directions.** A descending sort previously
  reversed the whole ordering, floating every empty cell to the top — not what "sort by
  priority, highest first" means. Emptiness is now decided before direction is applied,
  so only real values reverse (matching spreadsheets and every board tool). Caught by the
  new priority sort test; covered by a dedicated descending-with-blanks case.

- **The `calendar()` projection — dated events over the same selection** (`view` module —
  Phase 3 PR-4 of `code/specs/task-app-view-layer.md`).
  - `calendar(project, view, range, schedule) -> CalendarView` (and
    `ProjectState::calendar(view, range, project_start)`), with `DateRange { start, end }`
    (inclusive) and `CalendarEvent { task, label, start, finish, all_day, completed,
    overdue, critical }`.
  - The task set and order come from the same `select()` pipeline the table uses, so a
    calendar honours a view's filter exactly like every other shape.
  - **Two kinds of task land on the calendar**, which is what makes it serve both a
    project plan and a plain to-do list: a **scheduled** task contributes its computed
    start…finish span, and an **unscheduled task with a deadline** contributes a one-day
    marker on that deadline. A task with neither simply doesn't appear.
  - Events are clipped to the range by intersection (inclusive both ends), sorted by start
    then label, and flagged `overdue` when they finish past their deadline and aren't done.
    `all_day` is always true today — the model is day-granular; it's reserved for timed
    events (time-blocking) in a later phase.
  - 3 tests (85 total): scheduled + deadline tasks included and undated excluded,
    out-of-range exclusion, and filter-honouring plus the overdue flag.

- **The `table()` (sheet) projection — render-ready rows over the selection** (`view`
  module — Phase 3 PR-3 of `code/specs/task-app-view-layer.md`). A thin map over PR-2's
  `select()`: nothing new is computed, it's just shaped for a spreadsheet host.
  - `table(project, view, schedule) -> TableView` (and `ProjectState::table(view,
    project_start)`). `TableView { columns, groups }`, `ColumnHeader { field, label,
    kind }`, `ColumnKind { Text, Number, Date, Bool }`, `TableGroup { key_label, rows }`,
    `TableRow { task, cells }`, `Cell { value, display }`.
  - Columns come from the view's `visible_fields` (defaulting to a single **Name** column
    when none are set), each with a friendly header **label** and a value **kind** (so a
    host can right-align numbers, draw a checkbox for bools, etc.). `kind` matches how
    `cell()` resolves the field, so it never contradicts the values.
  - Every cell carries **both** the typed `CellValue` *and* its engine-formatted `display`
    string, so a host can render a control or just draw text — no field access, no
    formatting, no sorting on its side. This is the dumb-UI contract the Phase-5 sheet
    component renders.
  - `ProjectState::table`/`view_selection` share a `schedule_or_empty` helper (a cyclic
    network degrades to empty computed columns rather than failing the projection).
  - 3 tests (82 total): column labels + kinds, grouped/ordered/formatted rows (typed
    value and display string both present), and the default name column.

- **The view selection pipeline — filter → sort → group** (`view` module — Phase 3 PR-2
  of `code/specs/task-app-view-layer.md`). Built entirely on `cell()`, so what a view
  filters on, sorts by, and groups on is the same interpretation of a field.
  - `select(project, view, schedule) -> Vec<SelectionGroup>` — applies a `View`'s filter,
    multi-key sort, and grouping, returning ordered, grouped task ids. Summary tasks are
    excluded (they're outline structure, not rows); an ungrouped view returns one group.
  - `SelectionGroup { key, key_label, tasks }` — a group's raw key, its display label
    (via `format_cell`), and its task ids in sort order.
  - Sorting is **multi-key with direction**, tie-broken by outline order then id for
    determinism; `cmp_cell` is a **total order** (`Empty` sorts last, numbers by
    `total_cmp` so `NaN` can't break it, unlike types fall back to a stable type rank, so
    a mixed column never panics). Grouping puts the no-value group **last**.
  - `ProjectState::view_selection(view, project_start)` computes the schedule once so a
    view can filter/sort on **computed columns** (start/finish/slack/critical); a cyclic
    network degrades to an empty schedule rather than failing the selection.
  - The current `Filter` fields (status set / completion / name search) are evaluated
    here; the richer field-predicate tree from the spec layers on in a follow-up.
  - 4 tests: summary exclusion + each filter predicate, multi-key sort with direction and
    Empty-last, group-by with the no-value group last, and computed-column sorting.

- **The view/query layer's shared field accessor** (`view` module — Phase 3 PR-1 of
  `code/specs/task-app-view-layer.md`). The linchpin the whole "fat engine" view layer
  resolves through, so filter, sort, group, and display can never disagree about a
  field's value.
  - `CellValue { Text, Number, Date(Option<Date>), Bool, Empty }` — the comparable,
    formattable common currency of the view layer.
  - `cell(project, task, field, schedule) -> CellValue` — resolves a `FieldRef` to a
    value. Built-ins (`name`/`status`/`kind`/`completed`/`percentComplete`/`duration`/
    `deadline`/`start`/`finish`/`early*`/`late*`/`totalSlack`/`freeSlack`/`critical`)
    read the task or its computed `ScheduledDates`; a task absent from the schedule, or
    an unknown built-in name, yields `Empty` — graceful, never a panic. Custom fields
    read their stored `FieldValue` (money/duration → numeric magnitude for sorting;
    multi-selects joined); computed formula/rollup resolution is layered on in the
    filter/sort PR where the recompute already runs. The built-in catalogue is documented
    inline as the wire contract.
  - `format_cell(value, field, settings) -> String` — render-ready display owned by the
    engine: dates `YYYY-MM-DD`, booleans `✓`/`○`, `percentComplete` with `%`, and the
    working-time built-ins in the project's `DurationUnit` (`3d`/`1.5d`/`8h`/`90m`). This
    is where the web host's row-string formatting will move.
  - 6 tests: every built-in, computed scheduled fields, custom stored values, multi-select
    join, render-ready formatting golden strings, and duration-unit switching.

- **Workspace operations — the validated mutations that only make sense across
  projects** (Phase 2 PR-3 of `code/specs/task-app-workspace.md`). Same
  `Result<(), OpError>` style as the `ProjectState` ops; within-a-project edits are
  still done on the project itself.
  - **Project lifecycle**: `create_project` (roots vs. nested), `rename_project`,
    `delete_project` (rejected while it still has sub-projects, so nested work is never
    lost silently; prunes cross-project edges to its tasks), `nest_project` /
    `unnest_project` (forest-cycle-rejected, roots kept in sync).
  - **`create_task` with workspace-global id uniqueness** — a task id may exist in at
    most one project, so cross-project dependencies (which reference tasks by id alone)
    are always unambiguous. Closes the "duplicate id across projects" gap the Phase-2
    scheduler review flagged.
  - **`move_task`** — relocate a task between projects, migrating dependencies so the
    intra/cross invariant holds: an edge that now straddles the boundary moves into
    `cross_project_dependencies`, and a cross-project edge whose endpoints are now
    co-located collapses into that project's own `dependencies`. The moved task's
    resource assignments and non-scheduling links are dropped from the source (they
    reference the source project's pool/tasks and don't travel), leaving no dangling
    references.
  - **`link_cross_project_dependency` / `unlink_cross_project_dependency`** — validated
    like `link_dependency` but workspace-wide: rejects a self-link, unknown endpoints, a
    *same-project* link (use the project's own op), a duplicate, or a link that would
    cycle the whole-workspace network (cycle check reuses `directed-graph`).
  - **Shared resource pool**: `upsert_shared_resource` / `delete_shared_resource` (the
    latter prunes assignments to it across every project).
  - Helpers `project_is_ancestor` and `cross_project_would_cycle` are bounded /
    `directed-graph`-based, so hostile input can't hang them. 7 new tests.

- **Cross-project scheduling — projects schedule as one CPM network** (Phase 2 PR-2 of
  `code/specs/task-app-workspace.md`).
  - `scheduler::schedule_workspace(ws, project_start) -> WorkspaceSchedule` (and the
    `Workspace::schedule` convenience method) run one CPM pass over **every project's
    tasks at once**. Because task ids are workspace-global, a `DependencyLink` can cross
    a project boundary and sequence a task in one project after a task in another.
  - The CPM pass was refactored around a `Plan` that maps each schedulable task to **its
    owning project**, so calendars and working-time math resolve where the task actually
    lives; predecessor → successor timing is computed in absolute instant-space, so two
    projects on different working weeks compose correctly. `scheduler::schedule` (single
    project) is now the one-project case of the same `run`, and is **unchanged
    behaviourally** — all 8 pre-existing scheduler tests pass byte-for-byte, and a new
    test asserts a one-project workspace reproduces bare-project dates exactly.
  - Cross-project dependencies are honoured only when
    `WorkspaceSettings::schedule_as_one_network` is on; off (the default) schedules each
    project independently from the same start. A cross-project cycle is rejected exactly
    like an intra-project one.
  - `WorkspaceSchedule { dates, per_project, conflicts, project_start, project_finish }`
    and `ProjectRollup { start, finish, critical }`: a two-level rollup — leaf → summary
    within each project, then **project → parent** across the nesting forest, so a parent
    project's span covers its own tasks and every sub-project beneath it.
  - 5 new tests: cross-project sequencing, the independence toggle, cross-project cycle
    rejection, single-project equivalence, and forest rollup.

- **`Workspace` — projects become first-class, plural, and nestable** (Phase 2 PR-1 of
  `code/specs/task-app-workspace.md`). The model layer only; no behaviour changes yet.
  - `Workspace { id, name, projects, roots, cross_project_dependencies,
    shared_resources, settings }` — the container of every project. The `projects` map
    is **flat**; nesting is expressed by id via the new `ProjectState::parent`, matching
    how the rest of the model relates entities. That lets the scheduler (PR-2) build one
    graph over every task without walking an ownership tree, and keeps snapshots simple.
  - `ProjectState::parent: Option<ProjectId>` — `None` means a top-level project (listed
    in `Workspace::roots`). Serialized with `skip_serializing_if`, so root projects omit
    the field entirely and **pre-workspace snapshots still deserialize** (serde default).
  - `WorkspaceId` id newtype; `WorkspaceSettings { schedule_as_one_network }` — defaults
    to `false` (schedule each project independently, i.e. today's behaviour). Turning it
    on is the "incrementally add complexity" step from portfolio-less to portfolio.
  - Helpers: `Workspace::empty` (new-document state: one un-nested project),
    `from_project` (the migration path that wraps a pre-workspace `ProjectState`),
    `children_of`, `ancestors_of` (cycle-guarded so a corrupt snapshot truncates rather
    than hangs), and `project_of_task` (task ids are **workspace-global**, which is what
    lets a cross-project dependency be an ordinary `DependencyLink`).
  - 7 tests: forest construction/traversal, cycle-guard termination, global task
    ownership, single-project wrap equivalence, JSON round-trip with `parent` omitted on
    roots, and old-snapshot compatibility.

- **The data model** — the foundational entity set for a Microsoft Project-class,
  "one model, many views" task engine:
  - `Task` — the central entity, with an optional `TaskSchedule` block (task-type
    triangle, the 8 date constraints, deadline, calendar, actuals), workflow status,
    completion + percent-complete, typed custom-field values, and decision-tree
    support (`Decision`) for checklist parity.
  - `DependencyLink` (FS/SS/FF/SF + lag) driving the CPM network *and* flowchart
    edges; `GenericLink` (blocks/relates/…) for non-scheduling relations.
  - `Resource`, `Assignment` (units + work contour), and a four-layer working-time
    `Calendar` model (weekly pattern + dated exceptions).
  - Typed custom fields: `FieldDef`/`FieldKind` (text/number/bool/date/duration/
    money/select/relation/**formula**/**rollup**) and `FieldValue`.
  - `Workflow` (status state machine), `Baseline` (variance snapshot), and `View`
    (projection descriptor: checklist/todo/kanban/gantt/calendar/flowchart/table).
  - `ProjectState` — the flat, normalised root (entities stored by id, relationships
    by id, never by nesting).
- **Primitives** — `Date` (sharing `datetime-core`'s civil representation, with
  arithmetic delegated to it), `Duration` (working minutes), `Work` (person-minutes),
  and `Money` (exact minor units).
- **Typed string-backed ids** — `TaskId`, `ResourceId`, `CalendarId`, etc.; ids are
  minted by the facade (this core is clock-free and id-free), serialised as bare
  strings so they round-trip losslessly through JavaScript/JSON.
- **serde behind a feature flag** — the default model has zero external dependencies;
  the `serde` feature emits camelCase JSON. A JSON round-trip test proves the wire
  contract (camelCase field names, transparent string ids).

- **The working-time engine** (`calendar` module) — the unit the CPM scheduler
  measures in. Represents time as **instants** (integer minutes since the epoch) for
  clean half-open interval arithmetic, and provides:
  - `is_working_day` — calendar-aware working-day test (weekly pattern + holiday/
    overtime exceptions).
  - `next_working` — snap an instant forward to the next working minute.
  - `add_working` — advance an instant by a `Duration` in working time (skipping
    weekends/holidays/off-hours), with elapsed-duration and zero-duration handling.
  - `working_between` — count working minutes in a half-open interval (for slack).
  - Built on `datetime-core` weekday math; an absent calendar safely degrades to 24/7;
    walks are bounded so a no-working-time calendar can never hang.
  - `sub_working` — the inverse of `add_working` (walk working time *backward*),
    powering the backward pass, finish-anchored constraints, and FF/SF links.

- **The CPM scheduler** (`scheduler` module) — turns the stored inputs into a
  schedule via the Critical Path Method:
  - `schedule(project, project_start) -> ScheduleResult` — per-task `ScheduledDates`
    (early/late start & finish, total & free slack, `critical`), plus `conflicts` and
    the project finish; returns a `Cycle` error for a non-acyclic network.
  - Dependency ordering + cycle detection reuse `directed-graph`
    (`topological_sort`/`has_cycle`); no graph algorithm is reimplemented.
  - **Forward pass** (early dates) and **backward pass** (late dates, slack, critical
    path) honouring all four link types (FS/SS/FF/SF) with lag, in working time.
  - Date constraints: `AsSoonAsPossible`, `StartNoEarlierThan`, `MustStartOn`,
    `FinishNoEarlierThan` are applied; `StartNoLaterThan`/`FinishNoLaterThan`/
    `MustFinishOn`/`deadline` raise conflict flags. Summary tasks roll up to span
    their descendants.
  - First-cut limitations (refined in a follow-up): `AsLateAsPossible` is treated as
    `AsSoonAsPossible`; negative lag (lead) is applied in elapsed time; resource
    leveling is not yet applied.

- **Computed fields** (`formula` module) — formula and rollup fields, reusing the
  repo's named-variable expression stack:
  - A `[field]` bracket-syntax `parse` (our surface syntax, matching MS Project's
    `[Field]` convention) → `symbolic-ir` expression tree; supports `+ - * /`, unary
    minus, comparisons, and parentheses with correct precedence.
  - `referenced_fields` — extracts a formula's dependencies (the field names it reads)
    by walking the AST.
  - `eval_number` — evaluates a formula against name→value bindings via `symbolic-vm`'s
    `StrictBackend`. **Panic-safe on untrusted input**: unbound references and runtime
    errors (division by zero — which `StrictBackend` panics on) yield `None`, caught
    via `catch_unwind`, never a crash.
  - `rollup` — folds child values (Sum/Min/Max/Average/Count) directly in Rust.
  - `field_eval_order` — topological order of the computed-field dependency graph via
    `directed-graph`, rejecting self-referential and transitively cyclic formulas
    (self-loops are detected explicitly, since `directed-graph` drops them).

- **The pure operations API** (`ops` module) — every mutation as a validated method on
  `ProjectState` (`create_task`, `reparent`, `link_dependency`, `assign`,
  `add_calendar_exception`, `set_field_value`, …), returning `Result<(), OpError>`.
  There is **no command enum and no dispatch** (those are Flux/React idioms this engine
  deliberately avoids — see the architecture note below). Methods are pure (mutate the
  value in place; no I/O, no clock, no globals) and are the single trust boundary that
  enforces invariants: `percent_complete` clamped to 0..=100; calendar intervals
  validated (`start < end <= 1440`) and length-capped; reparent-cycle, self-, duplicate-,
  and cycle-forming dependency links rejected (cycle checks reuse `directed-graph`).
  `OpError` carries a stable `code()` for the C ABI.

- **View projections** (`projections` module) — the "one model, many views" thesis as
  pure `&self` queries, each returning plain (serde-friendly) data:
  - `checklist()` — flattens tasks honouring decision branches (a decision reveals only
    its answered branch; nothing until answered), with a visited-guard against shared
    or cyclic references.
  - `todos()` — flat leaf-task list sorted by deadline then name.
  - `kanban(workflow)` — cards grouped into status columns (category-ordered), with a
    leading "No status" column.
  - `gantt(project_start)` — timeline bars from the CPM schedule, with critical flags.
  - `flowchart()` — tasks as nodes; dependencies and generic links as labelled edges.
  - `schedule(project_start)` — convenience wrapper over the scheduler.

### Notes

- **Architecture correction:** the earlier "TaskCommand reducer" (a Flux/React command
  bus) was dropped in favour of this native pure-operations API. The engine is *pure
  computation only*; each backend will manage state in its own native conventions (no
  universal `dispatch`/`getProps`/`handleEvent` facade). See `task-app-architecture.md`.
- Design principle: everything a scheduler derives is computed, never stored — the
  input model, the working-time engine, the CPM scheduler, computed fields, and the
  operations API are in.
- Reuses `directed-graph` (dependency/recalc graph), `datetime-core` (date arithmetic),
  and `symbolic-ir`/`symbolic-vm` (named-variable formula evaluation) rather than
  reimplementing them.
