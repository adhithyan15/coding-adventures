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

### Notes

- Design principle: everything a scheduler derives is computed, never stored — this
  release is the *input* model; the CPM scheduler and reducer land next.
- Reuses `directed-graph` (declared; consumed by the forthcoming scheduler) and
  `datetime-core` (date arithmetic) rather than reimplementing them.
