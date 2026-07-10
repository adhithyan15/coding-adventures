//! The entities.
//!
//! One [`Task`] is a strict superset of every task tool: a checklist item is a task
//! with a done-flag and no scheduling; a todo is a task with a deadline; a kanban
//! card is a task with a workflow status; a Gantt bar is a fully-scheduled task; a
//! flowchart node is a task in the relation graph. The remaining types
//! ([`Resource`], [`Calendar`], [`FieldDef`], [`Workflow`], [`Baseline`], [`View`])
//! surround the task, and [`ProjectState`] is the flat, normalised root that holds
//! them all.

use crate::ids::*;
use crate::primitives::*;
use std::collections::BTreeMap;

// ─────────────────────────────────────────────────────────────────────────────
// Task
// ─────────────────────────────────────────────────────────────────────────────

/// The central entity. Most fields are optional and default to "unset", so a
/// checklist item populates a handful while a scheduled Gantt task populates all.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Task {
    /// Stable identity (minted by the facade).
    pub id: TaskId,
    /// Human-facing title.
    pub name: String,
    /// Free-form notes / description.
    pub notes: String,
    /// Parent in the work-breakdown structure; `None` for a top-level task.
    pub parent: Option<TaskId>,
    /// Sibling ordering key (lower sorts first). Reordering reassigns this, never
    /// moves data.
    pub order: i64,
    /// Leaf, summary (rolls up its children), or milestone (zero-duration marker).
    pub kind: TaskKind,
    /// UI: is this summary's subtree collapsed.
    pub collapsed: bool,

    /// Workflow status (kanban/agile). `None` uses the board's default status.
    pub status: Option<StatusId>,
    /// The checklist "done" flag — kept distinct from `status` so a board and a
    /// checklist can coexist over the same task.
    pub completed: bool,
    /// Progress, 0..=100.
    pub percent_complete: u8,

    /// The scheduling block — present only on tasks that participate in CPM.
    pub schedule: Option<TaskSchedule>,

    /// User-set values for custom fields. Formula/rollup fields are *not* stored
    /// here; they are recomputed.
    pub fields: BTreeMap<FieldId, FieldValue>,

    /// Decision-tree branch point (checklist parity). `Some` makes this task a
    /// yes/no question whose answer reveals one child subtree.
    pub decision: Option<Decision>,
}

impl Task {
    /// A bare leaf task: no scheduling, no status, not done.
    pub fn new(id: TaskId, name: impl Into<String>) -> Task {
        Task {
            id,
            name: name.into(),
            notes: String::new(),
            parent: None,
            order: 0,
            kind: TaskKind::Leaf,
            collapsed: false,
            status: None,
            completed: false,
            percent_complete: 0,
            schedule: None,
            fields: BTreeMap::new(),
            decision: None,
        }
    }
}

/// What kind of node a task is in the outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum TaskKind {
    /// An ordinary task with its own duration/work.
    Leaf,
    /// A container whose dates/work/cost roll up from its descendants.
    Summary,
    /// A zero-duration marker (a deadline or a phase gate).
    Milestone,
}

/// Everything the CPM scheduler needs. Present only on scheduled tasks.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TaskSchedule {
    /// Which leg of `Work = Duration × Units` is pinned when inputs change.
    pub task_type: TaskType,
    /// When true, changing assignments changes duration but never total work.
    pub effort_driven: bool,
    /// Working-time span of the task.
    pub duration: Duration,
    /// Total effort.
    pub work: Work,
    /// Task calendar override; falls back to resource/project/base calendars.
    pub calendar: Option<CalendarId>,
    /// The date constraint (default `Asap`).
    pub constraint: Constraint,
    /// A soft deadline: flags slippage, never forces a date.
    pub deadline: Option<Date>,
    /// Actual start once progress is recorded.
    pub actual_start: Option<Date>,
    /// Actual finish once complete.
    pub actual_finish: Option<Date>,
}

impl Default for TaskSchedule {
    fn default() -> Self {
        TaskSchedule {
            task_type: TaskType::FixedUnits,
            effort_driven: true,
            duration: Duration::minutes(0),
            work: Work::zero(),
            calendar: None,
            constraint: Constraint::Asap,
            deadline: None,
            actual_start: None,
            actual_finish: None,
        }
    }
}

/// Which quantity a task holds fixed while the scheduler recomputes the others.
///
/// | Type            | Pinned            | Add resources ⇒               |
/// |-----------------|-------------------|-------------------------------|
/// | `FixedUnits`    | assignment units  | duration shrinks (work const) |
/// | `FixedDuration` | duration          | units drop to absorb the work |
/// | `FixedWork`     | work (effort-driven) | duration shrinks           |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum TaskType {
    /// Units fixed.
    FixedUnits,
    /// Duration fixed.
    FixedDuration,
    /// Work fixed (always effort-driven).
    FixedWork,
}

/// A date constraint. Rigidity increases down the list; an inflexible constraint can
/// override a dependency (the scheduler records a conflict when they disagree).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum Constraint {
    /// As soon as possible (flexible, default).
    Asap,
    /// As late as possible (flexible).
    Alap,
    /// Start no earlier than this date (semi-flexible).
    StartNoEarlierThan(Date),
    /// Start no later than this date (semi-flexible).
    StartNoLaterThan(Date),
    /// Finish no earlier than this date (semi-flexible).
    FinishNoEarlierThan(Date),
    /// Finish no later than this date (semi-flexible).
    FinishNoLaterThan(Date),
    /// Must start exactly on this date (inflexible).
    MustStartOn(Date),
    /// Must finish exactly on this date (inflexible).
    MustFinishOn(Date),
}

/// Scheduler output for one task — **computed, never stored as truth**. Cached
/// alongside the state and rebuilt by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ScheduledDates {
    /// Earliest the task can start (forward pass).
    pub early_start: Date,
    /// Earliest the task can finish.
    pub early_finish: Date,
    /// Latest the task can start without delaying the project (backward pass).
    pub late_start: Date,
    /// Latest the task can finish.
    pub late_finish: Date,
    /// Start after constraints and calendars are applied.
    pub scheduled_start: Date,
    /// Finish after constraints and calendars are applied.
    pub scheduled_finish: Date,
    /// Total slack in working minutes (`late_start − early_start`).
    pub total_slack: i64,
    /// Free slack in working minutes.
    pub free_slack: i64,
    /// On the critical path (`total_slack <= 0`).
    pub critical: bool,
}

/// A yes/no branch point, preserving the checklist app's decision-tree semantics.
/// Until answered, only the question shows; the answer reveals one child subtree.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Decision {
    /// The yes/no question shown for this branch point.
    pub question: String,
    /// The answer; `None` while unanswered.
    pub answer: Option<bool>,
    /// Child tasks revealed when the answer is `true`.
    pub yes_children: Vec<TaskId>,
    /// Child tasks revealed when the answer is `false`.
    pub no_children: Vec<TaskId>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Relations: two graphs, deliberately separate
// ─────────────────────────────────────────────────────────────────────────────

/// A scheduling dependency. Drives the CPM network *and* renders as a flowchart
/// edge.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct DependencyLink {
    /// Link identity.
    pub id: LinkId,
    /// The task that must (partly) precede.
    pub predecessor: TaskId,
    /// The task that (partly) follows.
    pub successor: TaskId,
    /// Which endpoints are related.
    pub kind: DependencyKind,
    /// Delay (positive) or lead (negative) between the linked endpoints.
    pub lag: Duration,
}

/// The four precedence relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum DependencyKind {
    /// Successor starts after predecessor finishes (the default).
    FinishToStart,
    /// Successor starts after predecessor starts.
    StartToStart,
    /// Successor finishes after predecessor finishes.
    FinishToFinish,
    /// Successor finishes after predecessor starts.
    StartToFinish,
}

/// A non-scheduling relationship (Jira-style). Never affects dates; renders in the
/// flowchart alongside dependencies.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GenericLink {
    /// Link identity.
    pub id: LinkId,
    /// Source task.
    pub from: TaskId,
    /// Target task.
    pub to: TaskId,
    /// The relationship type.
    pub kind: LinkKind,
}

/// The type of a non-scheduling link.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum LinkKind {
    /// This task blocks the other.
    Blocks,
    /// Loosely related.
    Relates,
    /// A duplicate of the other.
    Duplicates,
    /// Causes the other.
    Causes,
    /// A user-named relationship.
    Custom(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// Resources & assignments
// ─────────────────────────────────────────────────────────────────────────────

/// Something that can be assigned to a task: a person, a machine, a material, or a
/// flat cost.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Resource {
    /// Resource identity.
    pub id: ResourceId,
    /// Display name.
    pub name: String,
    /// Work, material, or cost.
    pub kind: ResourceKind,
    /// Availability calendar; falls back to the project calendar.
    pub calendar: Option<CalendarId>,
    /// Capacity: `1.0` = one full-time unit, `3.0` = a crew of three.
    pub max_units: f64,
    /// Standard rate (per working hour for `Work`, per unit for `Material`).
    pub std_rate: Money,
    /// A fixed cost incurred each time the resource is used.
    pub cost_per_use: Money,
}

/// The nature of a resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum ResourceKind {
    /// People or equipment measured in time (drives duration).
    Work,
    /// Consumables measured in quantity.
    Material,
    /// A pure cost line with no time dimension.
    Cost,
}

/// A resource applied to a task.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Assignment {
    /// The task being worked.
    pub task: TaskId,
    /// The resource doing the work.
    pub resource: ResourceId,
    /// Fraction of the resource applied (`1.0` = 100%).
    pub units: f64,
    /// This resource's share of the task's work.
    pub work: Work,
    /// How the work is distributed across the task's span.
    pub contour: WorkContour,
}

/// How assigned work is spread over a task's duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum WorkContour {
    /// Even distribution (the default).
    Flat,
    /// Heavier at the start.
    FrontLoaded,
    /// Heavier at the end.
    BackLoaded,
    /// Ramps up then down.
    BellShaped,
}

// ─────────────────────────────────────────────────────────────────────────────
// Calendars (the working-time model)
// ─────────────────────────────────────────────────────────────────────────────

/// A working-time template: a weekly pattern plus dated exceptions. Resolution
/// layers resource → task → project → base; the first that specifies a time wins.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Calendar {
    /// Calendar identity.
    pub id: CalendarId,
    /// Display name.
    pub name: String,
    /// Optional base calendar to inherit unspecified days from.
    pub base: Option<CalendarId>,
    /// The weekly working pattern, indexed Monday (0) … Sunday (6).
    pub work_week: [DaySchedule; 7],
    /// Holidays and special days that override the weekly pattern.
    pub exceptions: Vec<CalendarException>,
}

impl Calendar {
    /// A standard Monday–Friday, 09:00–17:00 calendar.
    pub fn standard(id: CalendarId, name: impl Into<String>) -> Calendar {
        let workday = DaySchedule {
            working: true,
            intervals: vec![MinuteInterval {
                start_min: 9 * 60,
                end_min: 17 * 60,
            }],
        };
        let off = DaySchedule {
            working: false,
            intervals: Vec::new(),
        };
        Calendar {
            id,
            name: name.into(),
            base: None,
            // Mon, Tue, Wed, Thu, Fri, Sat, Sun
            work_week: [
                workday.clone(),
                workday.clone(),
                workday.clone(),
                workday.clone(),
                workday,
                off.clone(),
                off,
            ],
            exceptions: Vec::new(),
        }
    }
}

/// One day's working intervals.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct DaySchedule {
    /// Whether any work happens on this day.
    pub working: bool,
    /// The working windows (e.g. 09:00–12:00, 13:00–17:00).
    pub intervals: Vec<MinuteInterval>,
}

/// A half-open working window `[start_min, end_min)` in minutes from midnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct MinuteInterval {
    /// Inclusive start, minutes from midnight.
    pub start_min: u16,
    /// Exclusive end, minutes from midnight.
    pub end_min: u16,
}

/// A dated override of the weekly pattern (a holiday, or an overtime day).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CalendarException {
    /// The affected date.
    pub date: Date,
    /// The schedule that applies on that date.
    pub schedule: DaySchedule,
}

// ─────────────────────────────────────────────────────────────────────────────
// Typed custom fields (the flexibility layer)
// ─────────────────────────────────────────────────────────────────────────────

/// A user-defined column. `Formula` and `Rollup` fields are computed by the formula
/// engine (`symbolic-vm`), not stored per task.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FieldDef {
    /// Field identity.
    pub id: FieldId,
    /// Column name.
    pub name: String,
    /// The field's type and behavior.
    pub kind: FieldKind,
}

/// The type of a custom field.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum FieldKind {
    /// Free text.
    Text,
    /// A number.
    Number,
    /// A boolean.
    Bool,
    /// A date.
    Date,
    /// A working-time duration.
    Duration,
    /// A monetary amount.
    Money,
    /// A single- or multi-select from a fixed option set.
    Select {
        /// The allowed options.
        options: Vec<String>,
        /// Whether more than one option may be chosen.
        multi: bool,
    },
    /// A link to other entities.
    Relation {
        /// What the relation points at.
        target: RelationTarget,
    },
    /// A computed value, e.g. `[work] / [duration]`.
    Formula {
        /// The formula source in `[field]` bracket syntax.
        source: String,
    },
    /// An aggregate over related tasks.
    Rollup {
        /// Which set of tasks to aggregate over.
        over: RollupScope,
        /// The field being aggregated.
        field: FieldId,
        /// The aggregation function.
        agg: RollupAgg,
    },
}

/// What a `Relation` field points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum RelationTarget {
    /// Links to other tasks.
    Task,
    /// Links to resources.
    Resource,
}

/// The set a rollup aggregates over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum RollupScope {
    /// Direct children only.
    Children,
    /// All descendants.
    Descendants,
    /// Assignments on the task.
    Assignments,
}

/// A rollup aggregation function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum RollupAgg {
    /// Sum of the values.
    Sum,
    /// Smallest value.
    Min,
    /// Largest value.
    Max,
    /// Arithmetic mean.
    Average,
    /// Count of non-empty values.
    Count,
}

/// A stored value for a non-computed custom field.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum FieldValue {
    /// Text value.
    Text(String),
    /// Numeric value.
    Number(f64),
    /// Boolean value.
    Bool(bool),
    /// Date value.
    Date(Date),
    /// Duration value.
    Duration(Duration),
    /// Money value.
    Money(Money),
    /// One or more selected options.
    Select(Vec<String>),
    /// Referenced entity ids.
    Ref(Vec<String>),
}

// ─────────────────────────────────────────────────────────────────────────────
// Workflow (status state machine)
// ─────────────────────────────────────────────────────────────────────────────

/// A configurable set of statuses and legal transitions — the engine behind kanban
/// and agile boards.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Workflow {
    /// Workflow identity.
    pub id: WorkflowId,
    /// Display name.
    pub name: String,
    /// Statuses in board-column order.
    pub statuses: BTreeMap<StatusId, Status>,
    /// Allowed status moves; empty means any status may move to any other.
    pub transitions: Vec<Transition>,
    /// Entering this status marks a task `completed`.
    pub done_status: StatusId,
}

/// One status (a board column).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Status {
    /// Status identity.
    pub id: StatusId,
    /// Display name.
    pub name: String,
    /// Which lifecycle bucket it belongs to.
    pub category: StatusCategory,
    /// A display color (hex).
    pub color: String,
}

/// The lifecycle bucket a status belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum StatusCategory {
    /// Not started.
    Todo,
    /// Underway.
    InProgress,
    /// Finished.
    Done,
}

/// A legal status move.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Transition {
    /// The status moved from.
    pub from: StatusId,
    /// The status moved to.
    pub to: StatusId,
}

// ─────────────────────────────────────────────────────────────────────────────
// Baselines (variance)
// ─────────────────────────────────────────────────────────────────────────────

/// A named, immutable snapshot of the schedule at a point in time, used to show
/// planned-vs-actual variance.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Baseline {
    /// Baseline identity.
    pub id: BaselineId,
    /// Display name (e.g. "Baseline 1").
    pub name: String,
    /// When it was captured (the injected `now`, epoch millis).
    pub captured_at: u64,
    /// Per-task captured values.
    pub tasks: BTreeMap<TaskId, BaselineTask>,
}

/// The captured values for one task in a baseline.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BaselineTask {
    /// Planned start at capture.
    pub start: Option<Date>,
    /// Planned finish at capture.
    pub finish: Option<Date>,
    /// Planned duration at capture.
    pub duration: Duration,
    /// Planned work at capture.
    pub work: Work,
}

// ─────────────────────────────────────────────────────────────────────────────
// Views (projections)
// ─────────────────────────────────────────────────────────────────────────────

/// A saved projection: which tasks, how grouped/sorted, and the render shape. A view
/// holds no task data — it is a lens over `ProjectState::tasks`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct View {
    /// View identity.
    pub id: ViewId,
    /// Display name.
    pub name: String,
    /// How the tasks are rendered.
    pub shape: ViewShape,
    /// Which tasks are included.
    pub filter: Filter,
    /// Optional grouping column (e.g. kanban groups by status).
    pub group_by: Option<FieldRef>,
    /// Sort order.
    pub sort: Vec<SortKey>,
    /// Columns shown (table/gantt).
    pub visible_fields: Vec<FieldRef>,
}

/// The render shape of a view — each is a restriction of the one task model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum ViewShape {
    /// Decision-tree checklist.
    Checklist,
    /// Flat todo list.
    Todo,
    /// Status-column board.
    Kanban,
    /// Timeline with dependency bars.
    Gantt,
    /// Date grid.
    Calendar,
    /// Node-edge graph of dependencies and links.
    Flowchart,
    /// Spreadsheet-style grid.
    Table,
}

/// A predicate set selecting which tasks a view shows. Kept intentionally small for
/// now; richer field/date predicates are a follow-up.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Filter {
    /// Only tasks in these statuses (empty = any).
    pub statuses: Vec<StatusId>,
    /// Filter by completion (`None` = either).
    pub completed: Option<bool>,
    /// Case-insensitive substring match on the task name.
    pub search: Option<String>,
}

/// A reference to a column, either a built-in field or a custom one.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum FieldRef {
    /// A built-in column, named (e.g. "name", "start", "finish", "percentComplete").
    Builtin(String),
    /// A user-defined field.
    Custom(FieldId),
}

/// A sort key.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SortKey {
    /// The column to sort on.
    pub field: FieldRef,
    /// Ascending when true.
    pub ascending: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Project settings & the root state
// ─────────────────────────────────────────────────────────────────────────────

/// Project-wide conventions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ProjectSettings {
    /// The unit durations are entered and displayed in.
    pub duration_unit: DurationUnit,
    /// First day of the week (ISO: Monday = 1).
    pub week_start: u8,
    /// Default currency (ISO-4217).
    pub currency: String,
    /// Working hours in a standard day (for unit conversion).
    pub hours_per_day: u16,
    /// Working days in a standard week (for unit conversion).
    pub days_per_week: u8,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        ProjectSettings {
            duration_unit: DurationUnit::Days,
            week_start: 1,
            currency: "USD".to_string(),
            hours_per_day: 8,
            days_per_week: 5,
        }
    }
}

/// The unit durations are displayed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum DurationUnit {
    /// Minutes.
    Minutes,
    /// Hours.
    Hours,
    /// Days.
    Days,
    /// Weeks.
    Weeks,
}

/// The entire project — the flat, normalised root. Every entity is stored by id in a
/// map (deterministic iteration for stable serialization); relationships are by id,
/// never by nesting, which keeps the dependency graph, snapshots, and incremental
/// recompute simple.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ProjectState {
    /// Project identity.
    pub id: ProjectId,
    /// Project name.
    pub name: String,
    /// All tasks, by id.
    pub tasks: BTreeMap<TaskId, Task>,
    /// The scheduling network.
    pub dependencies: Vec<DependencyLink>,
    /// Non-scheduling relations.
    pub links: Vec<GenericLink>,
    /// All resources, by id.
    pub resources: BTreeMap<ResourceId, Resource>,
    /// Resource assignments.
    pub assignments: Vec<Assignment>,
    /// All calendars, by id.
    pub calendars: BTreeMap<CalendarId, Calendar>,
    /// The default working-time calendar.
    pub project_calendar: CalendarId,
    /// User-defined custom fields, by id.
    pub fields: BTreeMap<FieldId, FieldDef>,
    /// Status workflows, by id.
    pub workflows: BTreeMap<WorkflowId, Workflow>,
    /// Captured baselines, by id.
    pub baselines: BTreeMap<BaselineId, Baseline>,
    /// Saved views, by id.
    pub views: BTreeMap<ViewId, View>,
    /// Project-wide settings.
    pub settings: ProjectSettings,
}

impl ProjectState {
    /// An empty project seeded with a standard Monday–Friday calendar.
    pub fn empty(id: ProjectId) -> ProjectState {
        let cal_id = CalendarId::from_raw("calendar-standard");
        let mut calendars = BTreeMap::new();
        calendars.insert(
            cal_id.clone(),
            Calendar::standard(cal_id.clone(), "Standard"),
        );
        ProjectState {
            id,
            name: String::new(),
            tasks: BTreeMap::new(),
            dependencies: Vec::new(),
            links: Vec::new(),
            resources: BTreeMap::new(),
            assignments: Vec::new(),
            calendars,
            project_calendar: cal_id,
            fields: BTreeMap::new(),
            workflows: BTreeMap::new(),
            baselines: BTreeMap::new(),
            views: BTreeMap::new(),
            settings: ProjectSettings::default(),
        }
    }
}
