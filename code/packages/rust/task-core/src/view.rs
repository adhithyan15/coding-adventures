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

use crate::model::{
    DurationUnit, FieldRef, FieldValue, ProjectSettings, ProjectState, Task, TaskKind,
};
use crate::primitives::Date;
use crate::scheduler::ScheduleResult;

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
}
