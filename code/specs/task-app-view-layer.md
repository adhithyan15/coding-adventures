# task-app engine: the view/query layer (Phase 3)

**Status:** design (spec-first; committed before implementation)
**Phase:** 3 of [task-app-super-app](task-app-super-app.md)
**Touches:** `task-core` (projections, model, formatting), `task-wasm` (view exports)

This phase makes the engine **fat** in the way the master spec demands: *filtering,
sorting, grouping, column selection, and display formatting all live in `task-core`*, and
a projection **takes a view config and returns render-ready data**. Components become thin
renderers of that output — the sheet, board, and calendar UIs (Phases 5–7) will each draw
one of these projections and emit raw intent, holding no business logic and no formatting.

The view **config types already exist** (`View`, `Filter`, `SortKey`, `FieldRef`,
`ViewShape` in `model.rs`). What's missing is the **evaluation layer** that consumes them
and the **`table()`/`calendar()` projections**. This phase adds those, retrofits the
existing projections to be view-driven, and moves formatting out of the web host.

---

## 1. The problem today
- `todos()`, `kanban(workflow)`, `gantt(start)`, `checklist()`, `flowchart()` take **fixed
  arguments**, not a `View`. There is no filter/sort/group evaluation anywhere.
- `ViewShape::Table` and `ViewShape::Calendar` exist as enum variants but **have no
  projection** — you cannot render a sheet or a calendar from the engine.
- **Formatting lives in the web host** (`host/web/src/main.tsx` builds each row string:
  the `✓`/`○`, the `· due …`, the `→` schedule, the `⚠ overdue`). That violates
  fat-engine/dumb-UI and means every future host would reinvent it.

## 2. The evaluation layer (the core of this phase)

A single pipeline, applied by every view-driven projection:

```
tasks ──filter──▶ ordered ──sort──▶ grouped ──▶ shape-specific render (render-ready)
```

### 2.1 The field accessor — one resolver, used everywhere
The linchpin. A pure function that resolves a `FieldRef` on a task to a comparable,
formattable **`CellValue`**:

```rust
enum CellValue { Text(String), Number(f64), Date(Option<Date>), Bool(bool), Empty }

fn cell(project: &ProjectState, task: &Task, field: &FieldRef,
        schedule: &ScheduleResult) -> CellValue
```

- **Built-ins** (`FieldRef::Builtin("name"|"status"|"completed"|"percentComplete"|
  "start"|"finish"|"deadline"|"duration"|"slack"|"critical"|…)`) read the task or its
  computed `ScheduledDates`.
- **Custom** (`FieldRef::Custom(id)`) reads the stored value, or evaluates a formula/rollup
  field via the existing `formula` module.

Filter, sort, group, and display **all** go through `cell()`, so they agree by
construction — a huge simplification over reimplementing access three times.

### 2.2 Filter → sort → group
- **Filter**: extend the existing `Filter` (statuses / completed / search) with a small,
  composable **field-predicate** set — `{ field, op, value }` where `op ∈ {eq, ne, lt,
  lte, gt, gte, contains, isEmpty, isNotEmpty}` — evaluated against `cell()`. Nested
  AND/OR ≤ 3 deep (matches the app-landscape survey); represented as a small boolean tree.
  (The current three fields become sugar over the predicate set, kept for compatibility.)
- **Sort**: multi-key `Vec<SortKey>` (already in the model), comparing `cell()` values with
  a stable total order (Empty sorts last); ascending per key.
- **Group**: `group_by: Option<FieldRef>` (already in the model) → an ordered list of
  groups, each `{ key: CellValue, key_label: String, task_ids }`, group order stable
  (by key). A "no value" group is last. This is what swimlanes / status columns / grouped
  table sections all render from.

All three are **pure `&self`** and bounded (no recursion over untrusted data).

### 2.3 Display formatting — in the engine
`cell()` has a sibling `format_cell(value, field, settings) -> String` that produces the
**render-ready** string per project conventions: dates as `YYYY-MM-DD`, durations in the
project's `DurationUnit`, `Bool` as `✓`/`○` **only where a checklist wants it** (the shape
decides; the engine offers both the typed value and a formatted string so a host can pick).
The web host's row-building logic (§1) moves here.

## 3. New & upgraded projections

Every projection gains a `View`-driven form. Signatures return **render-ready** structs
(all serde-friendly, camelCase over the wire):

- **`table(view: &View, start: Date) -> TableView`**
  `TableView { columns: [ColumnHeader{ field, label, kind }], groups: [RowGroup{ key_label,
  rows: [Row{ task, cells: [Cell{ value, display }] }] }] }`. Honors filter/sort/group and
  `visible_fields`. This is the sheet.
- **`calendar(view: &View, range: DateRange, start: Date) -> CalendarView`**
  `CalendarView { events: [Event{ task, start, finish, all_day, label, overdue }] }` for
  tasks whose scheduled span intersects `range`. This is the calendar.
- **`todos(view: &View)`**, **`kanban(view: &View, workflow)`**, **`gantt(view: &View,
  start)`** — retrofitted to apply the same filter/sort/group first. The current
  no-argument forms remain as thin wrappers passing a default (empty) view, so existing
  callers and the shipped web app keep working unchanged.

A `View`'s `shape` selects which projection a generic **`render(view, …)`** dispatches to,
so a host can switch views by passing a different `View` and calling one entry point.

## 4. Model additions (small, additive)
Only what the views need to be useful; each optional with a default so nothing breaks:
- **`labels: Vec<LabelId>` on `Task`** + a `labels: BTreeMap<LabelId, Label{name,color}>`
  on `ProjectState` (Trello/Notion-grade tagging; a first-class filter/group dimension).
- **`priority: Option<Priority>` on `Task`** (`Low|Normal|High|Urgent`) — a common sort/
  group/filter dimension in every task app.
- The richer `Filter` predicate tree (§2.2).

Recurring tasks, reminders, attachments, and comments are **out of scope** here (later
phases); this phase is the *query/render* layer, not new task semantics beyond
labels/priority.

## 5. `task-wasm` surface
- `table` / `calendar` exports (taking a `View` + start/range JSON), plus view-driven
  `todos`/`kanban`/`gantt` variants that accept a `View`.
- Label/priority ops (`upsert_label`, `delete_label`, `set_task_labels`, `set_priority`).
- These operate on the **active project** (consistent with Phase 2's ABI).

## 6. Moving the web host onto the engine's output (proves the layer)
After `table()` + formatting land, rewrite `host/web/src/main.tsx` so the row strings come
from the engine (`table()` / a formatted `todos(view)`), not from JS. The host shrinks to:
query the engine with a `View` → render cells → emit intent. This is the dumb-UI end state
that the Mosaic sheet/board/calendar components (Phases 5–7) generalize.

## 7. Tests (`cargo test -p task-core`)
- `cell()` for every built-in + a custom formula field.
- Filter: each predicate op; the nested AND/OR tree; the legacy sugar fields.
- Sort: multi-key, Empty-last, stability.
- Group: ordered groups, no-value group last, by status/label/priority.
- `table()`: columns follow `visible_fields`; rows follow filter+sort; grouped sections.
- `calendar()`: only tasks intersecting the range; overdue flag; all-day vs. timed.
- Formatting: dates, durations per `DurationUnit`, bool glyphs — golden strings.
- Backward-compat: no-arg `todos()`/`kanban()`/`gantt()` reproduce today's output.

## 8. Sequencing within Phase 3 (small PRs)
1. **Field accessor + `CellValue`** + `format_cell`, with the built-in set and custom/
   formula resolution. (Pure, heavily tested; nothing consumes it yet.)
2. **Filter/sort/group evaluation** (predicate tree, multi-key sort, grouping) + `todos`/
   `kanban`/`gantt` retrofitted to view-driven (default-view wrappers keep compat).
3. **`table()` projection** + the sheet's render-ready output.
4. **`calendar()` projection** + `DateRange`.
5. **Labels + priority** model/ops + `task-wasm` label/priority ops.
6. **`task-wasm` view exports** + **move the web host onto engine formatting** (§6).

Each PR: tests → clippy (both feature configs) → CHANGELOG → `/security-review` → PR →
`/babysit-pr`. Pull `origin/main` first.

## 9. Open decisions deferred to their PR
- Whether the predicate tree also powers **saved views** persistence UI (Phase 9) — the
  data is here; the UI is later.
- Cross-**workspace** table/calendar (all projects at once) vs. per-project — start
  per-project (active project), add a workspace-scoped variant when the portfolio view
  needs it.
- Exact built-in field name catalog — fix it in PR-1 and treat it as the wire contract.
