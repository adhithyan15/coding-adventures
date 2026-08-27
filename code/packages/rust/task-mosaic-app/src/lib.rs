#![recursion_limit = "256"]
//! Standard Mosaic application adapter for TaskApp.
//!
//! `task-core` remains the pure domain engine. This crate owns only the portable
//! presentation cursor needed by generated Mosaic hosts and maps the TaskApp MIL
//! slot/event contract to the engine's typed operations and projections.

use mosaic_app_runtime::{
    Announcement, AppUpdate, ColorScheme, Event, MosaicApp, Politeness, Snapshot, StartContext,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use task_core::{
    Constraint, Date, DependencyKind, DependencyLink, Duration, Label, LabelId, LinkId, Note,
    NoteId, Priority, ProjectComplexity, ProjectId, TaskId, Workspace, WorkspaceId,
};

const SNAPSHOT_SCHEMA: &str = "task-mosaic-app/state";
const SNAPSHOT_VERSION: u32 = 1;
const PROJECT_START: Date = Date(20_458); // 2026-01-05, a Monday.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ViewMode {
    List,
    Board,
    Timeline,
    Sheet,
    Calendar,
    Notes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskAppState {
    workspace: Workspace,
    active_project: ProjectId,
    task_order: Vec<TaskId>,
    next_id: u64,
    view: ViewMode,
    dark_theme: bool,
    new_task_name: String,
    new_task_due: String,
    new_project_name: String,
    new_label_name: String,
    expanded_task: Option<TaskId>,
    sheet_selected_row: i64,
    sheet_selected_col: i64,
    sheet_edit_row: i64,
    sheet_edit_col: i64,
    sheet_edit_content: String,
    sheet_filter_text: String,
    sheet_sort_field: String,
    sheet_sort_open: bool,
    sheet_sort_ascending: bool,
    calendar_month_start: Date,
    selected_note_id: Option<NoteId>,
    note_title: String,
    note_body: String,
    note_task_name: String,
}

impl Default for TaskAppState {
    fn default() -> Self {
        let project = ProjectId::from_raw("project");
        let mut workspace = Workspace::empty(WorkspaceId::from_raw("workspace"), project.clone());
        workspace
            .projects
            .get_mut(&project)
            .expect("fresh workspace contains its root")
            .set_project_name("Inbox");
        Self {
            workspace,
            active_project: project,
            task_order: Vec::new(),
            next_id: 0,
            view: ViewMode::List,
            dark_theme: false,
            new_task_name: String::new(),
            new_task_due: String::new(),
            new_project_name: String::new(),
            new_label_name: String::new(),
            expanded_task: None,
            sheet_selected_row: -1,
            sheet_selected_col: -1,
            sheet_edit_row: -1,
            sheet_edit_col: -1,
            sheet_edit_content: String::new(),
            sheet_filter_text: String::new(),
            sheet_sort_field: String::new(),
            sheet_sort_open: false,
            sheet_sort_ascending: true,
            calendar_month_start: Date::from_ymd(2026, 1, 1).expect("valid fixed date"),
            selected_note_id: None,
            note_title: String::new(),
            note_body: String::new(),
            note_task_name: String::new(),
        }
    }
}

/// Concrete standard-ABI TaskApp implementation.
#[derive(Debug, Clone, Default)]
pub struct TaskMosaicApp {
    state: TaskAppState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskAppError {
    UnknownEvent(String),
    InvalidPayload { event: String, field: &'static str },
    Engine(String),
    InvalidSnapshot,
}

impl fmt::Display for TaskAppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEvent(event) => write!(formatter, "unknown TaskApp event `{event}`"),
            Self::InvalidPayload { event, field } => {
                write!(formatter, "TaskApp event `{event}` has invalid `{field}`")
            }
            Self::Engine(message) => {
                write!(formatter, "task-core rejected the operation: {message}")
            }
            Self::InvalidSnapshot => formatter.write_str("invalid TaskApp snapshot"),
        }
    }
}

impl Error for TaskAppError {}

impl TaskMosaicApp {
    fn update(&self) -> AppUpdate {
        AppUpdate::new(self.props())
    }

    fn announced_update(&self, message: impl Into<String>) -> AppUpdate {
        let mut update = self.update();
        update.announcements.push(Announcement {
            politeness: Politeness::Polite,
            message: message.into(),
        });
        update
    }

    fn active_project(&self) -> &task_core::ProjectState {
        self.state
            .workspace
            .projects
            .get(&self.state.active_project)
            .expect("active project is repaired when state is loaded")
    }

    fn active_project_mut(&mut self) -> &mut task_core::ProjectState {
        self.state
            .workspace
            .projects
            .get_mut(&self.state.active_project)
            .expect("active project is repaired when state is loaded")
    }

    fn repair(&mut self) {
        if !self
            .state
            .workspace
            .projects
            .contains_key(&self.state.active_project)
        {
            if let Some(project) = self
                .state
                .workspace
                .roots
                .first()
                .or_else(|| self.state.workspace.projects.keys().next())
            {
                self.state.active_project = project.clone();
            }
        }
        self.state.task_order.retain(|task| {
            self.state
                .workspace
                .projects
                .values()
                .any(|project| project.tasks.contains_key(task))
        });
        if self.active_project().settings.complexity == ProjectComplexity::Board
            && self.state.view == ViewMode::Timeline
        {
            self.state.view = ViewMode::List;
        }
    }

    fn ordered_task_ids(&self) -> Vec<TaskId> {
        let project = self.active_project();
        let mut ids: Vec<TaskId> = self
            .state
            .task_order
            .iter()
            .filter(|id| project.tasks.contains_key(*id))
            .cloned()
            .collect();
        let mut seen: BTreeSet<TaskId> = ids.iter().cloned().collect();
        for id in project.tasks.keys() {
            if seen.insert(id.clone()) {
                ids.push(id.clone());
            }
        }
        ids
    }

    fn task_ids(&self) -> Vec<TaskId> {
        let project = self.active_project();
        let mut ids = self.ordered_task_ids();
        ids.sort_by_key(|id| {
            let task = &project.tasks[id];
            if task.completed {
                2
            } else if task.percent_complete > 0 {
                0
            } else {
                1
            }
        });
        ids
    }

    fn project_rows(&self) -> (Vec<ProjectId>, Vec<Vec<String>>) {
        let all = &self.state.workspace.projects;
        let roots = self.state.workspace.roots.clone();
        let mut stack: Vec<(ProjectId, usize)> =
            roots.into_iter().rev().map(|id| (id, 0)).collect();
        let mut seen = BTreeSet::new();
        let mut ordered = Vec::new();
        let mut depths = Vec::new();
        while let Some((id, depth)) = stack.pop() {
            if !seen.insert(id.clone()) {
                continue;
            }
            ordered.push(id.clone());
            depths.push(depth);
            let mut children: Vec<ProjectId> = all
                .values()
                .filter(|project| project.parent.as_ref() == Some(&id))
                .map(|project| project.id.clone())
                .collect();
            children.sort_by_key(|child| all[child].name.clone());
            for child in children.into_iter().rev() {
                stack.push((child, depth + 1));
            }
        }
        for id in all.keys() {
            if seen.insert(id.clone()) {
                ordered.push(id.clone());
                depths.push(0);
            }
        }
        let rows = ordered
            .iter()
            .zip(depths)
            .map(|(id, depth)| {
                let project = &all[id];
                vec![
                    if project.name.is_empty() {
                        id.to_string()
                    } else {
                        project.name.clone()
                    },
                    if id == &self.state.active_project {
                        "active".to_string()
                    } else {
                        String::new()
                    },
                    if depth > 0 {
                        format!("{}↳", "  ".repeat(depth - 1))
                    } else {
                        String::new()
                    },
                ]
            })
            .collect();
        (ordered, rows)
    }

    fn props(&self) -> Value {
        let project = self.active_project();
        let ids = self.task_ids();
        let schedule = project.schedule(PROJECT_START).ok();
        let done_count = ids.iter().filter(|id| project.tasks[*id].completed).count();
        let overdue_count = ids
            .iter()
            .filter(|id| {
                let task = &project.tasks[*id];
                !task.completed
                    && task
                        .schedule
                        .as_ref()
                        .and_then(|details| details.deadline)
                        .is_some_and(|deadline| {
                            schedule
                                .as_ref()
                                .and_then(|result| result.dates.get(*id))
                                .is_some_and(|dates| dates.scheduled_finish > deadline)
                        })
            })
            .count();
        let percent = if ids.is_empty() {
            0
        } else {
            ((done_count * 100) as f64 / ids.len() as f64).round() as u64
        };
        let full = project.settings.complexity == ProjectComplexity::Full;
        let (_, project_rows) = self.project_rows();
        let task_rows = self.task_rows(&ids, schedule.as_ref());
        let board_cards = if self.state.view == ViewMode::Board {
            ids.iter()
                .map(|id| {
                    let task = &project.tasks[id];
                    vec![
                        task.name.clone(),
                        if task.completed {
                            "done"
                        } else if task.percent_complete > 0 {
                            "doing"
                        } else {
                            "next"
                        }
                        .to_string(),
                        id.to_string(),
                        String::new(),
                    ]
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let (timeline_scale, timeline_grid, timeline_rows) =
            self.timeline_props(&ids, schedule.as_ref());
        let (calendar_cells, calendar_events) = self.calendar_props(&ids, schedule.as_ref());
        let sheet_rows = self.sheet_rows(&ids);
        let mut notes: Vec<_> = project.notes.values().collect();
        notes.sort_by_key(|note| note.title.to_lowercase());
        let note_rows = if self.state.view == ViewMode::Notes {
            notes
                .iter()
                .map(|note| {
                    vec![
                        note.id.to_string(),
                        if note.title.trim().is_empty() {
                            "Untitled".to_string()
                        } else {
                            note.title.clone()
                        },
                    ]
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        json!({
            "app-title": "Tasks — auto-scheduled",
            "new-task-name": self.state.new_task_name,
            "new-task-due": self.state.new_task_due,
            "new-project-name": self.state.new_project_name,
            "project-rows": project_rows,
            "summary": format!("{} task(s) · {} done · projected finish {}", ids.len(), done_count,
                schedule.as_ref().and_then(|result| result.project_finish).map(format_date).unwrap_or_else(|| "—".to_string())),
            "status-label": if overdue_count > 0 { format!("{overdue_count} overdue") } else { "On track".to_string() },
            "status-warn": if overdue_count > 0 { "warn" } else { "" },
            "ring-gradient": "",
            "ring-percent": format!("{percent}%"),
            "ring-percent-value": percent,
            "theme-is-dark": if self.state.dark_theme { "dark" } else { "" },
            "complexity-label": if full { "Full CPM" } else { "Board" },
            "allow-timeline": if full { "full" } else { "" },
            "timeline-mode": if self.state.view == ViewMode::Timeline { "timeline" } else { "" },
            "timeline-grid": timeline_grid,
            "timeline-scale": timeline_scale,
            "timeline-rows": timeline_rows,
            "board-mode": if self.state.view == ViewMode::Board { "board" } else { "" },
            "board-columns": [["Up next", "next"], ["In progress", "doing"], ["Done", "done"]],
            "board-cards": board_cards,
            "sheet-mode": if self.state.view == ViewMode::Sheet { "sheet" } else { "" },
            "sheet-viewport-rows": sheet_rows,
            "sheet-column-headers": ["Name", "Done", "Due", "Priority", "Labels"],
            "sheet-column-widths": [3, 1, 2, 2, 2],
            "sheet-selected-row": self.state.sheet_selected_row,
            "sheet-selected-col": self.state.sheet_selected_col,
            "sheet-edit-row": self.state.sheet_edit_row,
            "sheet-edit-col": self.state.sheet_edit_col,
            "sheet-edit-content": self.state.sheet_edit_content,
            "sheet-filter-text": self.state.sheet_filter_text,
            "sheet-sort-field": if self.state.sheet_sort_field.is_empty() { "Sort by…" } else { &self.state.sheet_sort_field },
            "sheet-sort-options": ["Name", "Done", "Due", "Priority"],
            "sheet-sort-open": self.state.sheet_sort_open,
            "sheet-sort-ascending": self.state.sheet_sort_ascending,
            "new-label-name": self.state.new_label_name,
            "calendar-mode": if self.state.view == ViewMode::Calendar { "calendar" } else { "" },
            "calendar-title": if self.state.view == ViewMode::Calendar { month_label(self.state.calendar_month_start) } else { String::new() },
            "calendar-cells": calendar_cells,
            "calendar-events": calendar_events,
            "notes-mode": if self.state.view == ViewMode::Notes { "notes" } else { "" },
            "notes-title": "Notes",
            "note-rows": note_rows,
            "selected-note-id": self.state.selected_note_id.as_ref().map(ToString::to_string).unwrap_or_default(),
            "note-title-value": self.state.note_title,
            "note-body-value": self.state.note_body,
            "note-task-value": self.state.note_task_name,
            "task-rows": task_rows,
        })
    }

    fn task_rows(
        &self,
        ids: &[TaskId],
        schedule: Option<&task_core::scheduler::ScheduleResult>,
    ) -> Vec<Vec<String>> {
        let project = self.active_project();
        let full = project.settings.complexity == ProjectComplexity::Full;
        let mut last_group = String::new();
        let group_size = |group: &str| {
            ids.iter()
                .filter(|id| {
                    let task = &project.tasks[*id];
                    task_group(task.completed, task.percent_complete) == group
                })
                .count()
        };
        ids.iter()
            .map(|id| {
                let task = &project.tasks[id];
                let dates = schedule.and_then(|result| result.dates.get(id));
                let deadline = task.schedule.as_ref().and_then(|details| details.deadline);
                let open = self.state.expanded_task.as_ref() == Some(id);
                let group = task_group(task.completed, task.percent_complete);
                let heading = if group == last_group {
                    String::new()
                } else {
                    last_group = group.to_string();
                    capitalize(group)
                };
                let dependencies = if open {
                    project
                        .dependencies
                        .iter()
                        .filter_map(|link| {
                            if &link.predecessor == id {
                                Some(format!(
                                    "→ {} ({})",
                                    project.tasks[&link.successor].name,
                                    dependency_label(link.kind)
                                ))
                            } else if &link.successor == id {
                                Some(format!(
                                    "← {} ({})",
                                    project.tasks[&link.predecessor].name,
                                    dependency_label(link.kind)
                                ))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" · ")
                } else {
                    String::new()
                };
                let note_text = if open {
                    project
                        .notes
                        .values()
                        .filter(|note| note.attached_task.as_ref() == Some(id))
                        .map(|note| note.body.trim())
                        .filter(|body| !body.is_empty())
                        .collect::<Vec<_>>()
                        .join(" · ")
                } else {
                    String::new()
                };
                let labels = task
                    .labels
                    .iter()
                    .filter_map(|label| project.labels.get(label).map(|label| label.name.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ");
                vec![
                    if task.completed { "✓" } else { "○" }.to_string(),
                    task.name.clone(),
                    deadline
                        .map(|date| format!("due {}", format_date(date)))
                        .unwrap_or_default(),
                    if full {
                        dates
                            .map(|d| {
                                format!(
                                    "{} → {}",
                                    format_date(d.scheduled_start),
                                    format_date(d.scheduled_finish)
                                )
                            })
                            .unwrap_or_default()
                    } else {
                        String::new()
                    },
                    if !task.completed
                        && deadline
                            .zip(dates)
                            .is_some_and(|(due, d)| d.scheduled_finish > due)
                    {
                        "⚠ overdue".to_string()
                    } else {
                        String::new()
                    },
                    if open { "open" } else { "" }.to_string(),
                    if open && full {
                        dates
                            .map(|d| {
                                format!(
                                    "Scheduled {} → {} · earliest {}, latest {}",
                                    format_date(d.scheduled_start),
                                    format_date(d.scheduled_finish),
                                    format_date(d.early_start),
                                    format_date(d.late_start)
                                )
                            })
                            .unwrap_or_else(|| "Not scheduled yet.".to_string())
                    } else {
                        String::new()
                    },
                    if open && full {
                        dates
                            .map(|d| {
                                if d.critical {
                                    "On the critical path — any delay here delays the project."
                                        .to_string()
                                } else {
                                    format!("{} day(s) of slack.", d.total_slack as f64 / 480.0)
                                }
                            })
                            .unwrap_or_default()
                    } else {
                        String::new()
                    },
                    if open && full {
                        dates
                            .filter(|d| d.free_slack > 0 && !d.critical)
                            .map(|d| {
                                format!(
                                    "{} day(s) without disturbing the next task.",
                                    d.free_slack as f64 / 480.0
                                )
                            })
                            .unwrap_or_default()
                    } else {
                        String::new()
                    },
                    heading.clone(),
                    task.priority
                        .map(Priority::label)
                        .unwrap_or_default()
                        .to_string(),
                    labels,
                    dependencies,
                    note_text,
                    if heading.is_empty() {
                        String::new()
                    } else {
                        group_size(group).to_string()
                    },
                ]
            })
            .collect()
    }

    fn sheet_rows(&self, ids: &[TaskId]) -> Vec<Vec<String>> {
        if self.state.view != ViewMode::Sheet {
            return Vec::new();
        }
        let project = self.active_project();
        self.sheet_task_ids(ids)
            .iter()
            .map(|id| {
                let task = &project.tasks[id];
                vec![
                    task.name.clone(),
                    if task.completed { "✓" } else { "○" }.to_string(),
                    task.schedule
                        .as_ref()
                        .and_then(|details| details.deadline)
                        .map(format_date)
                        .unwrap_or_default(),
                    task.priority
                        .map(Priority::label)
                        .unwrap_or_default()
                        .to_string(),
                    task.labels
                        .iter()
                        .filter_map(|label| {
                            project.labels.get(label).map(|label| label.name.as_str())
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                ]
            })
            .collect()
    }

    fn sheet_task_ids(&self, ids: &[TaskId]) -> Vec<TaskId> {
        let project = self.active_project();
        let needle = self.state.sheet_filter_text.to_lowercase();
        let mut ordered: Vec<TaskId> = ids
            .iter()
            .filter(|id| {
                needle.is_empty() || project.tasks[*id].name.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect();
        let column = match self.state.sheet_sort_field.as_str() {
            "Done" => 1,
            "Due" => 2,
            "Priority" => 3,
            _ => 0,
        };
        let sort_value = |id: &TaskId| {
            let task = &project.tasks[id];
            match column {
                1 => task.completed.to_string(),
                2 => task
                    .schedule
                    .as_ref()
                    .and_then(|details| details.deadline)
                    .map(format_date)
                    .unwrap_or_default(),
                3 => task
                    .priority
                    .map(Priority::label)
                    .unwrap_or_default()
                    .to_lowercase(),
                _ => task.name.to_lowercase(),
            }
        };
        ordered.sort_by_key(sort_value);
        if !self.state.sheet_sort_ascending {
            ordered.reverse();
        }
        ordered
    }

    fn timeline_props(
        &self,
        ids: &[TaskId],
        schedule: Option<&task_core::scheduler::ScheduleResult>,
    ) -> (String, Vec<Vec<String>>, Vec<Vec<String>>) {
        if self.state.view != ViewMode::Timeline {
            return (String::new(), Vec::new(), Vec::new());
        }
        let Some(schedule) = schedule else {
            return ("No scheduled work".to_string(), Vec::new(), Vec::new());
        };
        let project = self.active_project();
        let finish = schedule.project_finish.unwrap_or(PROJECT_START);
        let span = PROJECT_START.days_until(finish).max(0) + 1;
        let grid = (0..span)
            .map(|offset| {
                let date = PROJECT_START.add_days(offset);
                vec![
                    format!("{}%", 100.0 / span as f64),
                    if date.weekday() >= 6 { "weekend" } else { "" }.to_string(),
                    if offset == 0 { "today" } else { "" }.to_string(),
                ]
            })
            .collect();
        let rows = ids
            .iter()
            .filter_map(|id| {
                let task = &project.tasks[id];
                schedule.dates.get(id).map(|dates| {
                    let pad = PROJECT_START.days_until(dates.scheduled_start).max(0);
                    let width = dates
                        .scheduled_start
                        .days_until(dates.scheduled_finish)
                        .max(0)
                        + 1;
                    vec![
                        task.name.clone(),
                        format!("{}%", pad as f64 * 100.0 / span as f64),
                        format!("{}%", width as f64 * 100.0 / span as f64),
                        format!(
                            "{} → {}",
                            format_date(dates.scheduled_start),
                            format_date(dates.scheduled_finish)
                        ),
                        if dates.critical { "critical" } else { "" }.to_string(),
                        if task.kind == task_core::TaskKind::Milestone {
                            "milestone"
                        } else {
                            ""
                        }
                        .to_string(),
                        format!("{}%", task.percent_complete),
                        format!(
                            "{}: {} → {}",
                            task.name,
                            format_date(dates.scheduled_start),
                            format_date(dates.scheduled_finish)
                        ),
                    ]
                })
            })
            .collect();
        (
            format!("{} – {}", format_date(PROJECT_START), format_date(finish)),
            grid,
            rows,
        )
    }

    fn calendar_props(
        &self,
        ids: &[TaskId],
        schedule: Option<&task_core::scheduler::ScheduleResult>,
    ) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
        if self.state.view != ViewMode::Calendar {
            return (Vec::new(), Vec::new());
        }
        let month_start = self.state.calendar_month_start;
        let grid_start = month_start.add_days(-i32::from(month_start.weekday() % 7));
        let cells = (0..42)
            .map(|offset| {
                let date = grid_start.add_days(offset);
                let (_, _, day) = date.to_ymd();
                vec![day.to_string(), format_date(date), String::new()]
            })
            .collect::<Vec<_>>();
        let grid_end = grid_start.add_days(41);
        let project = self.active_project();
        let mut events = Vec::new();
        for id in ids {
            let task = &project.tasks[id];
            let range = schedule
                .and_then(|result| result.dates.get(id))
                .map(|dates| {
                    (
                        dates.scheduled_start,
                        dates.scheduled_finish,
                        dates.critical,
                    )
                })
                .or_else(|| {
                    task.schedule
                        .as_ref()
                        .and_then(|details| details.deadline)
                        .map(|date| (date, date, false))
                });
            let Some((start, finish, critical)) = range else {
                continue;
            };
            let from = if start < grid_start {
                grid_start
            } else {
                start
            };
            let to = if finish > grid_end { grid_end } else { finish };
            if from > to {
                continue;
            }
            for offset in 0..=from.days_until(to) {
                events.push(vec![
                    id.to_string(),
                    task.name.clone(),
                    format_date(from.add_days(offset)),
                    if critical { "critical" } else { "" }.to_string(),
                    if task.completed { "done" } else { "" }.to_string(),
                    String::new(),
                ]);
            }
        }
        (cells, events)
    }

    fn dispatch_inner(&mut self, event: &Event) -> Result<AppUpdate, TaskAppError> {
        let normalized_name = normalize_event_name(&event.name);
        match normalized_name.as_str() {
            "newTaskNameChange" => self.state.new_task_name = text_payload(event, "value")?,
            "newTaskDueChange" => self.state.new_task_due = text_payload(event, "value")?,
            "newProjectNameChange" => self.state.new_project_name = text_payload(event, "value")?,
            "newLabelNameChange" => self.state.new_label_name = text_payload(event, "value")?,
            "noteTitleChange" => self.state.note_title = text_payload(event, "value")?,
            "noteBodyChange" => self.state.note_body = text_payload(event, "value")?,
            "noteTaskNameChange" => self.state.note_task_name = text_payload(event, "value")?,
            "sheetFilterChange" => self.state.sheet_filter_text = text_payload(event, "value")?,
            "sheetSortFieldChange" => {
                self.state.sheet_sort_field = text_payload(event, "value")?;
                self.state.sheet_sort_open = false;
            }
            "sheetFormulaChange" => self.state.sheet_edit_content = text_payload(event, "value")?,
            "toggleTheme" => self.state.dark_theme = !self.state.dark_theme,
            "showList" => self.state.view = ViewMode::List,
            "showBoard" => self.state.view = ViewMode::Board,
            "showTimeline"
                if self.active_project().settings.complexity == ProjectComplexity::Full =>
            {
                self.state.view = ViewMode::Timeline
            }
            "showTimeline" => {}
            "showSheet" => self.state.view = ViewMode::Sheet,
            "showCalendar" => self.state.view = ViewMode::Calendar,
            "showNotes" => self.state.view = ViewMode::Notes,
            "sheetToggleSortOpen" => self.state.sheet_sort_open = !self.state.sheet_sort_open,
            "sheetToggleSortDirection" => {
                self.state.sheet_sort_ascending = !self.state.sheet_sort_ascending
            }
            "sheetEditCancel" => self.clear_sheet_edit(),
            "calendarPrev" => {
                self.state.calendar_month_start = shift_month(self.state.calendar_month_start, -1)
            }
            "calendarNext" => {
                self.state.calendar_month_start = shift_month(self.state.calendar_month_start, 1)
            }
            "cancelNote" => self.clear_note(),
            "addTask" => return self.add_task(),
            "toggleTask" => return self.toggle_task(index_payload(event, "index")?),
            "deleteTask" => return self.delete_task(index_payload(event, "index")?),
            "expandTask" => self.expand_task(index_payload(event, "index")?),
            "addProject" => return self.add_project(false),
            "addSubproject" => return self.add_project(true),
            "selectProject" => self.select_project(index_payload(event, "index")?),
            "toggleProjectComplexity" => self.toggle_complexity(),
            "cardDropped" => self.move_card(event)?,
            "sheetNavigate" => {
                self.navigate_sheet(index_payload(event, "row")?, index_payload(event, "col")?)
            }
            "sheetEditCommit" => self.commit_sheet(text_payload(event, "value")?),
            "addLabel" => return self.add_label(),
            "calendarEventDropped" => self.move_calendar_event(event)?,
            "selectNote" => self.select_note(index_payload(event, "index")?),
            "newNote" => self.new_note(),
            "saveNote" => return self.save_note(),
            "deleteNote" => return self.delete_note(),
            _ => return Err(TaskAppError::UnknownEvent(event.name.clone())),
        }
        Ok(self.update())
    }

    fn add_task(&mut self) -> Result<AppUpdate, TaskAppError> {
        let name = self.state.new_task_name.trim().to_string();
        if name.is_empty() {
            return Ok(self.update());
        }
        let previous = self.ordered_task_ids().last().cloned();
        let id = self.next_task_id();
        let due = parse_date(&self.state.new_task_due);
        let project_id = self.state.active_project.clone();
        self.state
            .workspace
            .create_task(&project_id, id.clone(), name.clone(), None)
            .map_err(engine_error)?;
        let project = self.active_project_mut();
        project
            .set_duration(&id, Duration::minutes(480))
            .map_err(engine_error)?;
        if let Some(previous) = previous {
            let link = DependencyLink {
                id: LinkId::from_raw(format!("dependency-{}", id.as_str())),
                predecessor: previous,
                successor: id.clone(),
                kind: DependencyKind::FinishToStart,
                lag: Duration::zero(),
            };
            project.link_dependency(link).map_err(engine_error)?;
        }
        if let Some(due) = due {
            project.set_deadline(&id, Some(due)).map_err(engine_error)?;
        }
        self.state.task_order.push(id);
        self.state.new_task_name.clear();
        self.state.new_task_due.clear();
        Ok(self.announced_update(format!("Added {name}")))
    }

    fn toggle_task(&mut self, index: usize) -> Result<AppUpdate, TaskAppError> {
        let Some(id) = self.task_ids().get(index).cloned() else {
            return Ok(self.update());
        };
        let completed = !self.active_project().tasks[&id].completed;
        self.active_project_mut()
            .set_completed(&id, completed)
            .map_err(engine_error)?;
        Ok(self.announced_update(if completed {
            "Task completed"
        } else {
            "Task reopened"
        }))
    }

    fn delete_task(&mut self, index: usize) -> Result<AppUpdate, TaskAppError> {
        let Some(id) = self.task_ids().get(index).cloned() else {
            return Ok(self.update());
        };
        let name = self.active_project().tasks[&id].name.clone();
        self.active_project_mut()
            .delete_task(&id)
            .map_err(engine_error)?;
        self.state.task_order.retain(|candidate| candidate != &id);
        if self.state.expanded_task.as_ref() == Some(&id) {
            self.state.expanded_task = None;
        }
        Ok(self.announced_update(format!("Deleted {name}")))
    }

    fn expand_task(&mut self, index: usize) {
        if let Some(id) = self.task_ids().get(index).cloned() {
            self.state.expanded_task = if self.state.expanded_task.as_ref() == Some(&id) {
                None
            } else {
                Some(id)
            };
        }
    }

    fn add_project(&mut self, nested: bool) -> Result<AppUpdate, TaskAppError> {
        let name = self.state.new_project_name.trim().to_string();
        if name.is_empty() {
            return Ok(self.update());
        }
        self.state.next_id += 1;
        let id = ProjectId::from_raw(format!("project-{}", self.state.next_id));
        let parent = nested.then(|| self.state.active_project.clone());
        self.state
            .workspace
            .create_project(id.clone(), name.clone(), parent)
            .map_err(engine_error)?;
        self.state.active_project = id;
        self.state.new_project_name.clear();
        self.state.view = ViewMode::List;
        Ok(self.announced_update(format!("Created project {name}")))
    }

    fn select_project(&mut self, index: usize) {
        if let Some(id) = self.project_rows().0.get(index).cloned() {
            self.state.active_project = id;
            self.state.expanded_task = None;
            self.repair();
        }
    }

    fn toggle_complexity(&mut self) {
        let next = if self.active_project().settings.complexity == ProjectComplexity::Full {
            ProjectComplexity::Board
        } else {
            ProjectComplexity::Full
        };
        self.active_project_mut().set_project_complexity(next);
        self.repair();
    }

    fn move_card(&mut self, event: &Event) -> Result<(), TaskAppError> {
        let id = TaskId::from_raw(text_payload(event, "key")?);
        let target = text_payload(event, "targetKey")?;
        if !self.active_project().tasks.contains_key(&id) {
            return Ok(());
        }
        match target.as_str() {
            "done" => {
                self.active_project_mut()
                    .set_completed(&id, true)
                    .map_err(engine_error)?;
            }
            "doing" => {
                let project = self.active_project_mut();
                project.set_completed(&id, false).map_err(engine_error)?;
                project.set_percent_complete(&id, 1).map_err(engine_error)?;
            }
            "next" => {
                let project = self.active_project_mut();
                project.set_completed(&id, false).map_err(engine_error)?;
                project.set_percent_complete(&id, 0).map_err(engine_error)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn navigate_sheet(&mut self, row: usize, col: usize) {
        self.state.sheet_selected_row = row as i64;
        self.state.sheet_selected_col = col as i64;
        if col < 5 {
            let rows = self.sheet_rows(&self.task_ids());
            self.state.sheet_edit_row = row as i64;
            self.state.sheet_edit_col = col as i64;
            self.state.sheet_edit_content = rows
                .get(row)
                .and_then(|values| values.get(col))
                .cloned()
                .unwrap_or_default();
        }
    }

    fn commit_sheet(&mut self, value: String) {
        let row = self.state.sheet_edit_row;
        let col = self.state.sheet_edit_col;
        self.clear_sheet_edit();
        if row < 0 || col < 0 {
            return;
        }
        let ids = self.task_ids();
        let Some(id) = self.sheet_task_ids(&ids).get(row as usize).cloned() else {
            return;
        };
        let project = self.active_project_mut();
        match col {
            0 if !value.trim().is_empty() => {
                let _ = project.rename_task(&id, value.trim());
            }
            1 => {
                let completed = matches!(
                    value.trim().to_lowercase().as_str(),
                    "true" | "yes" | "done" | "✓"
                );
                let _ = project.set_completed(&id, completed);
            }
            2 => {
                let _ = project.set_deadline(&id, parse_date(&value));
            }
            3 => {
                let priority = match value.trim().to_lowercase().as_str() {
                    "low" => Some(Priority::Low),
                    "normal" => Some(Priority::Normal),
                    "high" => Some(Priority::High),
                    "urgent" => Some(Priority::Urgent),
                    _ => None,
                };
                let _ = project.set_priority(&id, priority);
            }
            4 => {
                let names: Vec<String> = value
                    .split(',')
                    .map(|part| part.trim().to_lowercase())
                    .filter(|part| !part.is_empty())
                    .collect();
                let labels: Option<Vec<LabelId>> = names
                    .iter()
                    .map(|name| {
                        project
                            .labels
                            .values()
                            .find(|label| label.name.to_lowercase() == *name)
                            .map(|label| label.id.clone())
                    })
                    .collect();
                if let Some(labels) = labels {
                    let _ = project.set_task_labels(&id, labels);
                }
            }
            _ => {}
        }
    }

    fn clear_sheet_edit(&mut self) {
        self.state.sheet_edit_row = -1;
        self.state.sheet_edit_col = -1;
        self.state.sheet_edit_content.clear();
    }

    fn add_label(&mut self) -> Result<AppUpdate, TaskAppError> {
        let name = self.state.new_label_name.trim().to_string();
        if name.is_empty() {
            return Ok(self.update());
        }
        self.state.next_id += 1;
        let id = LabelId::from_raw(format!("label-{}", self.state.next_id));
        self.active_project_mut().upsert_label(Label {
            id,
            name: name.clone(),
            color: String::new(),
        });
        self.state.new_label_name.clear();
        Ok(self.announced_update(format!("Created label {name}")))
    }

    fn move_calendar_event(&mut self, event: &Event) -> Result<(), TaskAppError> {
        let id = TaskId::from_raw(text_payload(event, "key")?);
        let Some(date) = parse_date(&text_payload(event, "targetKey")?) else {
            return Ok(());
        };
        if self.active_project().tasks.contains_key(&id) {
            self.active_project_mut()
                .set_constraint(&id, Constraint::MustStartOn(date))
                .map_err(engine_error)?;
        }
        Ok(())
    }

    fn new_note(&mut self) {
        self.state.next_id += 1;
        self.state.selected_note_id =
            Some(NoteId::from_raw(format!("note-{}", self.state.next_id)));
        self.state.note_title.clear();
        self.state.note_body.clear();
        self.state.note_task_name.clear();
    }

    fn select_note(&mut self, index: usize) {
        let project = self.active_project();
        let mut notes: Vec<_> = project.notes.values().cloned().collect();
        notes.sort_by_key(|note| note.title.to_lowercase());
        let Some(note) = notes.get(index) else { return };
        let task_name = note
            .attached_task
            .as_ref()
            .and_then(|id| project.tasks.get(id))
            .map(|task| task.name.clone())
            .unwrap_or_default();
        let id = note.id.clone();
        let title = note.title.clone();
        let body = note.body.clone();
        self.state.selected_note_id = Some(id);
        self.state.note_title = title;
        self.state.note_body = body;
        self.state.note_task_name = task_name;
    }

    fn save_note(&mut self) -> Result<AppUpdate, TaskAppError> {
        let Some(id) = self.state.selected_note_id.clone() else {
            return Ok(self.update());
        };
        let attached = if self.state.note_task_name.trim().is_empty() {
            None
        } else {
            self.active_project()
                .tasks
                .values()
                .find(|task| {
                    task.name
                        .eq_ignore_ascii_case(self.state.note_task_name.trim())
                })
                .map(|task| task.id.clone())
        };
        if !self.state.note_task_name.trim().is_empty() && attached.is_none() {
            return Ok(self.update());
        }
        let title = self.state.note_title.clone();
        let body = self.state.note_body.clone();
        self.active_project_mut().upsert_note(Note {
            id,
            title,
            body,
            attached_task: attached,
        });
        Ok(self.announced_update("Note saved"))
    }

    fn delete_note(&mut self) -> Result<AppUpdate, TaskAppError> {
        if let Some(id) = self.state.selected_note_id.clone() {
            self.active_project_mut().delete_note(&id);
            self.clear_note();
            return Ok(self.announced_update("Note deleted"));
        }
        Ok(self.update())
    }

    fn clear_note(&mut self) {
        self.state.selected_note_id = None;
        self.state.note_title.clear();
        self.state.note_body.clear();
        self.state.note_task_name.clear();
    }

    fn next_task_id(&mut self) -> TaskId {
        loop {
            self.state.next_id += 1;
            let id = TaskId::from_raw(format!("task-{}", self.state.next_id));
            if self.state.workspace.project_of_task(&id).is_none() {
                return id;
            }
        }
    }
}

impl MosaicApp for TaskMosaicApp {
    type Error = TaskAppError;

    fn start(&mut self, context: StartContext) -> Result<AppUpdate, Self::Error> {
        self.state.dark_theme = context.color_scheme == ColorScheme::Dark;
        if let Some(snapshot) = context.restored_snapshot {
            self.restore(snapshot)
        } else {
            Ok(self.update())
        }
    }

    fn dispatch(&mut self, event: Event) -> Result<AppUpdate, Self::Error> {
        let before = self.clone();
        match self.dispatch_inner(&event) {
            Ok(update) => Ok(update),
            Err(error) => {
                *self = before;
                Err(error)
            }
        }
    }

    fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error> {
        let bytes = serde_json::to_vec(&self.state).map_err(|_| TaskAppError::InvalidSnapshot)?;
        Ok(Some(Snapshot {
            schema: SNAPSHOT_SCHEMA.to_string(),
            version: SNAPSHOT_VERSION,
            bytes,
        }))
    }

    fn restore(&mut self, snapshot: Snapshot) -> Result<AppUpdate, Self::Error> {
        if snapshot.schema != SNAPSHOT_SCHEMA || snapshot.version != SNAPSHOT_VERSION {
            return Err(TaskAppError::InvalidSnapshot);
        }
        let state: TaskAppState =
            serde_json::from_slice(&snapshot.bytes).map_err(|_| TaskAppError::InvalidSnapshot)?;
        if state.workspace.projects.is_empty() {
            return Err(TaskAppError::InvalidSnapshot);
        }
        self.state = state;
        self.repair();
        Ok(self.update())
    }
}

fn text_payload(event: &Event, field: &'static str) -> Result<String, TaskAppError> {
    event
        .payload
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| TaskAppError::InvalidPayload {
            event: event.name.clone(),
            field,
        })
}

fn normalize_event_name(name: &str) -> String {
    let Some(rest) = name.strip_prefix("on") else {
        return name.to_string();
    };
    let mut chars = rest.chars();
    let Some(first) = chars.next() else {
        return name.to_string();
    };
    if !first.is_ascii_uppercase() {
        return name.to_string();
    }
    first.to_ascii_lowercase().to_string() + chars.as_str()
}

fn index_payload(event: &Event, field: &'static str) -> Result<usize, TaskAppError> {
    event
        .payload
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| TaskAppError::InvalidPayload {
            event: event.name.clone(),
            field,
        })
}

fn engine_error(error: task_core::ops::OpError) -> TaskAppError {
    TaskAppError::Engine(format!("{error:?}"))
}

fn parse_date(value: &str) -> Option<Date> {
    let mut parts = value.trim().split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Date::from_ymd(year, month, day)
}

fn format_date(date: Date) -> String {
    let (year, month, day) = date.to_ymd();
    format!("{year:04}-{month:02}-{day:02}")
}

fn shift_month(date: Date, delta: i32) -> Date {
    let (year, month, _) = date.to_ymd();
    let months = year * 12 + i32::from(month) - 1 + delta;
    Date::from_ymd(months.div_euclid(12), (months.rem_euclid(12) + 1) as u32, 1)
        .expect("normalized month is valid")
}

fn month_label(date: Date) -> String {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let (year, month, _) = date.to_ymd();
    format!("{} {year}", MONTHS[usize::from(month - 1)])
}

fn task_group(completed: bool, percent: u8) -> &'static str {
    if completed {
        "done"
    } else if percent > 0 {
        "in progress"
    } else {
        "up next"
    }
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

fn dependency_label(kind: DependencyKind) -> &'static str {
    match kind {
        DependencyKind::FinishToStart => "FS",
        DependencyKind::StartToStart => "SS",
        DependencyKind::FinishToFinish => "FF",
        DependencyKind::StartToFinish => "SF",
    }
}

mosaic_app_capi::export_mosaic_app!(TaskMosaicApp, TaskMosaicApp::default());

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_app_runtime::{MosaicRuntime, Platform, PROTOCOL_VERSION};

    const REQUIRED_PROPS: &[&str] = &[
        "app-title",
        "new-task-name",
        "new-task-due",
        "project-rows",
        "new-project-name",
        "summary",
        "status-label",
        "status-warn",
        "ring-gradient",
        "ring-percent",
        "ring-percent-value",
        "theme-is-dark",
        "complexity-label",
        "allow-timeline",
        "timeline-mode",
        "timeline-grid",
        "board-mode",
        "board-columns",
        "board-cards",
        "timeline-scale",
        "timeline-rows",
        "sheet-mode",
        "sheet-viewport-rows",
        "sheet-column-headers",
        "sheet-column-widths",
        "sheet-selected-row",
        "sheet-selected-col",
        "sheet-edit-row",
        "sheet-edit-col",
        "sheet-edit-content",
        "sheet-filter-text",
        "sheet-sort-field",
        "sheet-sort-options",
        "sheet-sort-open",
        "sheet-sort-ascending",
        "new-label-name",
        "calendar-mode",
        "calendar-title",
        "calendar-cells",
        "calendar-events",
        "notes-mode",
        "notes-title",
        "note-rows",
        "selected-note-id",
        "note-title-value",
        "note-body-value",
        "note-task-value",
        "task-rows",
    ];

    fn context() -> StartContext {
        StartContext::new("en-US", Platform::Linux)
    }

    fn event(sequence: u64, name: &str, payload: Value) -> Event {
        Event::new(sequence, format!("on{}", capitalize(name)), payload)
    }

    #[test]
    fn start_returns_every_required_task_app_prop() {
        let mut runtime = MosaicRuntime::new(TaskMosaicApp::default());
        let update = runtime.start(context()).unwrap();
        let object = update.props.as_object().unwrap();
        for key in REQUIRED_PROPS {
            assert!(object.contains_key(*key), "missing required prop {key}");
        }
    }

    #[test]
    fn core_task_flow_is_engine_backed() {
        let mut runtime = MosaicRuntime::new(TaskMosaicApp::default());
        runtime.start(context()).unwrap();
        runtime
            .dispatch(event(
                1,
                "newTaskNameChange",
                json!({"value":"Ship native TaskApp"}),
            ))
            .unwrap();
        runtime
            .dispatch(event(2, "newTaskDueChange", json!({"value":"2026-01-09"})))
            .unwrap();
        let added = runtime.dispatch(event(3, "addTask", json!({}))).unwrap();
        assert_eq!(added.props["task-rows"][0][1], "Ship native TaskApp");
        assert_eq!(added.props["task-rows"][0][2], "due 2026-01-09");
        let completed = runtime
            .dispatch(event(4, "toggleTask", json!({"index":0})))
            .unwrap();
        assert_eq!(completed.props["task-rows"][0][0], "✓");
        assert_eq!(completed.props["ring-percent"], "100%");
        // #12028 item 2: the same percent as typed data, not just the
        // pre-formatted caption string — this is what a future native
        // rendering of the progress ring would consume.
        assert_eq!(completed.props["ring-percent-value"], 100);
        let deleted = runtime
            .dispatch(event(5, "deleteTask", json!({"index":0})))
            .unwrap();
        assert_eq!(deleted.props["task-rows"], json!([]));
    }

    #[test]
    fn every_declared_event_is_accepted() {
        let mut app = TaskMosaicApp::default();
        app.start(context()).unwrap();
        let cases = [
            ("newTaskNameChange", json!({"value":"Task"})),
            ("newTaskDueChange", json!({"value":""})),
            ("newProjectNameChange", json!({"value":"Project"})),
            ("addProject", json!({})),
            ("addSubproject", json!({})),
            ("selectProject", json!({"index":0})),
            ("toggleProjectComplexity", json!({})),
            ("showList", json!({})),
            ("showBoard", json!({})),
            ("showTimeline", json!({})),
            ("showSheet", json!({})),
            ("showCalendar", json!({})),
            ("showNotes", json!({})),
            (
                "cardDropped",
                json!({"key":"missing","kind":"task","targetKey":"next","position":"inside"}),
            ),
            ("sheetNavigate", json!({"row":0,"col":0})),
            ("sheetFormulaChange", json!({"value":"Task"})),
            ("sheetEditCommit", json!({"value":"Task"})),
            ("sheetEditCancel", json!({})),
            ("sheetFilterChange", json!({"value":""})),
            ("sheetSortFieldChange", json!({"value":"Name"})),
            ("sheetToggleSortOpen", json!({})),
            ("sheetToggleSortDirection", json!({})),
            ("newLabelNameChange", json!({"value":"Urgent"})),
            ("addLabel", json!({})),
            ("calendarPrev", json!({})),
            ("calendarNext", json!({})),
            (
                "calendarEventDropped",
                json!({"key":"missing","kind":"task","targetKey":"2026-01-06","position":"inside"}),
            ),
            ("selectNote", json!({"index":0})),
            ("newNote", json!({})),
            ("noteTitleChange", json!({"value":"Note"})),
            ("noteBodyChange", json!({"value":"Body"})),
            ("noteTaskNameChange", json!({"value":""})),
            ("saveNote", json!({})),
            ("deleteNote", json!({})),
            ("cancelNote", json!({})),
            ("expandTask", json!({"index":0})),
            ("addTask", json!({})),
            ("toggleTask", json!({"index":0})),
            ("deleteTask", json!({"index":0})),
            ("toggleTheme", json!({})),
        ];
        for (name, payload) in cases {
            app.dispatch(event(1, name, payload))
                .unwrap_or_else(|error| panic!("{name}: {error}"));
        }
    }

    #[test]
    fn snapshot_round_trips_engine_and_presentation_state() {
        let mut app = TaskMosaicApp::default();
        app.start(context()).unwrap();
        app.dispatch(event(1, "newTaskNameChange", json!({"value":"Persist me"})))
            .unwrap();
        app.dispatch(event(2, "addTask", json!({}))).unwrap();
        app.dispatch(event(3, "showBoard", json!({}))).unwrap();
        let snapshot = app.snapshot().unwrap().unwrap();
        let mut restored = TaskMosaicApp::default();
        let update = restored.restore(snapshot).unwrap();
        assert_eq!(update.props["board-mode"], "board");
        assert_eq!(update.props["board-cards"][0][0], "Persist me");
    }

    #[test]
    fn sorted_sheet_edits_the_row_that_was_rendered() {
        let mut app = TaskMosaicApp::default();
        app.start(context()).unwrap();
        for name in ["Zulu", "Alpha"] {
            app.dispatch(event(1, "newTaskNameChange", json!({"value": name})))
                .unwrap();
            app.dispatch(event(1, "addTask", json!({}))).unwrap();
        }
        app.dispatch(event(1, "showSheet", json!({}))).unwrap();
        app.dispatch(event(1, "sheetSortFieldChange", json!({"value":"Name"})))
            .unwrap();
        let sorted = app.update();
        assert_eq!(sorted.props["sheet-viewport-rows"][0][0], "Alpha");

        app.dispatch(event(1, "sheetNavigate", json!({"row":0,"col":0})))
            .unwrap();
        let edited = app
            .dispatch(event(1, "sheetEditCommit", json!({"value":"Beta"})))
            .unwrap();
        let names: BTreeSet<&str> = edited.props["task-rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row[1].as_str().unwrap())
            .collect();
        assert_eq!(names, BTreeSet::from(["Beta", "Zulu"]));
    }

    #[test]
    fn failed_event_does_not_mutate_state() {
        let mut app = TaskMosaicApp::default();
        app.start(context()).unwrap();
        let before = app.snapshot().unwrap();
        assert!(app
            .dispatch(event(1, "newTaskNameChange", json!({"value":7})))
            .is_err());
        assert_eq!(app.snapshot().unwrap(), before);
    }

    #[test]
    fn protocol_constant_is_current() {
        assert_eq!(PROTOCOL_VERSION, 1);
    }
}
