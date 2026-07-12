# Changelog

All notable changes to `task-core` are documented here.

## [0.1.0] - Unreleased

### Added

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

- **The command/reducer surface** (`reducer` module) — the single entry point for
  every mutation:
  - `enum TaskCommand` (~35 variants: task CRUD, reparent/reorder, status/completion/
    percent, schedule edits, dependency + generic link CRUD, resource + assignment
    CRUD, calendar edits, field-def CRUD + set-value, decisions, baselines, views,
    load-state) and a pure, total `reduce(&ProjectState, TaskCommand) -> ProjectState`
    (immutable in, new state out — the `engram-core` house pattern).
  - **The reducer is the trust boundary** that enforces the validation the scheduler's
    security review deferred: `percent_complete` clamped to 0..=100; calendar
    intervals validated (`start < end <= 1440`) and length-capped; reparent cycles,
    self-dependencies, duplicate and cycle-forming dependency links rejected (cycle
    checks reuse `directed-graph`). Invalid commands are no-ops, never errors.
  - `TaskCommand` derives serde (behind the feature) as an internally-tagged enum, so
    the facade can dispatch host JSON directly.

### Notes

- Design principle: everything a scheduler derives is computed, never stored — the
  input model, the working-time engine, the CPM scheduler, and the reducer are in;
  formula/rollup fields land next.
- Reuses `directed-graph` (dependency graph) and `datetime-core` (date arithmetic)
  rather than reimplementing them.
