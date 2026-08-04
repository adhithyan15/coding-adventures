# task-app — Data Model

> Part of the [task-app spec series](task-app-overview.md). This is the foundational document:
> the entity model that every other spec builds on. It is deliberately comprehensive — the model is
> designed for the *hardest* case (Microsoft Project-class scheduling) so that every simpler tool
> (checklist, todo, kanban, flowchart) is a restriction of it, never a migration away from it.

## Design goals

1. **One `Task` entity, rich enough to be a superset of every task tool.** A checklist item, a
   todo, a kanban card, a Gantt bar, and a flowchart node are all the same entity with different
   capabilities switched on.
2. **Comprehensive from day one.** Resources, assignments, calendars, baselines, and typed custom
   fields are in the model now, so there is no schema migration when the UI grows into them.
3. **Stored vs. computed is explicit.** Anything a scheduler derives (early/late dates, slack,
   critical flag, rollups, formula values) is **never** stored as source of truth — it is recomputed
   from stored inputs. This mirrors `computeStats` in the legacy checklist-app and `CardProgress`
   separation in `engram-core`.
4. **Platform-neutral and serde-friendly.** All types are plain Rust with `#[serde(rename_all =
   "camelCase")]` (engram-core house style), `serde` behind a feature flag, no I/O, no clock.
5. **Flexible via a typed field system.** Beyond the fixed scheduling fields, users define their own
   columns (text/number/date/select/relation/**formula**/**rollup**) — the Notion/Airtable capability
   — evaluated by `symbolic-vm` (see [`task-app-formula-fields.md`](task-app-formula-fields.md)).

## Identifiers

Every entity carries a stable `Id` = a **UUID v7** (time-sortable; from the `uuid` crate). Typed
newtypes prevent cross-entity mixups:

```rust
pub struct TaskId(pub Uuid);
pub struct ResourceId(pub Uuid);
pub struct CalendarId(pub Uuid);
pub struct FieldId(pub Uuid);
pub struct BaselineId(pub Uuid);
pub struct ViewId(pub Uuid);
pub struct LinkId(pub Uuid);
```

References between entities are by id, never by nesting — the state is a flat, normalized set of
maps, which keeps the dependency graph, undo snapshots, and incremental recalc simple.

## Root state

```rust
/// The entire project. One `ProjectState` per open document.
#[serde(rename_all = "camelCase")]
pub struct ProjectState {
    pub id: ProjectId,
    pub name: String,
    pub tasks: IndexMap<TaskId, Task>,          // insertion-ordered → stable outline order
    pub dependencies: Vec<DependencyLink>,       // the scheduling network
    pub links: Vec<GenericLink>,                 // non-scheduling relations
    pub resources: IndexMap<ResourceId, Resource>,
    pub assignments: Vec<Assignment>,
    pub calendars: IndexMap<CalendarId, Calendar>,
    pub project_calendar: CalendarId,            // default working time
    pub fields: IndexMap<FieldId, FieldDef>,     // user-defined custom fields
    pub workflows: IndexMap<WorkflowId, Workflow>,
    pub baselines: IndexMap<BaselineId, Baseline>,
    pub views: IndexMap<ViewId, View>,
    pub settings: ProjectSettings,               // duration unit, week start, currency, etc.
}
```

`IndexMap` (insertion-ordered map) gives O(1) lookup by id *and* a deterministic order for the WBS
outline and serialization. Task hierarchy is expressed by `parent` + order, not by nesting, so a
task can be reparented without moving data.

## The Task entity

The core of the model. A Task is intentionally large; most fields are optional and default to
"unset," so a checklist item populates ~three of them while a Gantt task populates all of them.

```rust
#[serde(rename_all = "camelCase")]
pub struct Task {
    // ---- identity & hierarchy (always present) ----
    pub id: TaskId,
    pub name: String,
    pub notes: String,
    pub parent: Option<TaskId>,        // None = top level; forms the WBS tree
    pub kind: TaskKind,                // Leaf | Summary | Milestone
    pub collapsed: bool,               // UI: is the summary subtree collapsed

    // ---- workflow (kanban / agile) ----
    pub status: StatusId,              // references a Workflow's status; default = first status
    pub completed: bool,               // the checklist "done" flag; kept distinct from status
    pub percent_complete: u8,          // 0..=100

    // ---- scheduling block (Gantt / CPM) — optional capability ----
    pub schedule: Option<TaskSchedule>,

    // ---- flexible typed fields ----
    pub fields: HashMap<FieldId, FieldValue>,   // only user-set values; formula/rollup recomputed

    // ---- checklist decision-tree support (legacy-parity projection) ----
    pub decision: Option<Decision>,    // Some => this task is a yes/no branch point
}

pub enum TaskKind { Leaf, Summary, Milestone }
```

### The scheduling block

Everything the CPM engine needs. Present only on tasks that participate in scheduling.

```rust
#[serde(rename_all = "camelCase")]
pub struct TaskSchedule {
    pub task_type: TaskType,           // FixedUnits | FixedDuration | FixedWork
    pub effort_driven: bool,
    pub duration: Duration,            // working-time span (see Duration below)
    pub work: Work,                    // total effort (person-time)
    pub calendar: Option<CalendarId>,  // task calendar override; else project/resource calendar
    pub constraint: Constraint,        // one of the 8 constraint types + optional date
    pub deadline: Option<Date>,        // soft marker; flags slippage, never forces dates

    // actuals (progress tracking)
    pub actual_start: Option<Date>,
    pub actual_finish: Option<Date>,
}

/// The scheduling triangle:  Work = Duration × Units.
/// `task_type` pins one leg while the scheduler recomputes the others.
pub enum TaskType { FixedUnits, FixedDuration, FixedWork }
```

**Task types** (the recompute rule when an input changes):

| Type | Pinned | Add resources ⇒ |
|---|---|---|
| `FixedUnits` | assignment units | duration shrinks (work constant) |
| `FixedDuration` | duration | units drop to absorb the work |
| `FixedWork` | work (always effort-driven) | duration shrinks |

### Constraints (the 8 MS-Project types, three rigidity tiers)

```rust
pub enum Constraint {
    Asap,                          // flexible: as soon as possible (default)
    Alap,                          // flexible: as late as possible
    StartNoEarlierThan(Date),      // semi-flexible
    StartNoLaterThan(Date),
    FinishNoEarlierThan(Date),
    FinishNoLaterThan(Date),
    MustStartOn(Date),             // inflexible
    MustFinishOn(Date),
}
```

Rigidity ordering matters to the scheduler: inflexible > semi-flexible > flexible, and an
inflexible constraint can override a dependency. See
[`task-app-scheduling-engine.md`](task-app-scheduling-engine.md).

### Durations, work, and time units

Scheduling operates in **working time**, so a duration is not wall-clock. We store durations and
work as integer **minutes of working time** to stay exact and serde-simple; the UI renders them in
the project's chosen unit (`ProjectSettings.duration_unit`: minutes/hours/days/weeks) using the
calendar's hours-per-day/days-per-week.

```rust
pub struct Duration { pub working_minutes: i64, pub elapsed: bool } // elapsed=true ⇒ ignore calendar
pub struct Work { pub minutes: i64 }
/// A calendar date as days since the Unix epoch (matches datetime-core::Date; UTC, civil).
pub struct Date(pub i32);
/// A precise instant when needed (actuals with time-of-day): datetime-core wall-clock seconds.
pub struct DateTime(pub f64);
```

Dates reuse `datetime-core`'s civil `Date` representation exactly, so calendar arithmetic (weekday,
add-days, month math) is a direct call into that crate — no new date code.

### Computed schedule outputs (never stored as source of truth)

The scheduler produces a `ScheduledDates` per task on demand; it is cached in a separate
side-table (`schedule_cache: HashMap<TaskId, ScheduledDates>`), rebuilt by the engine, and excluded
from the canonical snapshot's "input" section (it may be serialized as a convenience but is always
reproducible).

```rust
#[serde(rename_all = "camelCase")]
pub struct ScheduledDates {
    pub early_start: Date, pub early_finish: Date,
    pub late_start: Date,  pub late_finish: Date,
    pub scheduled_start: Date, pub scheduled_finish: Date, // after constraints/calendars
    pub total_slack: i64, pub free_slack: i64,             // working minutes
    pub critical: bool,                                     // total_slack <= 0
}
```

### Decision-tree support (legacy checklist parity)

To preserve the checklist-app's branching semantics as a first-class projection, a task may be a
decision point. This reuses the existing model shape (`yesBranch`/`noBranch` become child-task
subsets gated by the answer), so `flattenVisibleItems` becomes a view over tasks:

```rust
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub question: String,
    pub answer: Option<bool>,          // None = unanswered → only the question shows
    pub yes_children: Vec<TaskId>,     // subtree revealed when answer = Some(true)
    pub no_children: Vec<TaskId>,      // subtree revealed when answer = Some(false)
}
```

## Relations: two graphs, deliberately separate

**Dependency links** drive scheduling *and* render as flowchart edges. **Generic links** capture
non-scheduling relationships (Jira-style) and never affect dates.

```rust
#[serde(rename_all = "camelCase")]
pub struct DependencyLink {
    pub id: LinkId,
    pub predecessor: TaskId,
    pub successor: TaskId,
    pub kind: DependencyKind,          // FS | SS | FF | SF
    pub lag: Duration,                 // may be negative (lead); may be elapsed or working-time
}
pub enum DependencyKind { FinishToStart, StartToStart, FinishToFinish, StartToFinish }

#[serde(rename_all = "camelCase")]
pub struct GenericLink {
    pub id: LinkId,
    pub from: TaskId,
    pub to: TaskId,
    pub kind: LinkKind,                // Blocks | Relates | Duplicates | Causes | Custom(String)
}
```

The dependency set is loaded into `directed-graph` for topological ordering and cycle detection;
a cycle is a validation error surfaced to the UI, never an infinite loop (the crate's
`has_cycle`/`topological_sort` give this for free).

## Resources, assignments, calendars

```rust
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub id: ResourceId,
    pub name: String,
    pub kind: ResourceKind,            // Work | Material | Cost
    pub calendar: Option<CalendarId>,  // resource calendar (availability)
    pub max_units: f64,                // e.g. 1.0 = one full-time person; 3.0 = a crew of three
    pub std_rate: Money,               // cost per working hour (Work) or per unit (Material)
    pub cost_per_use: Money,
}
pub enum ResourceKind { Work, Material, Cost }

#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub task: TaskId,
    pub resource: ResourceId,
    pub units: f64,                    // % of the resource applied (1.0 = 100%)
    pub work: Work,                    // this resource's share of the task work
    pub contour: WorkContour,          // Flat | FrontLoaded | BackLoaded | BellShaped | ...
}

pub struct Money { pub cents: i64, pub currency: [u8; 3] }  // exact; numeric-tower if fractional rates needed
```

### Calendars (the working-time model)

Four-layer resolution: **resource → task → project → base**. The first layer that specifies working
time wins for a given instant. A calendar is a weekly template plus dated exceptions (holidays,
overtime days).

```rust
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: CalendarId,
    pub name: String,
    pub base: Option<CalendarId>,      // inherit from a base calendar
    pub work_week: [DaySchedule; 7],   // Mon..Sun working intervals
    pub exceptions: Vec<CalendarException>, // holidays / special days override the week
}
pub struct DaySchedule { pub working: bool, pub intervals: Vec<MinuteInterval> } // e.g. 09:00–12:00, 13:00–17:00
pub struct MinuteInterval { pub start_min: u16, pub end_min: u16 }               // minutes from midnight
pub struct CalendarException { pub date: Date, pub schedule: DaySchedule }
```

Calendar arithmetic (is-this-a-working-minute, add-N-working-minutes-to-a-date,
count-working-minutes-between) is **new code folded into `task-core`**, built on `datetime-core`
primitives (`iso_weekday`, `add_days`). There is no working-day crate in the repo, so this small
module is justified new code — not a standalone micro-crate.

## The typed field system (flexibility layer)

Users add columns beyond the built-ins. This is what lets task-app model "any kind of task
management" — the Notion/Airtable capability — on top of the MS-Project skeleton.

```rust
#[serde(rename_all = "camelCase")]
pub struct FieldDef {
    pub id: FieldId,
    pub name: String,
    pub kind: FieldKind,
}
pub enum FieldKind {
    Text, Number, Bool, Date, Duration, Money,
    Select { options: Vec<String>, multi: bool },
    Relation { target: RelationTarget },               // link to other tasks/resources
    Formula { source: String },                        // e.g. "[work] / [duration]"
    Rollup { over: RollupScope, field: FieldId, agg: RollupAgg }, // aggregate children/assignments
}
pub enum RollupScope { Children, Descendants, Assignments }
pub enum RollupAgg { Sum, Min, Max, Average, Count }

/// Per-task stored values (only for user-set, non-computed fields).
pub enum FieldValue { Text(String), Number(f64), Bool(bool), Date(Date),
                      Duration(Duration), Money(Money), Select(Vec<String>), Ref(Vec<Uuid>) }
```

`Formula` and `Rollup` fields are **computed**, never stored: their source is parsed to a
`symbolic-ir` tree, the referenced fields are extracted as the dependency set, and values are
recomputed in topological order via `directed-graph.affected_nodes` when an input changes. Full
mechanics in [`task-app-formula-fields.md`](task-app-formula-fields.md).

## Workflow (status state machine)

Kanban/agile boards need configurable statuses and legal transitions.

```rust
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub id: WorkflowId,
    pub name: String,
    pub statuses: IndexMap<StatusId, Status>,          // ordered → board column order
    pub transitions: Vec<Transition>,                  // allowed status moves; empty ⇒ any→any
    pub done_status: StatusId,                          // entering it sets completed = true
}
pub struct Status { pub id: StatusId, pub name: String, pub category: StatusCategory, pub color: String }
pub enum StatusCategory { Todo, InProgress, Done }
pub struct Transition { pub from: StatusId, pub to: StatusId }
```

## Baselines (variance)

A baseline is a named immutable snapshot of the schedule inputs + computed dates, captured at a
point in time, used to show variance (planned vs. actual). Stored compactly as a diff-free copy of
the relevant task fields.

```rust
#[serde(rename_all = "camelCase")]
pub struct Baseline {
    pub id: BaselineId,
    pub name: String,
    pub captured_at: u64,                       // injected `now`
    pub tasks: HashMap<TaskId, BaselineTask>,   // start/finish/duration/work/cost at capture
}
```

## Views (projections)

A View is a saved projection descriptor: which tasks (filter), how grouped/sorted, and the render
shape. Views hold **no task data** — they are lenses over `tasks`.

```rust
#[serde(rename_all = "camelCase")]
pub struct View {
    pub id: ViewId,
    pub name: String,
    pub shape: ViewShape,                       // Checklist | Todo | Kanban | Gantt | Calendar | Flowchart | Table
    pub filter: Filter,                         // status/field/date predicates
    pub group_by: Option<FieldRef>,             // e.g. kanban groups by status
    pub sort: Vec<SortKey>,
    pub visible_fields: Vec<FieldRef>,          // table/gantt columns
}
```

### How each projection restricts the model

| Shape | Reads | Ignores |
|---|---|---|
| **Checklist** | `name`, `completed`, `decision`, hierarchy; `flattenVisibleItems` over decision answers | scheduling, resources |
| **Todo** | `name`, `completed`, `schedule.deadline`, `percent_complete` | dependencies, resources |
| **Kanban** | `status` (group), `name`, `percent_complete` | dates, dependencies |
| **Gantt** | full `schedule`, `dependencies`, calendars → `ScheduledDates` | generic links |
| **Calendar** | `ScheduledDates.scheduled_start/finish`, `deadline` | — |
| **Flowchart** | `dependencies` + `links` as edges, tasks as nodes | dates |
| **Table** | any `fields` + built-ins as columns | render-specific bits |

## Commands (mutation surface, summarized)

All mutations flow through a single `enum TaskCommand` reduced by a pure
`reduce(&ProjectState, TaskCommand) -> ProjectState` (engram-core pattern; new state out, never
mutate in place — cheap given `IndexMap`/`im`-style sharing where needed). The command catalog spans
task CRUD, reparent/reorder, set-status/complete, edit-schedule (duration/work/type/constraint),
link/unlink dependencies and generic links, resource + assignment CRUD, calendar edits, field-def
CRUD + set-value, answer-decision, capture/clear baseline, and view CRUD. The exhaustive command
list and the reducer's recompute triggers (which commands invalidate the schedule cache vs. only the
formula cache) live in [`task-app-architecture.md`](task-app-architecture.md) alongside the facade.

## Invariants & validation

- Hierarchy is acyclic; a task's `parent` chain never includes itself. Reparent commands reject cycles.
- The dependency network is acyclic (checked via `directed-graph.has_cycle`); a cycle is a surfaced
  validation error, not a panic.
- `percent_complete ∈ 0..=100`; `units ≥ 0`; `max_units ≥ 0`.
- Every `status` references an existing `Status` in the task's board `Workflow`; deleting a status
  requires remapping tasks off it.
- Summary tasks derive dates/work/%-complete/cost from descendants (rollup) and are not directly
  scheduled; setting a schedule input on a Summary is rejected.
- Formula/rollup field graphs are acyclic; a formula referencing its own field (directly or
  transitively) is a validation error.

## Snapshot & persistence

`ProjectState` serializes to JSON via `serde_json` (feature-gated), matching the engram-core /
spreadsheet-core idiom. The snapshot is the single source of truth; computed caches
(`schedule_cache`, formula values) are reproducible and may be omitted. Durable storage (optional)
uses `storage-core` with one JSON record per project, following the `memory-store` shape. See the
architecture spec for the facade's `snapshot()` / `load_snapshot()` and `hostIntent`-driven
file I/O.
