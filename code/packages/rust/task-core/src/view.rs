//! The view/query layer's shared field accessor.
//!
//! Every view-driven projection — the table (sheet), the calendar, and the filtered
//! todo/kanban/gantt — reads a task's field values through **one** function, [`cell`],
//! and renders them through **one** function, [`format_cell`]. Because filtering,
//! sorting, grouping, and display all resolve values the same way, they agree by
//! construction: a column can never sort by one interpretation of a field and display
//! another. This module is that single resolver; the pipeline that consumes it (filter →
//! sort → group → shape) lands in the following PRs. See
//! `code/specs/task-app-view-layer.md`.
//!
//! ## The built-in field catalogue (the wire contract)
//!
//! A [`FieldRef::Builtin`] names a column by a stable string. The set below is the
//! contract every host and view relies on; unknown names resolve to [`CellValue::Empty`]
//! rather than erroring, so a view referencing a not-yet-supported column degrades
//! gracefully.
//!
//! | name                              | value                                   |
//! |-----------------------------------|-----------------------------------------|
//! | `name`, `notes`                   | `Text`                                  |
//! | `status`                          | `Text` (status id) or `Empty`           |
//! | `kind`                            | `Text` (`leaf`/`summary`/`milestone`)   |
//! | `completed`, `critical`           | `Bool`                                  |
//! | `percentComplete`                 | `Number` (0..=100)                      |
//! | `duration`, `totalSlack`, `freeSlack` | `Number` (working minutes)          |
//! | `deadline`                        | `Date`                                  |
//! | `start`/`scheduledStart`, `finish`/`scheduledFinish` | `Date`               |
//! | `earlyStart`/`earlyFinish`/`lateStart`/`lateFinish`  | `Date`               |
//!
//! Custom fields ([`FieldRef::Custom`]) resolve to the task's **stored** value; computed
//! (formula/rollup) resolution is layered on where the recompute already runs, in the
//! filter/sort PR.

use crate::ids::TaskId;
use crate::model::{
    DurationUnit, FieldKind, FieldRef, FieldValue, Filter, ProjectSettings, ProjectState, SortKey,
    Task, TaskKind, View,
};
use crate::primitives::Date;
use crate::scheduler::ScheduleResult;
use std::cmp::Ordering;

/// A resolved, comparable, formattable field value — the common currency of the view
/// layer. Filter predicates compare it, sorts order it, groups key on it, and
/// [`format_cell`] renders it to a display string.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", tag = "type", content = "value")
)]
pub enum CellValue {
    /// Text (names, statuses, joined selects).
    Text(String),
    /// A number (percent, working minutes, custom numbers/money).
    Number(f64),
    /// A date, or `None` for an unset/absent date.
    Date(Option<Date>),
    /// A boolean.
    Bool(bool),
    /// No value — an unset field, or a built-in that doesn't apply to this task
    /// (e.g. `start` on an unscheduled task).
    Empty,
}

/// Resolve `field` on `task` to a [`CellValue`].
///
/// `schedule` supplies the computed dates/slack/critical built-ins (a task absent from
/// the schedule yields `Empty` for those). `project` is threaded through for the computed
/// custom-field resolution added in a later PR; today custom fields read their stored
/// value.
pub fn cell(
    project: &ProjectState,
    task: &Task,
    field: &FieldRef,
    schedule: &ScheduleResult,
) -> CellValue {
    let _ = project; // reserved for formula/rollup resolution (see module docs)
    match field {
        FieldRef::Builtin(name) => builtin_cell(task, name, schedule),
        FieldRef::Custom(id) => task.fields.get(id).map_or(CellValue::Empty, value_cell),
    }
}

/// Resolve a built-in column. Unknown names are `Empty` (graceful, never an error).
fn builtin_cell(task: &Task, name: &str, schedule: &ScheduleResult) -> CellValue {
    let dates = schedule.dates.get(&task.id);
    match name {
        "name" => CellValue::Text(task.name.clone()),
        "notes" => CellValue::Text(task.notes.clone()),
        "status" => task
            .status
            .as_ref()
            .map_or(CellValue::Empty, |s| CellValue::Text(s.to_string())),
        "kind" => CellValue::Text(
            match task.kind {
                TaskKind::Leaf => "leaf",
                TaskKind::Summary => "summary",
                TaskKind::Milestone => "milestone",
            }
            .to_string(),
        ),
        "completed" => CellValue::Bool(task.completed),
        "percentComplete" => CellValue::Number(task.percent_complete as f64),
        "duration" => task.schedule.as_ref().map_or(CellValue::Empty, |s| {
            CellValue::Number(s.duration.working_minutes as f64)
        }),
        "deadline" => CellValue::Date(task.schedule.as_ref().and_then(|s| s.deadline)),
        "start" | "scheduledStart" => CellValue::Date(dates.map(|d| d.scheduled_start)),
        "finish" | "scheduledFinish" => CellValue::Date(dates.map(|d| d.scheduled_finish)),
        "earlyStart" => CellValue::Date(dates.map(|d| d.early_start)),
        "earlyFinish" => CellValue::Date(dates.map(|d| d.early_finish)),
        "lateStart" => CellValue::Date(dates.map(|d| d.late_start)),
        "lateFinish" => CellValue::Date(dates.map(|d| d.late_finish)),
        "totalSlack" => dates.map_or(CellValue::Empty, |d| {
            CellValue::Number(d.total_slack as f64)
        }),
        "freeSlack" => dates.map_or(CellValue::Empty, |d| CellValue::Number(d.free_slack as f64)),
        "critical" => dates.map_or(CellValue::Empty, |d| CellValue::Bool(d.critical)),
        _ => CellValue::Empty,
    }
}

/// Map a stored [`FieldValue`] to a [`CellValue`]. Durations and money become their
/// integer magnitude (minutes / minor units) so they compare and sort numerically;
/// [`format_cell`] renders them per project conventions. Empty selects/refs are `Empty`.
fn value_cell(v: &FieldValue) -> CellValue {
    match v {
        FieldValue::Text(s) => CellValue::Text(s.clone()),
        FieldValue::Number(n) => CellValue::Number(*n),
        FieldValue::Bool(b) => CellValue::Bool(*b),
        FieldValue::Date(d) => CellValue::Date(Some(*d)),
        FieldValue::Duration(d) => CellValue::Number(d.working_minutes as f64),
        FieldValue::Money(m) => CellValue::Number(m.minor_units as f64),
        FieldValue::Select(opts) if !opts.is_empty() => CellValue::Text(opts.join(", ")),
        FieldValue::Ref(ids) if !ids.is_empty() => CellValue::Text(ids.join(", ")),
        FieldValue::Select(_) | FieldValue::Ref(_) => CellValue::Empty,
    }
}

/// Render a [`CellValue`] to a **display string**, per the project's conventions.
///
/// The engine owns formatting so every host shows the same thing: dates as `YYYY-MM-DD`,
/// booleans as `✓`/`○`, working-time built-ins in the project's [`DurationUnit`], and
/// `percentComplete` with a trailing `%`. Hosts still receive the typed [`CellValue`]
/// too, so a renderer that wants a checkbox rather than a glyph can use that instead.
pub fn format_cell(value: &CellValue, field: &FieldRef, settings: &ProjectSettings) -> String {
    match value {
        CellValue::Empty => String::new(),
        CellValue::Text(s) => s.clone(),
        CellValue::Bool(b) => if *b { "✓" } else { "○" }.to_string(),
        CellValue::Date(None) => String::new(),
        CellValue::Date(Some(d)) => {
            let (y, m, day) = d.to_ymd();
            format!("{y:04}-{m:02}-{day:02}")
        }
        CellValue::Number(n) => format_number(*n, field, settings),
    }
}

/// Format a numeric cell, special-casing the working-time and percent built-ins.
fn format_number(n: f64, field: &FieldRef, settings: &ProjectSettings) -> String {
    if let FieldRef::Builtin(name) = field {
        match name.as_str() {
            "percentComplete" => return format!("{}%", trim_number(n)),
            "duration" | "totalSlack" | "freeSlack" => return format_working_minutes(n, settings),
            _ => {}
        }
    }
    trim_number(n)
}

/// Working minutes in the project's duration unit, e.g. `3d`, `1.5d`, `4h`, `90m`.
fn format_working_minutes(minutes: f64, settings: &ProjectSettings) -> String {
    let per_day = settings.hours_per_day.max(1) as f64 * 60.0;
    let per_week = per_day * settings.days_per_week.max(1) as f64;
    match settings.duration_unit {
        DurationUnit::Minutes => format!("{}m", trim_number(minutes)),
        DurationUnit::Hours => format!("{}h", trim_number(minutes / 60.0)),
        DurationUnit::Days => format!("{}d", trim_number(minutes / per_day)),
        DurationUnit::Weeks => format!("{}w", trim_number(minutes / per_week)),
    }
}

/// Print a float without a trailing `.0` (so `3.0` shows as `3`, `1.5` as `1.5`).
fn trim_number(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{}", n as i64)
    } else {
        // Trim to at most 2 decimals, then drop trailing zeros.
        let s = format!("{n:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The selection pipeline: filter → sort → group
// ─────────────────────────────────────────────────────────────────────────────
//
// This is the second half of the view layer: given a `View`, produce the tasks it
// shows, in order, partitioned into its groups. Every step resolves field values
// through [`cell`], so what you filter on, sort by, group on, and display are the same
// interpretation of a field. Shape-specific projections (table, calendar, the filtered
// todo/kanban/gantt) are thin maps over this selection.

/// One group of a view's selection: the tasks sharing a group-by key, already in the
/// view's sort order. An ungrouped view yields a single group with an `Empty` key.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SelectionGroup {
    /// The raw group key (`Empty` for the no-value group, or an ungrouped view).
    pub key: CellValue,
    /// A display label for the key (via [`format_cell`]), for a section header.
    pub key_label: String,
    /// The task ids in this group, in the view's sort order.
    pub tasks: Vec<TaskId>,
}

/// Apply a view's **filter → sort → group** to a project's tasks.
///
/// Summary tasks are excluded — they are outline structure, not rows. The result is a
/// list of groups; an ungrouped view returns exactly one group (`Empty` key) holding
/// every matching task in sort order.
pub fn select(
    project: &ProjectState,
    view: &View,
    schedule: &ScheduleResult,
) -> Vec<SelectionGroup> {
    // 1. Filter.
    let mut tasks: Vec<&Task> = project
        .tasks
        .values()
        .filter(|t| t.kind != TaskKind::Summary)
        .filter(|t| passes_filter(t, &view.filter))
        .collect();

    // 2. Sort by the view's keys, tie-broken by outline order then id so the result is
    //    deterministic even when every key compares equal.
    tasks.sort_by(|a, b| {
        compare_by_keys(project, a, b, &view.sort, schedule)
            .then_with(|| a.order.cmp(&b.order))
            .then_with(|| a.id.0.cmp(&b.id.0))
    });

    // 3. Group.
    match &view.group_by {
        None => vec![SelectionGroup {
            key: CellValue::Empty,
            key_label: String::new(),
            tasks: tasks.iter().map(|t| t.id.clone()).collect(),
        }],
        Some(field) => group_tasks(project, &tasks, field, schedule),
    }
}

/// Whether a task satisfies a filter. The current `Filter` fields (status set,
/// completion, name search) are evaluated directly; the richer field-predicate tree
/// from the spec layers on here in a follow-up.
fn passes_filter(task: &Task, filter: &Filter) -> bool {
    if !filter.statuses.is_empty() {
        match &task.status {
            Some(s) if filter.statuses.contains(s) => {}
            _ => return false,
        }
    }
    if let Some(want) = filter.completed {
        if task.completed != want {
            return false;
        }
    }
    if let Some(q) = &filter.search {
        if !task.name.to_lowercase().contains(&q.to_lowercase()) {
            return false;
        }
    }
    true
}

/// Compare two tasks by a list of sort keys, first non-equal key wins. A descending key
/// reverses that key's order; ties fall through to the next key.
fn compare_by_keys(
    project: &ProjectState,
    a: &Task,
    b: &Task,
    keys: &[SortKey],
    schedule: &ScheduleResult,
) -> Ordering {
    for key in keys {
        let va = cell(project, a, &key.field, schedule);
        let vb = cell(project, b, &key.field, schedule);
        let ord = cmp_cell(&va, &vb);
        let ord = if key.ascending { ord } else { ord.reverse() };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    Ordering::Equal
}

/// A total order over cell values: **`Empty` sorts last** (missing data at the bottom),
/// same-typed values compare naturally (numbers by `total_cmp` so `NaN` can't break the
/// order), and unlike-typed values fall back to a stable type rank so a mixed column
/// never panics.
fn cmp_cell(a: &CellValue, b: &CellValue) -> Ordering {
    use CellValue::*;
    match (a, b) {
        (Empty, Empty) => Ordering::Equal,
        (Empty, _) => Ordering::Greater, // Empty last
        (_, Empty) => Ordering::Less,
        (Text(x), Text(y)) => x.cmp(y),
        (Number(x), Number(y)) => x.total_cmp(y),
        (Bool(x), Bool(y)) => x.cmp(y), // false < true
        (Date(x), Date(y)) => match (x, y) {
            (Some(dx), Some(dy)) => dx.0.cmp(&dy.0),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        },
        // Unlike types: order by a stable rank so the sort stays total.
        _ => type_rank(a).cmp(&type_rank(b)),
    }
}

fn type_rank(v: &CellValue) -> u8 {
    match v {
        CellValue::Bool(_) => 0,
        CellValue::Number(_) => 1,
        CellValue::Date(_) => 2,
        CellValue::Text(_) => 3,
        CellValue::Empty => 4,
    }
}

/// Partition already-sorted tasks into groups keyed by `field`. Group order follows the
/// key order ([`cmp_cell`], so the no-value group lands last); within a group tasks keep
/// their incoming sort order.
fn group_tasks(
    project: &ProjectState,
    tasks: &[&Task],
    field: &FieldRef,
    schedule: &ScheduleResult,
) -> Vec<SelectionGroup> {
    let mut groups: Vec<SelectionGroup> = Vec::new();
    for t in tasks {
        let key = cell(project, t, field, schedule);
        match groups.iter_mut().find(|g| g.key == key) {
            Some(g) => g.tasks.push(t.id.clone()),
            None => {
                let key_label = format_cell(&key, field, &project.settings);
                groups.push(SelectionGroup {
                    key,
                    key_label,
                    tasks: vec![t.id.clone()],
                });
            }
        }
    }
    groups.sort_by(|a, b| cmp_cell(&a.key, &b.key));
    groups
}

// ─────────────────────────────────────────────────────────────────────────────
// The table (sheet) projection — a render-ready map over the selection
// ─────────────────────────────────────────────────────────────────────────────

/// A render-ready spreadsheet: the columns to draw and the grouped, ordered rows, every
/// cell already resolved **and** formatted. A host draws this directly — no field access,
/// no formatting, no sorting on its side. This is the "dumb UI" contract the sheet
/// component (Phase 5) renders.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TableView {
    /// The columns, in display order.
    pub columns: Vec<ColumnHeader>,
    /// The rows, partitioned into the view's groups (one group if ungrouped).
    pub groups: Vec<TableGroup>,
}

/// A column: which field it shows, its header label, and its value kind (so a host can
/// right-align numbers, render a checkbox for bools, etc.).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ColumnHeader {
    /// The field this column resolves.
    pub field: FieldRef,
    /// The human-facing header label.
    pub label: String,
    /// The column's value kind.
    pub kind: ColumnKind,
}

/// The kind of a column's values — a rendering hint, not the exact `CellValue` variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum ColumnKind {
    /// Free text.
    Text,
    /// A number (percent, duration, money, computed).
    Number,
    /// A date.
    Date,
    /// A boolean.
    Bool,
}

/// A group of rows under a group-by key (the whole table is one group when ungrouped).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TableGroup {
    /// The group's display label (empty for the ungrouped / no-value group).
    pub key_label: String,
    /// The rows in this group, in the view's sort order.
    pub rows: Vec<TableRow>,
}

/// One row: the task it's for, and one cell per column, in column order.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TableRow {
    /// The task this row renders.
    pub task: TaskId,
    /// One cell per column, in the same order as [`TableView::columns`].
    pub cells: Vec<Cell>,
}

/// A single cell: the typed value (for a host that wants a control) **and** its
/// engine-formatted display string (for a host that just draws text).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Cell {
    /// The resolved value.
    pub value: CellValue,
    /// The value rendered per project conventions (via [`format_cell`]).
    pub display: String,
}

/// Build the render-ready table for `view`. The view's `visible_fields` become the
/// columns (defaulting to a single `name` column when none are set); [`select`] provides
/// the grouped, ordered rows.
pub fn table(project: &ProjectState, view: &View, schedule: &ScheduleResult) -> TableView {
    let fields: Vec<FieldRef> = if view.visible_fields.is_empty() {
        vec![FieldRef::Builtin("name".into())]
    } else {
        view.visible_fields.clone()
    };

    let columns = fields
        .iter()
        .map(|f| ColumnHeader {
            field: f.clone(),
            label: column_label(project, f),
            kind: column_kind(project, f),
        })
        .collect();

    let groups = select(project, view, schedule)
        .into_iter()
        .map(|g| TableGroup {
            key_label: g.key_label,
            rows: g
                .tasks
                .iter()
                .filter_map(|id| {
                    project.tasks.get(id).map(|t| TableRow {
                        task: id.clone(),
                        cells: fields
                            .iter()
                            .map(|f| {
                                let value = cell(project, t, f, schedule);
                                let display = format_cell(&value, f, &project.settings);
                                Cell { value, display }
                            })
                            .collect(),
                    })
                })
                .collect(),
        })
        .collect();

    TableView { columns, groups }
}

/// A column's header label: a friendly name for a built-in, or the custom field's name.
fn column_label(project: &ProjectState, field: &FieldRef) -> String {
    match field {
        FieldRef::Builtin(name) => builtin_label(name).to_string(),
        FieldRef::Custom(id) => project
            .fields
            .get(id)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| id.0.clone()),
    }
}

/// Friendly labels for the built-in columns; an unknown name is shown verbatim.
fn builtin_label(name: &str) -> &str {
    match name {
        "name" => "Name",
        "notes" => "Notes",
        "status" => "Status",
        "kind" => "Type",
        "completed" => "Done",
        "percentComplete" => "% Complete",
        "duration" => "Duration",
        "deadline" => "Deadline",
        "start" | "scheduledStart" => "Start",
        "finish" | "scheduledFinish" => "Finish",
        "earlyStart" => "Early Start",
        "earlyFinish" => "Early Finish",
        "lateStart" => "Late Start",
        "lateFinish" => "Late Finish",
        "totalSlack" => "Total Slack",
        "freeSlack" => "Free Slack",
        "critical" => "Critical",
        other => other,
    }
}

/// A column's value kind, for a host's rendering choices. Matches how [`cell`] resolves
/// the field, so the hint never contradicts the values.
fn column_kind(project: &ProjectState, field: &FieldRef) -> ColumnKind {
    match field {
        FieldRef::Builtin(name) => match name.as_str() {
            "completed" | "critical" => ColumnKind::Bool,
            "percentComplete" | "duration" | "totalSlack" | "freeSlack" => ColumnKind::Number,
            "deadline" | "start" | "scheduledStart" | "finish" | "scheduledFinish"
            | "earlyStart" | "earlyFinish" | "lateStart" | "lateFinish" => ColumnKind::Date,
            _ => ColumnKind::Text,
        },
        FieldRef::Custom(id) => match project.fields.get(id).map(|f| &f.kind) {
            Some(FieldKind::Bool) => ColumnKind::Bool,
            Some(
                FieldKind::Number
                | FieldKind::Duration
                | FieldKind::Money
                | FieldKind::Formula { .. }
                | FieldKind::Rollup { .. },
            ) => ColumnKind::Number,
            Some(FieldKind::Date) => ColumnKind::Date,
            _ => ColumnKind::Text,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{FieldId, ProjectId, StatusId, TaskId};
    use crate::model::{TaskKind, TaskSchedule};
    use crate::primitives::{Duration, Money, Work};

    fn project() -> ProjectState {
        ProjectState::empty(ProjectId::from_raw("p1"))
    }
    fn builtin(name: &str) -> FieldRef {
        FieldRef::Builtin(name.to_string())
    }
    /// An empty schedule (no computed dates) unless a test supplies one.
    fn no_schedule() -> ScheduleResult {
        project()
            .schedule(Date::from_ymd(2026, 7, 13).unwrap())
            .unwrap()
    }

    #[test]
    fn built_in_scalar_fields_resolve() {
        let p = project();
        let mut t = Task::new(TaskId::from_raw("a"), "Write spec");
        t.notes = "the notes".into();
        t.completed = true;
        t.percent_complete = 42;
        t.status = Some(StatusId::from_raw("doing"));
        t.kind = TaskKind::Milestone;
        let sched = no_schedule();

        assert_eq!(
            cell(&p, &t, &builtin("name"), &sched),
            CellValue::Text("Write spec".into())
        );
        assert_eq!(
            cell(&p, &t, &builtin("notes"), &sched),
            CellValue::Text("the notes".into())
        );
        assert_eq!(
            cell(&p, &t, &builtin("completed"), &sched),
            CellValue::Bool(true)
        );
        assert_eq!(
            cell(&p, &t, &builtin("percentComplete"), &sched),
            CellValue::Number(42.0)
        );
        assert_eq!(
            cell(&p, &t, &builtin("status"), &sched),
            CellValue::Text("doing".into())
        );
        assert_eq!(
            cell(&p, &t, &builtin("kind"), &sched),
            CellValue::Text("milestone".into())
        );
        // Unknown built-in degrades to Empty, never a panic.
        assert_eq!(cell(&p, &t, &builtin("nope"), &sched), CellValue::Empty);
        // A schedule built-in on an unscheduled task is Empty.
        assert_eq!(
            cell(&p, &t, &builtin("start"), &sched),
            CellValue::Date(None)
        );
    }

    #[test]
    fn scheduled_fields_read_computed_dates() {
        // One 1-day task starting Monday 2026-07-13; it appears in the schedule.
        let mut p = project();
        let mut t = Task::new(TaskId::from_raw("a"), "A");
        t.schedule = Some(TaskSchedule {
            duration: Duration::minutes(8 * 60),
            work: Work::minutes(8 * 60),
            ..TaskSchedule::default()
        });
        p.tasks.insert(t.id.clone(), t.clone());
        let sched = p.schedule(Date::from_ymd(2026, 7, 13).unwrap()).unwrap();

        assert_eq!(
            cell(&p, &t, &builtin("start"), &sched),
            CellValue::Date(Some(Date::from_ymd(2026, 7, 13).unwrap()))
        );
        assert_eq!(
            cell(&p, &t, &builtin("duration"), &sched),
            CellValue::Number(480.0)
        );
        assert_eq!(
            cell(&p, &t, &builtin("critical"), &sched),
            CellValue::Bool(true)
        );
    }

    #[test]
    fn custom_stored_values_resolve() {
        let p = project();
        let mut t = Task::new(TaskId::from_raw("a"), "A");
        let fid = FieldId::from_raw("f1");
        t.fields.insert(
            fid.clone(),
            FieldValue::Money(Money {
                minor_units: 1500,
                currency: "USD".into(),
            }),
        );
        let sched = no_schedule();
        // Money resolves to its minor-unit magnitude for comparison/sorting.
        assert_eq!(
            cell(&p, &t, &FieldRef::Custom(fid), &sched),
            CellValue::Number(1500.0)
        );
        // An unset custom field is Empty.
        assert_eq!(
            cell(
                &p,
                &t,
                &FieldRef::Custom(FieldId::from_raw("missing")),
                &sched
            ),
            CellValue::Empty
        );
    }

    #[test]
    fn multi_select_joins_and_empty_is_empty() {
        assert_eq!(
            value_cell(&FieldValue::Select(vec!["a".into(), "b".into()])),
            CellValue::Text("a, b".into())
        );
        assert_eq!(value_cell(&FieldValue::Select(vec![])), CellValue::Empty);
    }

    #[test]
    fn formatting_is_render_ready() {
        let s = ProjectSettings::default(); // days, 8h/day, 5d/week
        assert_eq!(format_cell(&CellValue::Empty, &builtin("name"), &s), "");
        assert_eq!(
            format_cell(&CellValue::Bool(true), &builtin("completed"), &s),
            "✓"
        );
        assert_eq!(
            format_cell(&CellValue::Bool(false), &builtin("completed"), &s),
            "○"
        );
        assert_eq!(
            format_cell(
                &CellValue::Date(Some(Date::from_ymd(2026, 7, 5).unwrap())),
                &builtin("start"),
                &s
            ),
            "2026-07-05"
        );
        assert_eq!(
            format_cell(&CellValue::Date(None), &builtin("deadline"), &s),
            ""
        );
        assert_eq!(
            format_cell(&CellValue::Number(42.0), &builtin("percentComplete"), &s),
            "42%"
        );
        // 8h = one working day.
        assert_eq!(
            format_cell(&CellValue::Number(480.0), &builtin("duration"), &s),
            "1d"
        );
        // 12h = 1.5 days.
        assert_eq!(
            format_cell(&CellValue::Number(720.0), &builtin("duration"), &s),
            "1.5d"
        );
        // A plain number trims its trailing .0.
        assert_eq!(
            format_cell(
                &CellValue::Number(1500.0),
                &FieldRef::Custom(FieldId::from_raw("f1")),
                &s
            ),
            "1500"
        );
    }

    #[test]
    fn duration_unit_changes_the_display() {
        let hours = ProjectSettings {
            duration_unit: DurationUnit::Hours,
            ..ProjectSettings::default()
        };
        assert_eq!(
            format_cell(&CellValue::Number(480.0), &builtin("duration"), &hours),
            "8h"
        );
        let minutes = ProjectSettings {
            duration_unit: DurationUnit::Minutes,
            ..ProjectSettings::default()
        };
        assert_eq!(
            format_cell(&CellValue::Number(90.0), &builtin("totalSlack"), &minutes),
            "90m"
        );
    }

    // ── selection pipeline: filter → sort → group ───────────────────────────────

    use crate::ids::{StatusId as SId, ViewId};
    use crate::model::{Filter, SortKey, View, ViewShape};

    /// A project with four leaf tasks + one summary (which the pipeline must exclude).
    fn sample() -> ProjectState {
        let mut p = project();
        let mk = |id: &str, name: &str, done: bool, status: Option<&str>, pct: u8| {
            let mut t = Task::new(TaskId::from_raw(id), name);
            t.completed = done;
            t.status = status.map(SId::from_raw);
            t.percent_complete = pct;
            t
        };
        for t in [
            mk("a", "Alpha", false, Some("doing"), 10),
            mk("b", "Bravo", true, Some("done"), 100),
            mk("c", "Charlie", false, Some("doing"), 60),
            mk("d", "Delta", false, None, 0),
        ] {
            p.tasks.insert(t.id.clone(), t);
        }
        let mut summary = Task::new(TaskId::from_raw("s"), "Summary");
        summary.kind = TaskKind::Summary;
        p.tasks.insert(summary.id.clone(), summary);
        p
    }

    fn view(filter: Filter, sort: Vec<SortKey>, group_by: Option<FieldRef>) -> View {
        View {
            id: ViewId::from_raw("v1"),
            name: "V".into(),
            shape: ViewShape::Table,
            filter,
            group_by,
            sort,
            visible_fields: vec![],
        }
    }
    fn asc(field: &str) -> SortKey {
        SortKey {
            field: builtin(field),
            ascending: true,
        }
    }
    /// The flat id order of a single-group (ungrouped) selection.
    fn ids(groups: &[SelectionGroup]) -> Vec<String> {
        assert_eq!(groups.len(), 1, "expected an ungrouped selection");
        groups[0].tasks.iter().map(|t| t.0.clone()).collect()
    }

    #[test]
    fn filter_excludes_summaries_and_honours_the_predicates() {
        let p = sample();
        let sched = no_schedule();

        // No filter: all four leaves, never the summary.
        let all = select(
            &p,
            &view(Filter::default(), vec![asc("name")], None),
            &sched,
        );
        assert_eq!(ids(&all), ["a", "b", "c", "d"]);

        // completed = false.
        let f = Filter {
            completed: Some(false),
            ..Filter::default()
        };
        assert_eq!(
            ids(&select(&p, &view(f, vec![asc("name")], None), &sched)),
            ["a", "c", "d"]
        );

        // status ∈ {doing}.
        let f = Filter {
            statuses: vec![SId::from_raw("doing")],
            ..Filter::default()
        };
        assert_eq!(
            ids(&select(&p, &view(f, vec![asc("name")], None), &sched)),
            ["a", "c"]
        );

        // case-insensitive name search.
        let f = Filter {
            search: Some("ALP".into()),
            ..Filter::default()
        };
        assert_eq!(
            ids(&select(&p, &view(f, vec![asc("name")], None), &sched)),
            ["a"]
        );
    }

    #[test]
    fn sort_is_multi_key_with_direction_and_empty_last() {
        let p = sample();
        let sched = no_schedule();

        // Descending percentComplete: 100, 60, 10, 0 → b, c, a, d.
        let desc = SortKey {
            field: builtin("percentComplete"),
            ascending: false,
        };
        assert_eq!(
            ids(&select(
                &p,
                &view(Filter::default(), vec![desc], None),
                &sched
            )),
            ["b", "c", "a", "d"]
        );

        // Sort by status (a,c = "doing"; b = "done"; d = Empty → last), tie-break name.
        let by_status = select(
            &p,
            &view(Filter::default(), vec![asc("status"), asc("name")], None),
            &sched,
        );
        assert_eq!(ids(&by_status), ["a", "c", "b", "d"]); // doing<done, Empty(d) last
    }

    #[test]
    fn group_by_partitions_with_no_value_group_last() {
        let p = sample();
        let sched = no_schedule();
        let groups = select(
            &p,
            &view(
                Filter::default(),
                vec![asc("name")],
                Some(builtin("status")),
            ),
            &sched,
        );
        // Groups ordered by key: "doing", "done", then the Empty (no-status) group last.
        let labels: Vec<_> = groups.iter().map(|g| g.key_label.clone()).collect();
        assert_eq!(labels, ["doing", "done", ""]);
        assert_eq!(
            groups[0]
                .tasks
                .iter()
                .map(|t| t.0.clone())
                .collect::<Vec<_>>(),
            ["a", "c"]
        );
        assert_eq!(groups[2].key, CellValue::Empty);
        assert_eq!(
            groups[2]
                .tasks
                .iter()
                .map(|t| t.0.clone())
                .collect::<Vec<_>>(),
            ["d"]
        );
    }

    #[test]
    fn view_selection_computes_the_schedule_for_computed_columns() {
        // Two scheduled tasks (b finishes after a via FS); sort by finish date.
        let mut p = project();
        for (id, name) in [("a", "A"), ("b", "B")] {
            let mut t = Task::new(TaskId::from_raw(id), name);
            t.schedule = Some(TaskSchedule {
                duration: crate::primitives::Duration::minutes(8 * 60),
                ..TaskSchedule::default()
            });
            p.tasks.insert(t.id.clone(), t);
        }
        p.dependencies.push(crate::model::DependencyLink {
            id: crate::ids::LinkId::from_raw("l1"),
            predecessor: TaskId::from_raw("a"),
            successor: TaskId::from_raw("b"),
            kind: crate::model::DependencyKind::FinishToStart,
            lag: crate::primitives::Duration::zero(),
        });
        let groups = p.view_selection(
            &view(Filter::default(), vec![asc("finish")], None),
            Date::from_ymd(2026, 7, 13).unwrap(),
        );
        // a finishes Monday, b Tuesday → a before b by the computed finish column.
        assert_eq!(ids(&groups), ["a", "b"]);
    }

    // ── table projection ─────────────────────────────────────────────────────────

    #[test]
    fn table_columns_carry_labels_and_kinds() {
        let p = sample();
        let sched = no_schedule();
        let v = View {
            visible_fields: vec![
                builtin("name"),
                builtin("completed"),
                builtin("percentComplete"),
            ],
            ..view(Filter::default(), vec![asc("name")], None)
        };
        let t = table(&p, &v, &sched);
        assert_eq!(
            t.columns
                .iter()
                .map(|c| c.label.clone())
                .collect::<Vec<_>>(),
            ["Name", "Done", "% Complete"]
        );
        assert_eq!(t.columns[0].kind, ColumnKind::Text);
        assert_eq!(t.columns[1].kind, ColumnKind::Bool);
        assert_eq!(t.columns[2].kind, ColumnKind::Number);
    }

    #[test]
    fn table_rows_are_grouped_ordered_and_formatted() {
        let p = sample();
        let sched = no_schedule();
        let v = View {
            visible_fields: vec![
                builtin("name"),
                builtin("completed"),
                builtin("percentComplete"),
            ],
            ..view(
                Filter::default(),
                vec![asc("name")],
                Some(builtin("status")),
            )
        };
        let t = table(&p, &v, &sched);

        // Groups follow the selection: doing, done, then the no-status group.
        assert_eq!(
            t.groups
                .iter()
                .map(|g| g.key_label.clone())
                .collect::<Vec<_>>(),
            ["doing", "done", ""]
        );
        // First group ("doing") holds a, c in name order; check a's formatted cells.
        let doing = &t.groups[0];
        assert_eq!(
            doing
                .rows
                .iter()
                .map(|r| r.task.0.clone())
                .collect::<Vec<_>>(),
            ["a", "c"]
        );
        let a = &doing.rows[0];
        assert_eq!(a.cells[0].display, "Alpha"); // name
        assert_eq!(a.cells[1].display, "○"); // completed=false → glyph
        assert_eq!(a.cells[1].value, CellValue::Bool(false)); // typed value still available
        assert_eq!(a.cells[2].display, "10%"); // percentComplete formatted
    }

    #[test]
    fn table_defaults_to_a_name_column_when_none_are_set() {
        let p = sample();
        let sched = no_schedule();
        let t = table(
            &p,
            &view(Filter::default(), vec![asc("name")], None),
            &sched,
        );
        assert_eq!(t.columns.len(), 1);
        assert_eq!(t.columns[0].label, "Name");
        // Ungrouped → one group with all four leaves.
        assert_eq!(t.groups.len(), 1);
        assert_eq!(t.groups[0].rows.len(), 4);
    }
}
