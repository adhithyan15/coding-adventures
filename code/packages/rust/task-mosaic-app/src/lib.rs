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
use task_core::checklists::{ChecklistOutlineRow, ChecklistSummary, MAX_CHECKLIST_ITEMS};
use task_core::{
    ChecklistId, Constraint, Date, Decision, DependencyKind, DependencyLink, Duration, Label,
    LabelId, LinkId, Note, NoteId, Priority, ProjectComplexity, ProjectId, RunStatus, TaskId,
    Workspace, WorkspaceId,
};

const SNAPSHOT_SCHEMA: &str = "task-mosaic-app/state";
const SNAPSHOT_VERSION: u32 = 1;
const PROJECT_START: Date = Date(20_458); // 2026-01-05, a Monday.

/// The longest a Checklists composer (a checklist's or an item's name) may
/// grow. Longer input is refused as it is typed, so no draft can grow the
/// state file without bound (task-app-checklists-view-v1.md, "Bounds").
const MAX_CHECKLIST_TEXT_CHARS: usize = 512;

/// The deepest indent a checklist row draws. Depth is unbounded in a restored
/// snapshot (a chain of N items would make N² bytes of indent per render);
/// past this, rows keep the deepest indent rather than growing without bound.
const MAX_CHECKLIST_INDENT_DEPTH: usize = 16;

fn checklist_indent(depth: u32) -> String {
    "  ".repeat((depth as usize).min(MAX_CHECKLIST_INDENT_DEPTH))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ViewMode {
    List,
    Board,
    Timeline,
    Sheet,
    Calendar,
    Notes,
    /// C3 (#14018). Appended: declaration order is snapshot history.
    Checklists,
}

impl ViewMode {
    /// The view switcher's order (#14016). Not the declaration order above,
    /// which is snapshot history. Timeline is last so that leaving it out of
    /// a Board-tier project never shifts another view's index.
    const SWITCHER_ORDER: [Self; 7] = [
        Self::List,
        Self::Board,
        Self::Sheet,
        Self::Calendar,
        Self::Notes,
        Self::Checklists,
        Self::Timeline,
    ];

    fn switcher_label(self) -> &'static str {
        match self {
            Self::List => "List",
            Self::Board => "Board",
            Self::Sheet => "Sheet",
            Self::Calendar => "Calendar",
            Self::Notes => "Notes",
            Self::Checklists => "Checklists",
            Self::Timeline => "Timeline",
        }
    }

    /// The views the switcher offers: all seven for a Full project, and all
    /// but Timeline otherwise. Checklists is offered at both tiers.
    fn switcher_views(full: bool) -> &'static [Self] {
        if full {
            &Self::SWITCHER_ORDER
        } else {
            &Self::SWITCHER_ORDER[..6]
        }
    }
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
    // Composer errors are transient presentation state. They should guide the
    // current correction, not survive a process restart in the durable snapshot.
    #[serde(skip)]
    new_task_name_error: String,
    #[serde(skip)]
    new_task_due_error: String,
    #[serde(skip)]
    new_task_name_focus: String,
    #[serde(skip)]
    new_task_due_focus: String,
    // List editing is transient: only committed values belong in the engine snapshot.
    #[serde(skip)]
    editing_task: Option<TaskId>,
    #[serde(skip)]
    edit_task_name: String,
    #[serde(skip)]
    edit_task_due: String,
    #[serde(skip)]
    edit_task_name_error: String,
    #[serde(skip)]
    edit_task_due_error: String,
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
    // Checklists (C3). Defaulted, so every earlier snapshot still restores.
    #[serde(default)]
    selected_checklist: Option<ChecklistId>,
    #[serde(default)]
    new_checklist_name: String,
    #[serde(default)]
    new_checklist_item: String,
    /// The template outline item selected for editing (C3c).
    #[serde(default)]
    selected_outline_item: Option<TaskId>,
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
            new_task_name_error: String::new(),
            new_task_due_error: String::new(),
            new_task_name_focus: String::new(),
            new_task_due_focus: String::new(),
            editing_task: None,
            edit_task_name: String::new(),
            edit_task_due: String::new(),
            edit_task_name_error: String::new(),
            edit_task_due_error: String::new(),
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
            selected_checklist: None,
            new_checklist_name: String::new(),
            new_checklist_item: String::new(),
            selected_outline_item: None,
        }
    }
}

/// Concrete standard-ABI TaskApp implementation.
#[derive(Debug, Clone)]
pub struct TaskMosaicApp {
    state: TaskAppState,
    /// Milliseconds since the Unix epoch. Checklist ops stamp runs with it;
    /// tests replace it. Not part of the snapshot.
    clock: fn() -> u64,
}

impl Default for TaskMosaicApp {
    fn default() -> Self {
        Self {
            state: TaskAppState::default(),
            clock: system_clock,
        }
    }
}

fn system_clock() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
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
    /// An app whose clock is `clock` instead of the system's: for tests and
    /// for hosts that replay a recorded session.
    pub fn with_clock(clock: fn() -> u64) -> Self {
        Self {
            clock,
            ..Self::default()
        }
    }

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
        if let Some(id) = &self.state.selected_checklist {
            if !self.active_project().checklists.contains_key(id) {
                self.state.selected_checklist = None;
                self.state.selected_outline_item = None;
            }
        }
        if self.state.selected_outline_item.is_some() && self.selected_outline_row().is_none() {
            self.state.selected_outline_item = None;
        }
        for draft in [
            &mut self.state.new_checklist_name,
            &mut self.state.new_checklist_item,
        ] {
            if draft.chars().count() > MAX_CHECKLIST_TEXT_CHARS {
                draft.clear();
            }
        }
    }

    /// The project's tasks in list order. Checklist templates' and runs'
    /// items are not tasks on the list: they belong to the Checklists view,
    /// exactly as task-core's own views leave them out
    /// (task-app-checklists-v1.md).
    fn ordered_task_ids(&self) -> Vec<TaskId> {
        let project = self.active_project();
        let owned = project.checklist_owned();
        let mut ids: Vec<TaskId> = self
            .state
            .task_order
            .iter()
            .filter(|id| project.tasks.contains_key(*id) && !owned.contains(*id))
            .cloned()
            .collect();
        let mut seen: BTreeSet<TaskId> = ids.iter().cloned().collect();
        for id in project.tasks.keys() {
            if !owned.contains(id) && seen.insert(id.clone()) {
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
        let board_status = |id: &TaskId| {
            let task = &project.tasks[id];
            if task.completed {
                "done"
            } else if task.percent_complete > 0 {
                "doing"
            } else {
                "next"
            }
        };
        let board_columns = [
            ("Up next", "next"),
            ("In progress", "doing"),
            ("Done", "done"),
        ]
        .into_iter()
        .map(|(title, key)| {
            let count = ids.iter().filter(|id| board_status(id) == key).count();
            let accent = match (self.state.dark_theme, key) {
                (true, "next") => "#867c70",
                (true, "doing") => "#eaa63f",
                (true, "done") => "#6fb489",
                (false, "next") => "#9b9289",
                (false, "doing") => "#e0942a",
                (false, "done") => "#4f8e6a",
                _ => unreachable!("board columns use a fixed status vocabulary"),
            };
            vec![
                title.to_string(),
                key.to_string(),
                count.to_string(),
                accent.to_string(),
            ]
        })
        .collect::<Vec<_>>();
        let board_cards = if self.state.view == ViewMode::Board {
            ids.iter()
                .map(|id| {
                    let task = &project.tasks[id];
                    vec![
                        task.name.clone(),
                        board_status(id).to_string(),
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

        let checklists = self.checklist_props();

        let views = ViewMode::switcher_views(full);
        // One label per view for the toolkit SegmentedControl. The control
        // reports which view is selected through the kernel's selected state
        // (UI86), so no name carries ", selected" any more (#15420).
        let nav_options = views
            .iter()
            .map(|&view| view.switcher_label())
            .collect::<Vec<_>>();
        let nav_selected_index = views
            .iter()
            .position(|&view| view == self.state.view)
            .unwrap_or(0);

        json!({
            "app-title": "Tasks — auto-scheduled",
            "new-task-name": self.state.new_task_name,
            "new-task-due": self.state.new_task_due,
            "new-task-name-error": self.state.new_task_name_error,
            "new-task-due-error": self.state.new_task_due_error,
            "new-task-name-focus": self.state.new_task_name_focus,
            "new-task-due-focus": self.state.new_task_due_focus,
            "edit-task-name": self.state.edit_task_name,
            "edit-task-due": self.state.edit_task_due,
            "edit-task-name-error": self.state.edit_task_name_error,
            "edit-task-due-error": self.state.edit_task_due_error,
            "empty-list": if ids.is_empty() { "empty" } else { "" },
            "new-project-name": self.state.new_project_name,
            "project-rows": project_rows,
            "summary": format!("{} task(s) · {} done · projected finish {}", ids.len(), done_count,
                schedule.as_ref().and_then(|result| result.project_finish).map(format_date).unwrap_or_else(|| "—".to_string())),
            "status-label": if overdue_count > 0 { format!("{overdue_count} overdue") } else { "On track".to_string() },
            "status-warn": if overdue_count > 0 { "warn" } else { "" },
            "storage-status": "Saved locally on this device",
            "storage-location": local_data_guidance(),
            "storage-warning": "",
            "ring-gradient": "",
            "ring-percent": format!("{percent}%"),
            "ring-percent-value": percent,
            "theme-is-dark": if self.state.dark_theme { "dark" } else { "" },
            "complexity-label": if full { "Full CPM" } else { "Board" },
            "allow-timeline": if full { "full" } else { "" },
            "nav-options": nav_options,
            "nav-selected-index": nav_selected_index,
            "timeline-mode": if self.state.view == ViewMode::Timeline { "timeline" } else { "" },
            "timeline-grid": timeline_grid,
            "timeline-scale": timeline_scale,
            "timeline-rows": timeline_rows,
            "board-mode": if self.state.view == ViewMode::Board { "board" } else { "" },
            "board-columns": board_columns,
            "board-cards": board_cards,
            "sheet-mode": if self.state.view == ViewMode::Sheet { "sheet" } else { "" },
            "sheet-viewport-rows": sheet_rows,
            "sheet-column-headers": ["Name", "Done", "Due", "Priority", "Labels"],
            // #15131 -- PIXEL widths, which is what the slot means.
            //
            // These were `[3, 1, 2, 2, 2]`, plainly intended as relative
            // proportions, but `Grid.mil` declares `column-widths` as
            // "per-column pixel widths" and every backend threads them
            // straight into a width. Three of the sheet's cells therefore
            // measured ZERO WIDTH at 1280, 900 and 700 alike -- a 1px
            // column has no room for its text. The same proportions at a
            // 80px unit, matching the grid's own 72px minimum cell.
            "sheet-column-widths": [240, 80, 160, 160, 160],
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
            "checklists-mode": if self.state.view == ViewMode::Checklists { "checklists" } else { "" },
            "checklists-title": "Checklists",
            "checklist-library-rows": checklists.library_rows,
            "checklist-library-empty": project.checklists.is_empty(),
            "selected-checklist-key": self.state.selected_checklist.as_ref().map(ToString::to_string).unwrap_or_default(),
            "new-checklist-name": self.state.new_checklist_name,
            "checklist-template-mode": checklists.template_mode,
            "checklist-run-mode": checklists.run_mode,
            "checklist-outline-rows": checklists.outline_rows,
            "selected-outline-key": self.state.selected_outline_item.as_ref().map(ToString::to_string).unwrap_or_default(),
            "outline-item-selected": checklists.outline_item_selected,
            "outline-question-selected": checklists.outline_question_selected,
            "outline-toggle-label": checklists.outline_toggle_label,
            "new-checklist-item": self.state.new_checklist_item,
            "checklist-run-title": checklists.run_title,
            "checklist-run-progress": checklists.run_progress,
            "checklist-run-rows": checklists.run_rows,
            "checklist-complete-label": checklists.complete_label,
            "checklist-abandon-label": checklists.abandon_label,
            "task-rows": task_rows,
        })
    }

    // ── Checklists (C3, task-app-checklists-view-v1.md) ─────────────────────
    //
    // The view is a library of the project's templates and runs beside the
    // selection: a template's outline with its composer, or a run through
    // ChecklistRun (C2). All the rows are positional lists of text, like
    // every other view's, and use only truthy markers.

    /// The library in display order: templates by name, then runs newest
    /// first. The library's rows, and `onSelectChecklist`'s index, both come
    /// from this one list, so a click can only ever name a row just drawn.
    fn checklist_library(&self) -> Vec<ChecklistSummary> {
        let mut all = self.active_project().checklists();
        all.sort_by(|a, b| match (&a.status, &b.status) {
            (None, None) => a
                .name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.id.cmp(&b.id)),
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(_), Some(_)) => b
                .created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.cmp(&a.id)),
        });
        all
    }

    fn checklist_props(&self) -> ChecklistProps {
        let mut props = ChecklistProps::default();
        if self.state.view != ViewMode::Checklists {
            return props;
        }
        let project = self.active_project();
        let mut last_heading = "";
        props.library_rows = self
            .checklist_library()
            .iter()
            .map(|summary| {
                let group = if summary.status.is_none() {
                    "Templates"
                } else {
                    "Runs"
                };
                let heading = if group == last_heading {
                    String::new()
                } else {
                    last_heading = group;
                    group.to_string()
                };
                let subtitle = match (summary.status, summary.progress) {
                    (Some(status), Some(progress)) => progress_label(status, &progress),
                    _ => item_count_label(summary.items),
                };
                let badge = match summary.status {
                    Some(RunStatus::Completed) => "✓",
                    Some(RunStatus::Abandoned) => "✗",
                    _ => "",
                };
                vec![
                    summary.id.to_string(),
                    heading,
                    display_name(&summary.name),
                    subtitle,
                    String::new(),
                    badge.to_string(),
                ]
            })
            .collect();

        let Some(id) = &self.state.selected_checklist else {
            return props;
        };
        if let Some(run) = project.checklist_run(id) {
            let in_progress = run.status == RunStatus::InProgress;
            props.run_mode = true;
            props.run_title = display_name(&run.name);
            props.run_progress = progress_label(run.status, &run.progress);
            props.run_rows = run
                .rows
                .iter()
                .map(|row| {
                    let marker = |on: bool| if on { "1" } else { "" }.to_string();
                    vec![
                        row.task.to_string(),
                        checklist_indent(row.depth),
                        row.name.clone(),
                        marker(row.is_decision),
                        marker(if row.is_decision {
                            row.answered == Some(true)
                        } else {
                            row.completed
                        }),
                        marker(row.is_decision && row.answered == Some(false)),
                    ]
                })
                .collect();
            if in_progress && run.progress.complete {
                props.complete_label = "Complete".to_string();
            }
            if in_progress {
                props.abandon_label = "Abandon".to_string();
            }
        } else if let Some(outline) = project.checklist_outline(id) {
            props.template_mode = true;
            props.outline_rows = outline
                .iter()
                .map(|row| {
                    vec![
                        row.task.to_string(),
                        checklist_indent(row.depth),
                        row.name.clone(),
                        if row.is_decision { "1" } else { "" }.to_string(),
                        match row.branch {
                            Some(true) => "Yes",
                            Some(false) => "No",
                            None => "",
                        }
                        .to_string(),
                        if self.state.selected_outline_item.as_ref() == Some(&row.task) {
                            "1"
                        } else {
                            ""
                        }
                        .to_string(),
                    ]
                })
                .collect();
            if let Some(selected) = self.selected_outline_row() {
                props.outline_item_selected = true;
                props.outline_question_selected = selected.is_decision;
                props.outline_toggle_label = if selected.is_decision {
                    "Make it a step"
                } else {
                    "Make it a question"
                }
                .to_string();
            }
        }
        props
    }

    fn select_checklist(&mut self, index: usize) {
        if let Some(summary) = self.checklist_library().get(index) {
            self.state.selected_checklist = Some(summary.id.clone());
            self.state.selected_outline_item = None;
            self.state.new_checklist_item.clear();
        }
    }

    /// The selection, if it is a template (`want_run == false`) or a run.
    fn selected_checklist_of_kind(
        &self,
        event: &Event,
        want_run: bool,
    ) -> Result<ChecklistId, TaskAppError> {
        self.state
            .selected_checklist
            .as_ref()
            .and_then(|id| self.active_project().checklists.get(id))
            .filter(|checklist| checklist.run.is_some() == want_run)
            .map(|checklist| checklist.id.clone())
            .ok_or_else(|| TaskAppError::InvalidPayload {
                event: event.name.clone(),
                field: "selection",
            })
    }

    /// Advance the shared id counter. Checked: a counter at `u64::MAX` (only a
    /// tampered snapshot gets there) fails the event instead of wrapping to
    /// ids that already exist, or panicking in a build with overflow checks.
    fn bump_next_id(&mut self) -> Result<u64, TaskAppError> {
        self.state.next_id = self
            .state
            .next_id
            .checked_add(1)
            .ok_or_else(|| TaskAppError::Engine("id counter exhausted".to_string()))?;
        Ok(self.state.next_id)
    }

    /// The next free id `{prefix}-{n}` from the shared counter, skipping any
    /// id (or derived root id) already taken. Checked, so a tampered counter
    /// fails rather than wraps.
    fn next_checklist_id(&mut self, prefix: &str) -> Result<ChecklistId, TaskAppError> {
        loop {
            let n = self.bump_next_id()?;
            let id = ChecklistId::from_raw(format!("{prefix}-{n}"));
            let root = TaskId::from_raw(format!("{id}/root"));
            let taken = self
                .state
                .workspace
                .projects
                .values()
                .any(|project| project.checklists.contains_key(&id))
                || self.state.workspace.project_of_task(&root).is_some();
            if !taken {
                return Ok(id);
            }
        }
    }

    fn create_checklist(&mut self) -> Result<AppUpdate, TaskAppError> {
        let name = self.state.new_checklist_name.trim().to_string();
        if name.is_empty() {
            return Ok(self.update());
        }
        let id = self.next_checklist_id("checklist")?;
        let root = TaskId::from_raw(format!("{id}/root"));
        let now = (self.clock)();
        self.active_project_mut()
            .create_checklist_template(id.clone(), root, name.clone(), "", now)
            .map_err(engine_error)?;
        self.state.selected_checklist = Some(id);
        self.state.selected_outline_item = None;
        self.state.new_checklist_name.clear();
        self.state.new_checklist_item.clear();
        Ok(self.announced_update(format!("Created checklist {name}")))
    }

    fn add_checklist_item(&mut self, event: &Event) -> Result<AppUpdate, TaskAppError> {
        let template = self.selected_checklist_of_kind(event, false)?;
        let root = self.active_project().checklists[&template].root.clone();
        self.add_item_under(template, root, None)
    }

    /// C3c: the composer's text as a new item in the selected question's Yes
    /// (`true`) or No branch.
    fn add_checklist_branch_item(
        &mut self,
        event: &Event,
        branch: bool,
    ) -> Result<AppUpdate, TaskAppError> {
        let template = self.selected_checklist_of_kind(event, false)?;
        let question = self
            .selected_outline_row()
            .filter(|row| row.is_decision)
            .ok_or_else(|| TaskAppError::InvalidPayload {
                event: event.name.clone(),
                field: "selection",
            })?;
        self.add_item_under(template, question.task, Some(branch))
    }

    /// Add the composer's text as a new item under `parent`: at the next
    /// sibling order, and, for `Some(branch)`, listed in that branch of the
    /// question `parent`. `dispatch` rolls every step back if one fails.
    fn add_item_under(
        &mut self,
        template: ChecklistId,
        parent: TaskId,
        branch: Option<bool>,
    ) -> Result<AppUpdate, TaskAppError> {
        let name = self.state.new_checklist_item.trim().to_string();
        if name.is_empty() {
            return Ok(self.update());
        }
        let project = self.active_project();
        let items = project
            .checklist_outline(&template)
            .map_or(0, |rows| rows.len());
        // After this item the subtree is `items + 2` tasks (the root, the
        // existing items, this one); instantiate refuses more than the cap.
        if items + 2 > MAX_CHECKLIST_ITEMS {
            return Err(TaskAppError::Engine("checklist is full".to_string()));
        }
        // Siblings sort by (order, id), and minted ids do not sort by number
        // (`task-10` < `task-9`), so each new item takes the next order.
        let order = project
            .tasks
            .values()
            .filter(|task| task.parent.as_ref() == Some(&parent))
            .map(|task| task.order)
            .max()
            .map_or(0, |max| max.saturating_add(1));
        let decision = project.tasks.get(&parent).and_then(|t| t.decision.clone());
        let id = self.next_task_id()?;
        let project = self.active_project_mut();
        project
            .create_task(id.clone(), name.clone(), Some(parent.clone()))
            .map_err(engine_error)?;
        project.set_order(&id, order).map_err(engine_error)?;
        if let Some(yes) = branch {
            let mut decision = decision.ok_or_else(|| {
                TaskAppError::Engine("the selected item is not a question".to_string())
            })?;
            if yes {
                decision.yes_children.push(id.clone());
            } else {
                decision.no_children.push(id.clone());
            }
            project
                .set_decision(&parent, Some(decision))
                .map_err(engine_error)?;
        }
        self.state.new_checklist_item.clear();
        Ok(self.announced_update(format!("Added {name}")))
    }

    /// The selected template's outline row for the selected item, if both
    /// still exist.
    fn selected_outline_row(&self) -> Option<ChecklistOutlineRow> {
        let item = self.state.selected_outline_item.as_ref()?;
        let template = self.state.selected_checklist.as_ref()?;
        let project = self.active_project();
        if project.checklists.get(template)?.run.is_some() {
            return None;
        }
        project
            .checklist_outline(template)?
            .into_iter()
            .find(|row| &row.task == item)
    }

    fn select_outline_item(&mut self, event: &Event) -> Result<(), TaskAppError> {
        let template = self.selected_checklist_of_kind(event, false)?;
        let index = index_payload(event, "index")?;
        let row = self
            .active_project()
            .checklist_outline(&template)
            .and_then(|rows| rows.into_iter().nth(index))
            .ok_or_else(|| TaskAppError::InvalidPayload {
                event: event.name.clone(),
                field: "index",
            })?;
        // Clicking the selected item again clears the selection.
        self.state.selected_outline_item =
            if self.state.selected_outline_item.as_ref() == Some(&row.task) {
                None
            } else {
                Some(row.task)
            };
        Ok(())
    }

    /// A step becomes a question (its sub-items, if any, its Yes branch);
    /// a question becomes a step again (its branch items, sub-items).
    fn toggle_outline_question(&mut self, event: &Event) -> Result<AppUpdate, TaskAppError> {
        self.selected_checklist_of_kind(event, false)?;
        let row = self
            .selected_outline_row()
            .ok_or_else(|| TaskAppError::InvalidPayload {
                event: event.name.clone(),
                field: "selection",
            })?;
        let project = self.active_project_mut();
        if row.is_decision {
            project
                .set_decision(&row.task, None)
                .map_err(engine_error)?;
            return Ok(self.announced_update(format!("{} is a step", row.name)));
        }
        let mut children: Vec<&task_core::Task> = project
            .tasks
            .values()
            .filter(|task| task.parent.as_ref() == Some(&row.task))
            .collect();
        children.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.id.cmp(&b.id)));
        let yes_children = children.into_iter().map(|task| task.id.clone()).collect();
        project
            .set_decision(
                &row.task,
                Some(Decision {
                    question: row.name.clone(),
                    answer: None,
                    yes_children,
                    no_children: Vec::new(),
                }),
            )
            .map_err(engine_error)?;
        Ok(self.announced_update(format!("{} is a question", row.name)))
    }

    /// Delete the selected item and everything under it, deepest first:
    /// `delete_task` only reparents, and a question's branch items reparented
    /// onto its parent would break the decision invariant.
    fn delete_outline_item(&mut self, event: &Event) -> Result<AppUpdate, TaskAppError> {
        let template = self.selected_checklist_of_kind(event, false)?;
        let selected = self
            .selected_outline_row()
            .ok_or_else(|| TaskAppError::InvalidPayload {
                event: event.name.clone(),
                field: "selection",
            })?;
        let outline = self
            .active_project()
            .checklist_outline(&template)
            .unwrap_or_default();
        // Outline rows are depth-first preorder: the subtree is the selected
        // row and the rows after it that sit deeper.
        let start = outline
            .iter()
            .position(|row| row.task == selected.task)
            .unwrap_or(outline.len());
        let subtree: Vec<TaskId> = outline[start..]
            .iter()
            .enumerate()
            .take_while(|(i, row)| *i == 0 || row.depth > selected.depth)
            .map(|(_, row)| row.task.clone())
            .collect();
        let project = self.active_project_mut();
        for id in subtree.iter().rev() {
            project.delete_task(id).map_err(engine_error)?;
        }
        self.state.selected_outline_item = None;
        Ok(self.announced_update(format!("Deleted {}", selected.name)))
    }

    fn start_checklist_run(&mut self, event: &Event) -> Result<AppUpdate, TaskAppError> {
        let template = self.selected_checklist_of_kind(event, false)?;
        let run = self.next_checklist_id("run")?;
        let now = (self.clock)();
        self.active_project_mut()
            .instantiate_checklist(&template, run.clone(), now)
            .map_err(engine_error)?;
        self.state.selected_checklist = Some(run);
        self.state.selected_outline_item = None;
        Ok(self.announced_update("Run started"))
    }

    /// The visible row at `index` of the selected run, as just rendered.
    fn checklist_run_row(
        &self,
        event: &Event,
    ) -> Result<task_core::projections::ChecklistRow, TaskAppError> {
        let run = self.selected_checklist_of_kind(event, true)?;
        let index = index_payload(event, "index")?;
        self.active_project()
            .checklist_run(&run)
            .and_then(|view| view.rows.into_iter().nth(index))
            .ok_or_else(|| TaskAppError::InvalidPayload {
                event: event.name.clone(),
                field: "index",
            })
    }

    fn toggle_checklist_item(&mut self, event: &Event) -> Result<(), TaskAppError> {
        let row = self.checklist_run_row(event)?;
        if row.is_decision {
            return Err(TaskAppError::InvalidPayload {
                event: event.name.clone(),
                field: "index",
            });
        }
        self.active_project_mut()
            .set_completed(&row.task, !row.completed)
            .map_err(engine_error)
    }

    /// Answer a question; answering with the answer it already has clears it,
    /// so a mis-tap can be undone (as in the standalone Checklist app).
    fn answer_checklist_decision(
        &mut self,
        event: &Event,
        answer: bool,
    ) -> Result<(), TaskAppError> {
        let row = self.checklist_run_row(event)?;
        if !row.is_decision {
            return Err(TaskAppError::InvalidPayload {
                event: event.name.clone(),
                field: "index",
            });
        }
        let project = self.active_project_mut();
        if row.answered == Some(answer) {
            project.clear_decision_answer(&row.task)
        } else {
            project.answer_decision(&row.task, answer)
        }
        .map_err(engine_error)
    }

    fn finish_checklist_run(
        &mut self,
        event: &Event,
        complete: bool,
    ) -> Result<AppUpdate, TaskAppError> {
        let run = self.selected_checklist_of_kind(event, true)?;
        let now = (self.clock)();
        let project = self.active_project_mut();
        if complete {
            project.complete_checklist_run(&run, now)
        } else {
            project.abandon_checklist_run(&run, now)
        }
        .map_err(engine_error)?;
        Ok(self.announced_update(if complete {
            "Run completed"
        } else {
            "Run abandoned"
        }))
    }

    fn delete_checklist(&mut self, event: &Event) -> Result<AppUpdate, TaskAppError> {
        let id =
            self.state
                .selected_checklist
                .clone()
                .ok_or_else(|| TaskAppError::InvalidPayload {
                    event: event.name.clone(),
                    field: "selection",
                })?;
        self.active_project_mut()
            .delete_checklist(&id)
            .map_err(engine_error)?;
        self.state.selected_checklist = None;
        self.state.selected_outline_item = None;
        self.state.new_checklist_item.clear();
        Ok(self.announced_update("Checklist deleted"))
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
                    if self.state.editing_task.as_ref() == Some(id) {
                        "editing"
                    } else {
                        ""
                    }
                    .to_string(),
                    format!(
                        "{} task: {}",
                        if task.completed { "Reopen" } else { "Complete" },
                        task.name
                    ),
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
            "newTaskNameChange" => {
                self.state.new_task_name = text_payload(event, "value")?;
                if !self.state.new_task_name.trim().is_empty() {
                    self.state.new_task_name_error.clear();
                }
            }
            "newTaskDueChange" => {
                self.state.new_task_due = text_payload(event, "value")?;
                let correcting_error = !self.state.new_task_due_error.is_empty();
                if self.state.new_task_due.trim().is_empty()
                    || parse_date(&self.state.new_task_due).is_some()
                {
                    self.state.new_task_due_error.clear();
                    if correcting_error {
                        self.state.new_task_due_focus = "focus".to_string();
                    }
                }
            }
            "editTaskNameChange" => {
                self.state.edit_task_name = text_payload(event, "value")?;
                if !self.state.edit_task_name.trim().is_empty() {
                    self.state.edit_task_name_error.clear();
                }
            }
            "editTaskDueChange" => {
                self.state.edit_task_due = text_payload(event, "value")?;
                if self.state.edit_task_due.trim().is_empty()
                    || parse_date(&self.state.edit_task_due).is_some()
                {
                    self.state.edit_task_due_error.clear();
                }
            }
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
            "showChecklists" => self.state.view = ViewMode::Checklists,
            // The switcher's selection. The index is into the views THIS
            // project offers, so Timeline (index 6) is out of range for a
            // Board-tier project and refused, like any other bad index.
            "showView" => {
                let index = index_payload(event, "index")?;
                let full = self.active_project().settings.complexity == ProjectComplexity::Full;
                self.state.view = *ViewMode::switcher_views(full).get(index).ok_or(
                    TaskAppError::InvalidPayload {
                        event: event.name.clone(),
                        field: "index",
                    },
                )?;
            }
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
            "editTask" => self.edit_task(index_payload(event, "index")?),
            "saveTaskEdit" => return self.save_task_edit(),
            "cancelTaskEdit" => self.clear_task_edit(true),
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
            "newNote" => self.new_note()?,
            "saveNote" => return self.save_note(),
            "deleteNote" => return self.delete_note(),
            "selectChecklist" => self.select_checklist(index_payload(event, "index")?),
            "newChecklistNameChange" => {
                self.state.new_checklist_name = bounded_text_payload(event, "value")?
            }
            "newChecklistItemChange" => {
                self.state.new_checklist_item = bounded_text_payload(event, "value")?
            }
            "createChecklist" => return self.create_checklist(),
            "addChecklistItem" => return self.add_checklist_item(event),
            "selectOutlineItem" => self.select_outline_item(event)?,
            "toggleOutlineQuestion" => return self.toggle_outline_question(event),
            "addChecklistItemYes" => return self.add_checklist_branch_item(event, true),
            "addChecklistItemNo" => return self.add_checklist_branch_item(event, false),
            "deleteOutlineItem" => return self.delete_outline_item(event),
            "startChecklistRun" => return self.start_checklist_run(event),
            "checklistToggle" => self.toggle_checklist_item(event)?,
            "checklistAnswerYes" => self.answer_checklist_decision(event, true)?,
            "checklistAnswerNo" => self.answer_checklist_decision(event, false)?,
            "completeChecklistRun" => return self.finish_checklist_run(event, true),
            "abandonChecklistRun" => return self.finish_checklist_run(event, false),
            "deleteChecklist" => return self.delete_checklist(event),
            _ => return Err(TaskAppError::UnknownEvent(event.name.clone())),
        }
        Ok(self.update())
    }

    fn add_task(&mut self) -> Result<AppUpdate, TaskAppError> {
        let name = self.state.new_task_name.trim().to_string();
        if name.is_empty() {
            self.state.new_task_name_error = "Enter a task name.".to_string();
            return Ok(self.update());
        }
        self.state.new_task_name_error.clear();
        let due_text = self.state.new_task_due.trim();
        let due = if due_text.is_empty() {
            None
        } else if let Some(due) = parse_date(due_text) {
            Some(due)
        } else {
            self.state.new_task_due_error = "Use a real date in YYYY-MM-DD format.".to_string();
            return Ok(self.update());
        };
        self.state.new_task_due_error.clear();
        let previous = self.ordered_task_ids().last().cloned();
        let id = self.next_task_id()?;
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
        self.state.new_task_name_focus = "focus".to_string();
        self.state.new_task_due_focus.clear();
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
        if self.state.editing_task.as_ref() == Some(&id) {
            self.clear_task_edit(true);
        }
        Ok(self.announced_update(format!("Deleted {name}")))
    }

    fn edit_task(&mut self, index: usize) {
        let Some(id) = self.task_ids().get(index).cloned() else {
            return;
        };
        let (name, due) = {
            let task = &self.active_project().tasks[&id];
            (
                task.name.clone(),
                task.schedule
                    .as_ref()
                    .and_then(|schedule| schedule.deadline)
                    .map(format_date)
                    .unwrap_or_default(),
            )
        };
        self.state.edit_task_name = name;
        self.state.edit_task_due = due;
        self.state.editing_task = Some(id.clone());
        self.state.expanded_task = None;
        self.state.edit_task_name_error.clear();
        self.state.edit_task_due_error.clear();
    }

    fn save_task_edit(&mut self) -> Result<AppUpdate, TaskAppError> {
        let Some(id) = self.state.editing_task.clone() else {
            return Ok(self.update());
        };
        let name = self.state.edit_task_name.trim().to_string();
        if name.is_empty() {
            self.state.edit_task_name_error = "Enter a task name.".to_string();
            return Ok(self.update());
        }
        self.state.edit_task_name_error.clear();
        let due_text = self.state.edit_task_due.trim();
        let due = if due_text.is_empty() {
            None
        } else if let Some(due) = parse_date(due_text) {
            Some(due)
        } else {
            self.state.edit_task_due_error = "Use a real date in YYYY-MM-DD format.".to_string();
            return Ok(self.update());
        };
        self.state.edit_task_due_error.clear();
        let project = self.active_project_mut();
        project
            .rename_task(&id, name.trim())
            .map_err(engine_error)?;
        project.set_deadline(&id, due).map_err(engine_error)?;
        self.clear_task_edit(true);
        Ok(self.announced_update(format!("Saved {name}")))
    }

    fn clear_task_edit(&mut self, return_focus: bool) {
        self.state.editing_task = None;
        self.state.edit_task_name.clear();
        self.state.edit_task_due.clear();
        self.state.edit_task_name_error.clear();
        self.state.edit_task_due_error.clear();
        if return_focus {
            self.state.new_task_name_focus = "focus".to_string();
        }
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
        let n = self.bump_next_id()?;
        let id = ProjectId::from_raw(format!("project-{n}"));
        let parent = nested.then(|| self.state.active_project.clone());
        self.state
            .workspace
            .create_project(id.clone(), name.clone(), parent)
            .map_err(engine_error)?;
        self.state.active_project = id;
        self.state.selected_checklist = None;
        self.state.selected_outline_item = None;
        self.state.new_project_name.clear();
        self.state.view = ViewMode::List;
        Ok(self.announced_update(format!("Created project {name}")))
    }

    fn select_project(&mut self, index: usize) {
        if let Some(id) = self.project_rows().0.get(index).cloned() {
            if id != self.state.active_project {
                self.state.selected_checklist = None;
                self.state.selected_outline_item = None;
            }
            self.state.active_project = id;
            self.state.expanded_task = None;
            self.clear_task_edit(false);
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
        if !self.is_listed_task(&id) {
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
        let n = self.bump_next_id()?;
        let id = LabelId::from_raw(format!("label-{n}"));
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
        if self.is_listed_task(&id) {
            self.active_project_mut()
                .set_constraint(&id, Constraint::MustStartOn(date))
                .map_err(engine_error)?;
        }
        Ok(())
    }

    /// A drop names its task by key, straight from the payload. Only a task the
    /// views actually show may be moved: a checklist's items are not on the
    /// board or the calendar, and `set_constraint` has no checklist guard, so a
    /// crafted drop could otherwise stamp a template item (copied into every
    /// run) or a finished run's record.
    fn is_listed_task(&self, id: &TaskId) -> bool {
        let project = self.active_project();
        project.tasks.contains_key(id) && !project.checklist_owned().contains(id)
    }

    fn new_note(&mut self) -> Result<(), TaskAppError> {
        let n = self.bump_next_id()?;
        self.state.selected_note_id = Some(NoteId::from_raw(format!("note-{n}")));
        self.state.note_title.clear();
        self.state.note_body.clear();
        self.state.note_task_name.clear();
        Ok(())
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

    fn next_task_id(&mut self) -> Result<TaskId, TaskAppError> {
        loop {
            let n = self.bump_next_id()?;
            let id = TaskId::from_raw(format!("task-{n}"));
            if self.state.workspace.project_of_task(&id).is_none() {
                return Ok(id);
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

/// A composer value, refused past [`MAX_CHECKLIST_TEXT_CHARS`].
fn bounded_text_payload(event: &Event, field: &'static str) -> Result<String, TaskAppError> {
    let value = text_payload(event, field)?;
    if value.chars().count() > MAX_CHECKLIST_TEXT_CHARS {
        return Err(TaskAppError::InvalidPayload {
            event: event.name.clone(),
            field,
        });
    }
    Ok(value)
}

/// The Checklists view's computed slot values; empty outside the view.
#[derive(Default)]
struct ChecklistProps {
    library_rows: Vec<Vec<String>>,
    template_mode: bool,
    run_mode: bool,
    outline_rows: Vec<Vec<String>>,
    outline_item_selected: bool,
    outline_question_selected: bool,
    outline_toggle_label: String,
    run_title: String,
    run_progress: String,
    run_rows: Vec<Vec<String>>,
    complete_label: String,
    abandon_label: String,
}

/// `"3 of 5 done"`, with `" · 1 of 2 answered"` when the run has questions;
/// a finished run just says how it finished.
fn progress_label(
    status: RunStatus,
    progress: &task_core::checklists::ChecklistProgress,
) -> String {
    match status {
        RunStatus::Completed => "Completed".to_string(),
        RunStatus::Abandoned => "Abandoned".to_string(),
        RunStatus::InProgress if progress.decisions > 0 => format!(
            "{} of {} done · {} of {} answered",
            progress.checked, progress.total, progress.answered, progress.decisions
        ),
        RunStatus::InProgress => format!("{} of {} done", progress.checked, progress.total),
    }
}

fn item_count_label(items: usize) -> String {
    match items {
        1 => "1 item".to_string(),
        n => format!("{n} items"),
    }
}

/// RecordList draws a row's title as its button: never let it be empty.
fn display_name(name: &str) -> String {
    if name.trim().is_empty() {
        "Untitled checklist".to_string()
    } else {
        name.to_string()
    }
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
        .and_then(json_index)
        .ok_or_else(|| TaskAppError::InvalidPayload {
            event: event.name.clone(),
            field,
        })
}

fn json_index(value: &Value) -> Option<usize> {
    if let Some(value) = value.as_u64() {
        return usize::try_from(value).ok();
    }

    let value = value.as_f64()?;
    let exclusive_upper_bound = 2.0_f64.powi(usize::BITS as i32);
    if !value.is_finite() || value < 0.0 || value >= exclusive_upper_bound || value.fract() != 0.0 {
        return None;
    }

    let index = value as usize;
    (index as f64 == value).then_some(index)
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

fn local_data_guidance() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        return "Local only · %LOCALAPPDATA%\\task-app\\mosaic-state.v1.json · close Trestle before backup or restore";
    }
    #[cfg(target_os = "macos")]
    {
        return "Local only · ~/Library/Application Support/task-app/mosaic-state.v1.json · close Trestle before backup or restore";
    }
    #[cfg(target_os = "linux")]
    {
        return "Local only · see LOCAL-DATA.txt beside this release for the exact Linux backend path and backup steps";
    }
    #[allow(unreachable_code)]
    "Local only · see LOCAL-DATA.txt beside this release for the platform data path and backup steps"
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

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PresentationContract {
        schema_version: u32,
        today: i32,
        today_iso: String,
        host_only_differences: Value,
        steps: Vec<PresentationStep>,
    }

    #[derive(Debug, Deserialize)]
    struct PresentationStep {
        id: String,
        event: Option<PresentationEvent>,
        #[serde(default)]
        restore: bool,
        expected: Value,
    }

    #[derive(Debug, Deserialize)]
    struct PresentationEvent {
        #[serde(rename = "type")]
        event_type: String,
        payload: Value,
    }

    const REQUIRED_PROPS: &[&str] = &[
        "app-title",
        "new-task-name",
        "new-task-due",
        "new-task-name-error",
        "new-task-due-error",
        "new-task-name-focus",
        "new-task-due-focus",
        "edit-task-name",
        "edit-task-due",
        "edit-task-name-error",
        "edit-task-due-error",
        "empty-list",
        "project-rows",
        "new-project-name",
        "summary",
        "status-label",
        "status-warn",
        "storage-status",
        "storage-location",
        "storage-warning",
        "ring-gradient",
        "ring-percent",
        "ring-percent-value",
        "theme-is-dark",
        "complexity-label",
        "allow-timeline",
        "nav-options",
        "nav-selected-index",
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
        "checklists-mode",
        "checklists-title",
        "checklist-library-rows",
        "checklist-library-empty",
        "selected-checklist-key",
        "new-checklist-name",
        "checklist-template-mode",
        "checklist-run-mode",
        "checklist-outline-rows",
        "selected-outline-key",
        "outline-item-selected",
        "outline-question-selected",
        "outline-toggle-label",
        "new-checklist-item",
        "checklist-run-title",
        "checklist-run-progress",
        "checklist-run-rows",
        "checklist-complete-label",
        "checklist-abandon-label",
        "task-rows",
    ];

    fn context() -> StartContext {
        StartContext::new("en-US", Platform::Linux)
    }

    fn event(sequence: u64, name: &str, payload: Value) -> Event {
        Event::new(sequence, format!("on{}", capitalize(name)), payload)
    }

    fn canonical_contract(app: &TaskMosaicApp, props: &Value) -> Value {
        let active = app.active_project();
        let mut projects = app
            .state
            .workspace
            .projects
            .values()
            .map(|project| {
                let mut tasks = project
                    .tasks
                    .values()
                    .map(|task| {
                        json!({
                            "name": task.name,
                            "completed": task.completed,
                            "deadline": task.schedule.as_ref().and_then(|schedule| schedule.deadline).map(|date| date.0),
                        })
                    })
                    .collect::<Vec<_>>();
                tasks.sort_by_key(|task| task["name"].as_str().unwrap_or_default().to_string());
                json!({
                    "name": project.name,
                    "complexity": match project.settings.complexity {
                        ProjectComplexity::Board => "board",
                        ProjectComplexity::Full => "full",
                    },
                    "tasks": tasks,
                })
            })
            .collect::<Vec<_>>();
        projects.sort_by_key(|project| project["name"].as_str().unwrap_or_default().to_string());
        let view = if props["board-mode"] == "board" {
            "board"
        } else if props["timeline-mode"] == "timeline" {
            "timeline"
        } else if props["sheet-mode"] == "sheet" {
            "sheet"
        } else if props["calendar-mode"] == "calendar" {
            "calendar"
        } else if props["notes-mode"] == "notes" {
            "notes"
        } else if props["checklists-mode"] == "checklists" {
            "checklists"
        } else {
            "list"
        };
        let task_rows = props["task-rows"]
            .as_array()
            .expect("task rows are an array")
            .iter()
            .map(|row| {
                Value::Array(
                    row.as_array()
                        .expect("task row is an array")
                        .iter()
                        .take(4)
                        .cloned()
                        .collect(),
                )
            })
            .collect::<Vec<_>>();
        json!({
            "engine": {
                "activeProject": active.name,
                "projects": projects,
            },
            "slots": {
                "view": view,
                "summary": props["summary"],
                "ringPercent": props["ring-percent"],
                "complexityLabel": props["complexity-label"],
                "newTaskName": props["new-task-name"],
                "newTaskDue": props["new-task-due"],
                "emptyList": props["empty-list"],
                "newProjectName": props["new-project-name"],
                "projectRows": props["project-rows"],
                "taskRows": task_rows,
            },
        })
    }

    #[test]
    fn shared_web_native_presentation_contract() {
        let fixture: PresentationContract = serde_json::from_str(include_str!(
            "../../../../programs/mosaic/task-app/fixtures/presentation-contract-v1.json"
        ))
        .expect("valid presentation contract fixture");
        assert_eq!(fixture.schema_version, 1);
        assert_eq!(fixture.today, PROJECT_START.0);
        assert_eq!(fixture.today_iso, format_date(PROJECT_START));
        assert!(fixture.host_only_differences.is_object());

        let mut app = TaskMosaicApp::default();
        let mut update = app.start(context()).unwrap();
        let mut sequence = 1;
        for step in fixture.steps {
            if step.restore {
                let snapshot = app.snapshot().unwrap().expect("TaskApp snapshot");
                let mut restored = TaskMosaicApp::default();
                update = restored.restore(snapshot).unwrap();
                app = restored;
            } else if let Some(input) = step.event {
                update = app
                    .dispatch(event(sequence, &input.event_type, input.payload))
                    .unwrap_or_else(|error| panic!("{}: {error}", step.id));
                sequence += 1;
            }
            assert_eq!(
                canonical_contract(&app, &update.props),
                step.expected,
                "{}",
                step.id
            );
        }
    }

    #[test]
    fn start_returns_every_required_task_app_prop() {
        let mut runtime = MosaicRuntime::new(TaskMosaicApp::default());
        let update = runtime.start(context()).unwrap();
        let object = update.props.as_object().unwrap();
        for key in REQUIRED_PROPS {
            assert!(object.contains_key(*key), "missing required prop {key}");
        }
        assert_eq!(object["storage-status"], "Saved locally on this device");
        assert!(object["storage-location"]
            .as_str()
            .is_some_and(|location| location.starts_with("Local only · ")));
        assert_eq!(object["storage-warning"], "");
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
        assert_eq!(added.props["task-rows"][0][3], "");
        assert_eq!(
            added.props["task-rows"][0][16],
            "Complete task: Ship native TaskApp"
        );
        let scheduled = runtime
            .dispatch(event(4, "toggleProjectComplexity", json!({})))
            .unwrap();
        assert_eq!(
            scheduled.props["task-rows"][0][3],
            "2026-01-05 → 2026-01-05"
        );
        let completed = runtime
            .dispatch(event(5, "toggleTask", json!({"index":0.0})))
            .unwrap();
        assert_eq!(completed.props["task-rows"][0][0], "✓");
        assert_eq!(
            completed.props["task-rows"][0][16],
            "Reopen task: Ship native TaskApp"
        );
        assert_eq!(completed.props["ring-percent"], "100%");
        // #12028 item 2: the same percent as typed data, not just the
        // pre-formatted caption string — this is what a future native
        // rendering of the progress ring would consume.
        assert_eq!(completed.props["ring-percent-value"], 100);
        let reopened = runtime
            .dispatch(event(6, "toggleTask", json!({"index":0})))
            .unwrap();
        assert_eq!(reopened.props["task-rows"][0][0], "○");
        assert_eq!(
            reopened.props["task-rows"][0][16],
            "Complete task: Ship native TaskApp"
        );
        assert_eq!(reopened.props["ring-percent"], "0%");
        let deleted = runtime
            .dispatch(event(7, "deleteTask", json!({"index":0.0})))
            .unwrap();
        assert_eq!(deleted.props["task-rows"], json!([]));
    }

    #[test]
    fn composer_validation_is_visible_correctable_and_atomic() {
        let mut app = TaskMosaicApp::default();
        let started = app.start(context()).unwrap();
        assert_eq!(started.props["new-task-name-error"], "");
        assert_eq!(started.props["new-task-due-error"], "");
        assert_eq!(started.props["new-task-name-focus"], "");
        assert_eq!(started.props["new-task-due-focus"], "");

        let before_blank = app.snapshot().unwrap();
        let blank = app.dispatch(event(1, "addTask", json!({}))).unwrap();
        assert_eq!(blank.props["new-task-name-error"], "Enter a task name.");
        assert_eq!(blank.props["task-rows"], json!([]));
        assert_eq!(app.snapshot().unwrap(), before_blank);

        let corrected_name = app
            .dispatch(event(
                2,
                "newTaskNameChange",
                json!({"value":"Plan the launch"}),
            ))
            .unwrap();
        assert_eq!(corrected_name.props["new-task-name-error"], "");
        app.dispatch(event(3, "newTaskDueChange", json!({"value":"2026-02-31"})))
            .unwrap();
        let before_bad_due = app.snapshot().unwrap();
        let bad_due = app.dispatch(event(4, "addTask", json!({}))).unwrap();
        assert_eq!(
            bad_due.props["new-task-due-error"],
            "Use a real date in YYYY-MM-DD format."
        );
        assert_eq!(bad_due.props["task-rows"], json!([]));
        assert_eq!(app.snapshot().unwrap(), before_bad_due);

        let corrected_due = app
            .dispatch(event(5, "newTaskDueChange", json!({"value":"2026-02-28"})))
            .unwrap();
        assert_eq!(corrected_due.props["new-task-due-error"], "");
        assert_eq!(corrected_due.props["new-task-due-focus"], "focus");
        let added = app.dispatch(event(6, "addTask", json!({}))).unwrap();
        assert_eq!(added.props["task-rows"][0][1], "Plan the launch");
        assert_eq!(added.props["task-rows"][0][2], "due 2026-02-28");
        assert_eq!(added.props["new-task-name-focus"], "focus");
        assert_eq!(added.props["new-task-due-focus"], "");
    }

    #[test]
    fn list_edit_validates_atomically_and_commits_through_task_core() {
        let mut app = TaskMosaicApp::default();
        app.start(context()).unwrap();
        app.dispatch(event(1, "newTaskNameChange", json!({"value":"Draft plan"})))
            .unwrap();
        app.dispatch(event(2, "addTask", json!({}))).unwrap();

        let editing = app
            .dispatch(event(3, "editTask", json!({"index":0})))
            .unwrap();
        assert_eq!(editing.props["task-rows"][0][15], "editing");
        assert_eq!(editing.props["edit-task-name"], "Draft plan");

        app.dispatch(event(4, "editTaskNameChange", json!({"value":""})))
            .unwrap();
        app.dispatch(event(5, "editTaskDueChange", json!({"value":"2026-02-31"})))
            .unwrap();
        let before_invalid = app.snapshot().unwrap();
        let invalid_name = app.dispatch(event(6, "saveTaskEdit", json!({}))).unwrap();
        assert_eq!(
            invalid_name.props["edit-task-name-error"],
            "Enter a task name."
        );
        assert_eq!(
            app.active_project().tasks[&TaskId::from_raw("task-1")].name,
            "Draft plan"
        );
        assert_eq!(app.snapshot().unwrap(), before_invalid);

        app.dispatch(event(
            7,
            "editTaskNameChange",
            json!({"value":"Launch plan"}),
        ))
        .unwrap();
        let invalid_due = app.dispatch(event(8, "saveTaskEdit", json!({}))).unwrap();
        assert_eq!(
            invalid_due.props["edit-task-due-error"],
            "Use a real date in YYYY-MM-DD format."
        );
        assert_eq!(
            app.active_project().tasks[&TaskId::from_raw("task-1")].name,
            "Draft plan"
        );

        app.dispatch(event(9, "editTaskDueChange", json!({"value":"2026-02-28"})))
            .unwrap();
        let saved = app.dispatch(event(10, "saveTaskEdit", json!({}))).unwrap();
        let task = &app.active_project().tasks[&TaskId::from_raw("task-1")];
        assert_eq!(task.name, "Launch plan");
        assert_eq!(
            task.schedule
                .as_ref()
                .and_then(|schedule| schedule.deadline),
            Date::from_ymd(2026, 2, 28)
        );
        assert_eq!(saved.props["task-rows"][0][15], "");
        assert_eq!(saved.props["new-task-name-focus"], "focus");
    }

    #[test]
    fn index_payload_accepts_only_in_range_integral_json_numbers() {
        assert_eq!(json_index(&json!(0)), Some(0));
        assert_eq!(json_index(&json!(42.0)), Some(42));
        assert_eq!(json_index(&json!(-0.0)), Some(0));
        assert_eq!(json_index(&json!(0.5)), None);
        assert_eq!(json_index(&json!(-1.0)), None);
        assert_eq!(json_index(&json!(2.0_f64.powi(usize::BITS as i32))), None);
        assert_eq!(json_index(&json!("0")), None);
    }

    #[test]
    fn fractional_index_event_is_rejected_without_mutating_state() {
        let mut app = TaskMosaicApp::default();
        app.start(context()).unwrap();
        let before = app.snapshot().unwrap();

        assert!(matches!(
            app.dispatch(event(1, "toggleTask", json!({"index":0.5}))),
            Err(TaskAppError::InvalidPayload { field: "index", .. })
        ));
        assert_eq!(app.snapshot().unwrap(), before);
    }

    /// #14016: the toolkit SegmentedControl is fed by `nav-options` /
    /// `nav-selected-index` and answers with `onShowView(index)`. Rows, index
    /// and the `*-mode` slots must describe the same view at every step, and
    /// Timeline is offered only to a Full project.
    #[test]
    fn view_switcher_rows_track_the_view_and_the_tier() {
        let mut app = TaskMosaicApp::default();
        app.start(context()).unwrap();
        let labels = |props: &Value| -> Vec<String> {
            props["nav-options"]
                .as_array()
                .unwrap()
                .iter()
                .map(|label| label.as_str().unwrap().to_string())
                .collect()
        };

        // A new project is Board tier: six views, no Timeline.
        let board = app
            .dispatch(event(1, "showView", json!({"index":1})))
            .unwrap()
            .props;
        assert_eq!(
            labels(&board),
            ["List", "Board", "Sheet", "Calendar", "Notes", "Checklists"]
        );
        assert_eq!(board["nav-selected-index"], 1);
        assert_eq!(board["board-mode"], "board");
        assert_eq!(
            board["board-columns"],
            json!([
                ["Up next", "next", "0", "#9b9289"],
                ["In progress", "doing", "0", "#e0942a"],
                ["Done", "done", "0", "#4f8e6a"]
            ]),
            "every board column row must satisfy the four-cell layout contract"
        );

        // Index 6 is Timeline, which a Board-tier project does not offer.
        let before = app.snapshot().unwrap();
        assert!(matches!(
            app.dispatch(event(2, "showView", json!({"index":6}))),
            Err(TaskAppError::InvalidPayload { field: "index", .. })
        ));
        assert_eq!(
            app.snapshot().unwrap(),
            before,
            "a refused index changes nothing"
        );

        // Full tier adds Timeline last, so no other index moves.
        let full = app
            .dispatch(event(3, "toggleProjectComplexity", json!({})))
            .unwrap()
            .props;
        assert_eq!(
            labels(&full),
            [
                "List",
                "Board",
                "Sheet",
                "Calendar",
                "Notes",
                "Checklists",
                "Timeline"
            ]
        );
        let modes = [
            ("", ""),
            ("board-mode", "board"),
            ("sheet-mode", "sheet"),
            ("calendar-mode", "calendar"),
            ("notes-mode", "notes"),
            ("checklists-mode", "checklists"),
            ("timeline-mode", "timeline"),
        ];
        for (index, (slot, value)) in modes.iter().enumerate() {
            let props = app
                .dispatch(event(
                    4 + index as u64,
                    "showView",
                    json!({ "index": index }),
                ))
                .unwrap()
                .props;
            assert_eq!(props["nav-selected-index"], index, "index {index}");
            // The labels never change with the selection: the selected state
            // is the control's to report (UI86), not a word in the name.
            assert_eq!(
                labels(&props),
                [
                    "List",
                    "Board",
                    "Sheet",
                    "Calendar",
                    "Notes",
                    "Checklists",
                    "Timeline"
                ],
                "index {index}"
            );
            if !slot.is_empty() {
                assert_eq!(props[*slot], *value, "index {index}");
            }
        }

        // Dropping back to Board tier while on Timeline returns to List,
        // and the index follows.
        let back = app
            .dispatch(event(20, "toggleProjectComplexity", json!({})))
            .unwrap()
            .props;
        assert_eq!(back["nav-selected-index"], 0);
        assert_eq!(labels(&back).len(), 6);
    }

    #[test]
    fn every_declared_event_is_accepted() {
        let mut app = TaskMosaicApp::default();
        app.start(context()).unwrap();
        let cases = [
            ("newTaskNameChange", json!({"value":"Task"})),
            ("newTaskDueChange", json!({"value":""})),
            ("editTaskNameChange", json!({"value":"Task"})),
            ("editTaskDueChange", json!({"value":""})),
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
            ("showChecklists", json!({})),
            ("showView", json!({"index":0})),
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
            ("editTask", json!({"index":0})),
            ("saveTaskEdit", json!({})),
            ("cancelTaskEdit", json!({})),
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
    fn release_upgrade_fixture_from_v0_1_0_restores() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../programs/mosaic/task-app/fixtures/release-upgrade-v0.1.0.json"
        ))
        .expect("valid release upgrade fixture");
        assert_eq!(fixture["fixtureSchema"], "task-app/release-upgrade-fixture");
        assert_eq!(fixture["sourceVersion"], "0.1.0");

        let snapshot = Snapshot {
            schema: fixture["snapshotSchema"].as_str().unwrap().to_string(),
            version: fixture["snapshotVersion"].as_u64().unwrap() as u32,
            bytes: serde_json::to_vec(&fixture["state"]).unwrap(),
        };
        let mut restored = TaskMosaicApp::default();
        let update = restored.restore(snapshot).unwrap();
        assert_eq!(
            update.props["task-rows"][0][1],
            fixture["expectedTask"]["name"]
        );
        assert_eq!(update.props["task-rows"][0][2], "due 2026-01-09");
        assert_eq!(update.props["task-rows"][0][3], "2026-01-05 → 2026-01-05");
        assert_eq!(update.props["complexity-label"], "Full CPM");
    }

    #[test]
    fn protocol_constant_is_current() {
        assert_eq!(PROTOCOL_VERSION, 1);
    }

    /// #15131 -- `column-widths` is declared by `Grid.mil` as "per-column
    /// PIXEL widths", and every backend threads the number straight into a
    /// width. These were once `[3, 1, 2, 2, 2]`, plainly meant as relative
    /// proportions, and three of the sheet's cells measured ZERO WIDTH at
    /// 1280, 900 and 700 alike -- a 1px column has no room for its text.
    ///
    /// A ratio and a pixel width are both `number`, so nothing upstream can
    /// catch the confusion; this asserts the magnitude instead. The bound
    /// is deliberately loose -- it is here to catch single digits, not to
    /// pin a layout.
    #[test]
    fn sheet_column_widths_are_pixels_not_ratios() {
        let mut app = TaskMosaicApp::default();
        let update = app.start(context()).unwrap();
        let widths = update.props["sheet-column-widths"]
            .as_array()
            .expect("sheet-column-widths is a list");
        assert!(!widths.is_empty(), "the sheet declares columns");
        for (index, value) in widths.iter().enumerate() {
            let px = value.as_f64().expect("each width is a number");
            assert!(
                px >= 40.0,
                "column {index} is {px}px wide -- that is a ratio, not a pixel width (#15131)"
            );
        }
        // Parallel-shaped to the headers, which `Grid.mil` also requires.
        let headers = update.props["sheet-column-headers"]
            .as_array()
            .expect("sheet-column-headers is a list");
        assert_eq!(widths.len(), headers.len(), "one width per column");
    }

    // ── Checklists (C3) ─────────────────────────────────────────────────────

    const NOW: u64 = 1_790_000_000_000;

    fn fixed_clock() -> u64 {
        NOW
    }

    /// A started app in the Checklists view, with a fixed clock.
    fn checklists_app() -> TaskMosaicApp {
        let mut app = TaskMosaicApp::with_clock(fixed_clock);
        app.start(context()).unwrap();
        app.dispatch(event(1, "showChecklists", json!({}))).unwrap();
        app
    }

    fn send(app: &mut TaskMosaicApp, name: &str, payload: Value) -> Value {
        app.dispatch(event(1, name, payload))
            .unwrap_or_else(|error| panic!("{name}: {error}"))
            .props
    }

    /// Create a template named `name` holding `items`, left selected.
    fn template(app: &mut TaskMosaicApp, name: &str, items: &[&str]) -> Value {
        send(app, "newChecklistNameChange", json!({ "value": name }));
        let mut props = send(app, "createChecklist", json!({}));
        for item in items {
            send(app, "newChecklistItemChange", json!({ "value": item }));
            props = send(app, "addChecklistItem", json!({}));
        }
        props
    }

    fn column(rows: &Value, field: usize) -> Vec<String> {
        rows.as_array()
            .unwrap()
            .iter()
            .map(|row| row[field].as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn checklists_view_starts_empty() {
        let props = checklists_app().update().props;
        assert_eq!(props["checklists-mode"], "checklists");
        assert_eq!(props["checklist-library-empty"], true);
        assert_eq!(props["checklist-library-rows"], json!([]));
        assert_eq!(props["checklist-template-mode"], false);
        assert_eq!(props["checklist-run-mode"], false);
        assert_eq!(props["selected-checklist-key"], "");
    }

    #[test]
    fn a_template_is_built_run_and_completed() {
        let mut app = checklists_app();
        // Twelve items: the eleventh and twelfth must still come after the
        // ninth and tenth although their ids sort before them.
        let names: Vec<String> = (1..=12).map(|n| format!("Step {n}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let props = template(&mut app, "Release", &refs);
        assert_eq!(props["checklist-template-mode"], true);
        assert_eq!(column(&props["checklist-outline-rows"], 2), names);
        assert_eq!(props["new-checklist-item"], "", "the composer clears");
        assert_eq!(props["checklist-library-rows"][0][1], "Templates");
        assert_eq!(props["checklist-library-rows"][0][2], "Release");
        assert_eq!(props["checklist-library-rows"][0][3], "12 items");
        // Template items are not list tasks.
        assert_eq!(props["task-rows"], json!([]));
        assert_eq!(props["empty-list"], "empty");

        let props = send(&mut app, "startChecklistRun", json!({}));
        assert_eq!(props["checklist-run-mode"], true);
        assert_eq!(props["checklist-run-title"], "Release");
        assert_eq!(props["checklist-run-progress"], "0 of 12 done");
        assert_eq!(props["checklist-complete-label"], "", "not complete yet");
        assert_eq!(props["checklist-abandon-label"], "Abandon");
        assert_eq!(column(&props["checklist-run-rows"], 2), names);
        assert_eq!(props["checklist-run-rows"][0][3], "", "a check item");
        // The library: the template, then the run, each group headed once.
        assert_eq!(
            column(&props["checklist-library-rows"], 1),
            ["Templates", "Runs"]
        );

        let mut props = props;
        for index in 0..12 {
            props = send(&mut app, "checklistToggle", json!({ "index": index }));
        }
        assert_eq!(props["checklist-run-progress"], "12 of 12 done");
        assert_eq!(props["checklist-run-rows"][0][4], "1", "ticked");
        assert_eq!(props["checklist-complete-label"], "Complete");

        // Unticking withdraws the offer; ticking again restores it.
        let props = send(&mut app, "checklistToggle", json!({ "index": 3 }));
        assert_eq!(props["checklist-complete-label"], "");
        send(&mut app, "checklistToggle", json!({ "index": 3 }));

        let props = send(&mut app, "completeChecklistRun", json!({}));
        assert_eq!(props["checklist-run-progress"], "Completed");
        assert_eq!(props["checklist-complete-label"], "");
        assert_eq!(props["checklist-abandon-label"], "");
        assert_eq!(props["checklist-library-rows"][1][5], "✓");
        // Neither template nor run leaks into the other views.
        assert_eq!(props["task-rows"], json!([]));
        assert_eq!(
            app.active_project().checklists[&ChecklistId::from_raw("run-14")]
                .run
                .as_ref()
                .unwrap()
                .finished_at,
            Some(NOW)
        );
    }

    #[test]
    fn an_incomplete_run_cannot_be_completed() {
        let mut app = checklists_app();
        template(&mut app, "Release", &["Only step"]);
        send(&mut app, "startChecklistRun", json!({}));
        let before = app.snapshot().unwrap();
        assert!(matches!(
            app.dispatch(event(1, "completeChecklistRun", json!({}))),
            Err(TaskAppError::Engine(_))
        ));
        assert_eq!(app.snapshot().unwrap(), before);
    }

    #[test]
    fn a_decision_reveals_its_branch_and_a_repeat_answer_clears_it() {
        let mut app = checklists_app();
        template(
            &mut app,
            "Pre-flight",
            &["Raining?", "Take umbrella", "Wear hat"],
        );
        // Decision authoring is C3c; build the branch through the engine.
        let ids: Vec<TaskId> = {
            let props = app.update().props;
            column(&props["checklist-outline-rows"], 0)
                .into_iter()
                .map(TaskId::from_raw)
                .collect()
        };
        app.active_project_mut()
            .set_decision(
                &ids[0],
                Some(task_core::Decision {
                    question: "Raining?".to_string(),
                    answer: None,
                    yes_children: vec![ids[1].clone()],
                    no_children: vec![ids[2].clone()],
                }),
            )
            .unwrap();
        let props = app.update().props;
        assert_eq!(
            column(&props["checklist-outline-rows"], 1),
            ["", "  ", "  "],
            "the outline shows both branches, indented"
        );

        let props = send(&mut app, "startChecklistRun", json!({}));
        assert_eq!(column(&props["checklist-run-rows"], 2), ["Raining?"]);
        assert_eq!(props["checklist-run-rows"][0][3], "1", "a decision");
        assert_eq!(
            props["checklist-run-progress"],
            "0 of 0 done · 0 of 1 answered"
        );

        let props = send(&mut app, "checklistAnswerYes", json!({"index":0}));
        assert_eq!(
            column(&props["checklist-run-rows"], 2),
            ["Raining?", "Take umbrella"]
        );
        assert_eq!(props["checklist-run-rows"][0][4], "1");
        assert_eq!(props["checklist-run-rows"][0][5], "");

        let props = send(&mut app, "checklistAnswerNo", json!({"index":0}));
        assert_eq!(
            column(&props["checklist-run-rows"], 2),
            ["Raining?", "Wear hat"]
        );
        assert_eq!(props["checklist-run-rows"][0][5], "1");

        let props = send(&mut app, "checklistAnswerNo", json!({"index":0}));
        assert_eq!(
            column(&props["checklist-run-rows"], 2),
            ["Raining?"],
            "cleared"
        );

        // A decision is answered, not ticked; a check item is ticked, not answered.
        assert!(app
            .dispatch(event(1, "checklistToggle", json!({"index":0})))
            .is_err());
        send(&mut app, "checklistAnswerYes", json!({"index":0}));
        assert!(app
            .dispatch(event(1, "checklistAnswerYes", json!({"index":1})))
            .is_err());
        assert!(
            app.dispatch(event(1, "checklistToggle", json!({"index":2})))
                .is_err(),
            "out of range"
        );
    }

    #[test]
    fn an_abandoned_run_is_read_only() {
        let mut app = checklists_app();
        template(&mut app, "Release", &["Step"]);
        send(&mut app, "startChecklistRun", json!({}));
        let props = send(&mut app, "abandonChecklistRun", json!({}));
        assert_eq!(props["checklist-run-progress"], "Abandoned");
        assert_eq!(props["checklist-abandon-label"], "");
        assert_eq!(props["checklist-library-rows"][1][5], "✗");
        let before = app.snapshot().unwrap();
        assert!(app
            .dispatch(event(1, "checklistToggle", json!({"index":0})))
            .is_err());
        assert!(app
            .dispatch(event(1, "abandonChecklistRun", json!({})))
            .is_err());
        assert_eq!(app.snapshot().unwrap(), before);
    }

    #[test]
    fn template_and_run_events_need_the_right_selection() {
        let mut app = checklists_app();
        for name in [
            "addChecklistItem",
            "startChecklistRun",
            "completeChecklistRun",
            "deleteChecklist",
        ] {
            assert!(
                app.dispatch(event(1, name, json!({}))).is_err(),
                "{name} with nothing selected"
            );
        }
        template(&mut app, "Release", &["Step"]);
        assert!(
            app.dispatch(event(1, "completeChecklistRun", json!({})))
                .is_err(),
            "a template is not a run"
        );
        assert!(app
            .dispatch(event(1, "checklistToggle", json!({"index":0})))
            .is_err());
        send(&mut app, "startChecklistRun", json!({}));
        assert!(
            app.dispatch(event(1, "startChecklistRun", json!({})))
                .is_err(),
            "a run is not a template"
        );
        assert!(app
            .dispatch(event(1, "addChecklistItem", json!({})))
            .is_err());
    }

    #[test]
    fn empty_names_are_no_ops_and_long_ones_are_refused() {
        let mut app = checklists_app();
        send(&mut app, "newChecklistNameChange", json!({"value":"   "}));
        let props = send(&mut app, "createChecklist", json!({}));
        assert_eq!(props["checklist-library-empty"], true);

        let long = "x".repeat(MAX_CHECKLIST_TEXT_CHARS + 1);
        assert!(app
            .dispatch(event(1, "newChecklistNameChange", json!({"value": long})))
            .is_err());
        assert!(app
            .dispatch(event(1, "newChecklistItemChange", json!({"value": long})))
            .is_err());
        let fits = "é".repeat(MAX_CHECKLIST_TEXT_CHARS);
        send(&mut app, "newChecklistNameChange", json!({"value": fits}));

        template(&mut app, "Release", &[]);
        send(&mut app, "newChecklistItemChange", json!({"value":"  "}));
        let props = send(&mut app, "addChecklistItem", json!({}));
        assert_eq!(props["checklist-outline-rows"], json!([]));
    }

    #[test]
    fn selecting_and_deleting() {
        let mut app = checklists_app();
        template(&mut app, "Zulu", &["Z"]);
        let props = template(&mut app, "alpha", &["A"]);
        assert_eq!(
            column(&props["checklist-library-rows"], 2),
            ["alpha", "Zulu"],
            "by name, ignoring case"
        );
        let props = send(&mut app, "selectChecklist", json!({"index":1}));
        assert_eq!(column(&props["checklist-outline-rows"], 2), ["Z"]);
        let zulu = props["selected-checklist-key"].clone();
        assert_eq!(props["checklist-library-rows"][1][0], zulu);

        let props = send(&mut app, "deleteChecklist", json!({}));
        assert_eq!(props["selected-checklist-key"], "");
        assert_eq!(column(&props["checklist-library-rows"], 2), ["alpha"]);
        assert!(
            app.dispatch(event(1, "selectChecklist", json!({"index":5})))
                .is_ok(),
            "a stale index selects nothing"
        );
        assert_eq!(app.update().props["selected-checklist-key"], "");
    }

    #[test]
    fn switching_project_clears_the_selection() {
        let mut app = checklists_app();
        template(&mut app, "Release", &[]);
        send(&mut app, "newProjectNameChange", json!({"value":"Other"}));
        let props = send(&mut app, "addProject", json!({}));
        assert_eq!(props["selected-checklist-key"], "");
        assert_eq!(props["checklist-library-empty"], true);
    }

    #[test]
    fn the_selection_and_drafts_survive_a_restore() {
        let mut app = checklists_app();
        template(&mut app, "Release", &["Step"]);
        send(&mut app, "startChecklistRun", json!({}));
        send(&mut app, "newChecklistNameChange", json!({"value":"Draft"}));
        let snapshot = app.snapshot().unwrap().unwrap();
        let mut restored = TaskMosaicApp::with_clock(fixed_clock);
        let props = restored.restore(snapshot).unwrap().props;
        assert_eq!(props["checklist-run-mode"], true);
        assert_eq!(props["checklist-run-title"], "Release");
        assert_eq!(props["new-checklist-name"], "Draft");
    }

    #[test]
    fn restore_repairs_a_dangling_selection_and_an_oversized_draft() {
        let mut app = checklists_app();
        let snapshot = app.snapshot().unwrap().unwrap();
        let mut state: Value = serde_json::from_slice(&snapshot.bytes).unwrap();
        state["selectedChecklist"] = json!("missing");
        state["newChecklistItem"] = json!("x".repeat(MAX_CHECKLIST_TEXT_CHARS + 1));
        let props = app
            .restore(Snapshot {
                bytes: serde_json::to_vec(&state).unwrap(),
                ..snapshot
            })
            .unwrap()
            .props;
        assert_eq!(props["selected-checklist-key"], "");
        assert_eq!(props["new-checklist-item"], "");
    }

    #[test]
    fn a_snapshot_from_before_checklists_restores() {
        let mut app = checklists_app();
        let snapshot = app.snapshot().unwrap().unwrap();
        let mut state: Value = serde_json::from_slice(&snapshot.bytes).unwrap();
        for field in ["selectedChecklist", "newChecklistName", "newChecklistItem"] {
            state.as_object_mut().unwrap().remove(field).expect(field);
        }
        app.restore(Snapshot {
            bytes: serde_json::to_vec(&state).unwrap(),
            ..snapshot
        })
        .unwrap();
    }

    #[test]
    fn the_checklists_view_is_offered_at_both_tiers_and_by_index() {
        let mut app = checklists_app();
        let props = send(&mut app, "showView", json!({"index":5}));
        assert_eq!(props["checklists-mode"], "checklists");
        assert_eq!(props["nav-selected-index"], 5);
        // Rows are computed only in the view.
        template(&mut app, "Release", &["Step"]);
        let props = send(&mut app, "showList", json!({}));
        assert_eq!(props["checklist-library-rows"], json!([]));
        assert_eq!(props["checklist-outline-rows"], json!([]));
        assert_eq!(props["checklist-library-empty"], false);
    }

    #[test]
    fn a_deep_chain_draws_a_bounded_indent() {
        let mut app = checklists_app();
        template(&mut app, "Deep", &["Top"]);
        // Chain 40 items under one another through the engine (the view only
        // adds flat items; a restored snapshot can hold any depth).
        let mut parent = TaskId::from_raw(
            app.update().props["checklist-outline-rows"][0][0]
                .as_str()
                .unwrap(),
        );
        for n in 0..40 {
            let id = TaskId::from_raw(format!("deep-{n}"));
            app.active_project_mut()
                .create_task(id.clone(), format!("Level {n}"), Some(parent))
                .unwrap();
            parent = id;
        }
        let props = app.update().props;
        let indents = column(&props["checklist-outline-rows"], 1);
        assert_eq!(indents.len(), 41);
        let widest = indents.iter().map(String::len).max().unwrap();
        assert_eq!(widest, 2 * MAX_CHECKLIST_INDENT_DEPTH);
    }

    #[test]
    fn an_exhausted_id_counter_fails_the_event_instead_of_wrapping() {
        let mut app = checklists_app();
        template(&mut app, "Release", &[]);
        app.state.next_id = u64::MAX;
        send(&mut app, "newChecklistItemChange", json!({"value":"Step"}));
        let before = app.snapshot().unwrap();
        for name in ["addChecklistItem", "startChecklistRun", "newNote"] {
            assert!(
                matches!(
                    app.dispatch(event(1, name, json!({}))),
                    Err(TaskAppError::Engine(_))
                ),
                "{name}"
            );
            assert_eq!(app.snapshot().unwrap(), before, "{name} changed nothing");
        }
        send(&mut app, "newTaskNameChange", json!({"value":"Task"}));
        assert!(app.dispatch(event(1, "addTask", json!({}))).is_err());
    }

    #[test]
    fn drops_cannot_reach_checklist_items() {
        let mut app = checklists_app();
        template(&mut app, "Release", &["Step"]);
        let item = app.update().props["checklist-outline-rows"][0][0]
            .as_str()
            .unwrap()
            .to_string();
        send(
            &mut app,
            "calendarEventDropped",
            json!({"key": item, "kind":"task", "targetKey":"2026-02-02", "position":"inside"}),
        );
        send(
            &mut app,
            "cardDropped",
            json!({"key": item, "kind":"task", "targetKey":"done", "position":"inside"}),
        );
        let task = &app.active_project().tasks[&TaskId::from_raw(item)];
        assert!(
            task.schedule
                .as_ref()
                .is_none_or(|s| s.constraint == Constraint::Asap),
            "no constraint stamped"
        );
        assert!(!task.completed, "a template item is never ticked");
    }

    // ── C3c: writing questions into a template ──────────────────────────────

    fn outline(app: &TaskMosaicApp) -> Vec<Vec<String>> {
        serde_json::from_value(app.update().props["checklist-outline-rows"].clone()).unwrap()
    }

    fn select_item(app: &mut TaskMosaicApp, name: &str) -> Value {
        let index = outline(app)
            .iter()
            .position(|row| row[2] == name)
            .expect(name);
        send(app, "selectOutlineItem", json!({ "index": index }))
    }

    fn add_to(app: &mut TaskMosaicApp, branch: &str, name: &str) -> Value {
        send(app, "newChecklistItemChange", json!({ "value": name }));
        send(app, branch, json!({}))
    }

    #[test]
    fn a_question_gets_yes_and_no_items_and_a_run_reveals_one_branch() {
        let mut app = checklists_app();
        template(&mut app, "Pre-flight", &["Raining?", "Doors closed"]);
        let props = select_item(&mut app, "Raining?");
        assert_eq!(props["outline-item-selected"], true);
        assert_eq!(
            props["checklist-outline-rows"][0][5], "1",
            "the selected marker"
        );
        assert_eq!(props["checklist-outline-rows"][1][5], "");
        assert_eq!(props["outline-toggle-label"], "Make it a question");
        let props = send(&mut app, "toggleOutlineQuestion", json!({}));
        assert_eq!(props["outline-question-selected"], true);
        assert_eq!(props["outline-toggle-label"], "Make it a step");

        add_to(&mut app, "addChecklistItemYes", "Take umbrella");
        add_to(&mut app, "addChecklistItemNo", "Wear hat");
        add_to(&mut app, "addChecklistItemYes", "Close windows");
        let rows = outline(&app);
        let shape: Vec<(&str, &str, &str, &str)> = rows
            .iter()
            .map(|r| (r[1].as_str(), r[2].as_str(), r[3].as_str(), r[4].as_str()))
            .collect();
        assert_eq!(
            shape,
            [
                ("", "Raining?", "1", ""),
                ("  ", "Take umbrella", "", "Yes"),
                ("  ", "Close windows", "", "Yes"),
                ("  ", "Wear hat", "", "No"),
                ("", "Doors closed", "", ""),
            ]
        );

        let props = send(&mut app, "startChecklistRun", json!({}));
        assert_eq!(
            column(&props["checklist-run-rows"], 2),
            ["Raining?", "Doors closed"]
        );
        let props = send(&mut app, "checklistAnswerYes", json!({"index":0}));
        assert_eq!(
            column(&props["checklist-run-rows"], 2),
            ["Raining?", "Take umbrella", "Close windows", "Doors closed"]
        );
    }

    #[test]
    fn a_question_can_become_a_step_and_a_step_with_items_a_question() {
        let mut app = checklists_app();
        template(&mut app, "T", &["Q"]);
        select_item(&mut app, "Q");
        send(&mut app, "toggleOutlineQuestion", json!({}));
        add_to(&mut app, "addChecklistItemNo", "N");
        // Back to a step: its branch item becomes an ordinary sub-item.
        let props = send(&mut app, "toggleOutlineQuestion", json!({}));
        assert_eq!(props["outline-question-selected"], false);
        let rows = outline(&app);
        assert_eq!((rows[1][2].as_str(), rows[1][4].as_str()), ("N", ""));
        // And a question again: that sub-item becomes its Yes branch.
        send(&mut app, "toggleOutlineQuestion", json!({}));
        let rows = outline(&app);
        assert_eq!((rows[1][2].as_str(), rows[1][4].as_str()), ("N", "Yes"));
    }

    #[test]
    fn deleting_an_item_deletes_everything_under_it() {
        let mut app = checklists_app();
        template(&mut app, "T", &["Q", "After"]);
        select_item(&mut app, "Q");
        send(&mut app, "toggleOutlineQuestion", json!({}));
        add_to(&mut app, "addChecklistItemYes", "Y");
        select_item(&mut app, "Y");
        send(&mut app, "toggleOutlineQuestion", json!({}));
        add_to(&mut app, "addChecklistItemNo", "Deep");
        assert_eq!(
            column(&json!(outline(&app)), 2),
            ["Q", "Y", "Deep", "After"]
        );

        // Deleting a branch item takes it out of its question's branch.
        select_item(&mut app, "Deep");
        send(&mut app, "deleteOutlineItem", json!({}));
        assert_eq!(column(&json!(outline(&app)), 2), ["Q", "Y", "After"]);

        // Deleting a question deletes its branches too; nothing is stranded.
        select_item(&mut app, "Q");
        let props = send(&mut app, "deleteOutlineItem", json!({}));
        assert_eq!(props["selected-outline-key"], "");
        assert_eq!(column(&json!(outline(&app)), 2), ["After"]);
        send(&mut app, "startChecklistRun", json!({}));
        assert_eq!(
            column(&app.update().props["checklist-run-rows"], 2),
            ["After"]
        );
    }

    #[test]
    fn the_outline_selection_toggles_and_is_cleared_with_its_checklist() {
        let mut app = checklists_app();
        template(&mut app, "T", &["A"]);
        select_item(&mut app, "A");
        let props = select_item(&mut app, "A");
        assert_eq!(
            props["outline-item-selected"], false,
            "a second click clears it"
        );
        select_item(&mut app, "A");
        template(&mut app, "Other", &[]);
        assert_eq!(app.update().props["selected-outline-key"], "");
        // Without a selection (or a question), the authoring events are refused.
        for name in [
            "toggleOutlineQuestion",
            "deleteOutlineItem",
            "addChecklistItemYes",
        ] {
            send(&mut app, "newChecklistItemChange", json!({"value":"x"}));
            assert!(app.dispatch(event(1, name, json!({}))).is_err(), "{name}");
        }
        // The library lists templates by name: "Other", then "T".
        send(&mut app, "selectChecklist", json!({"index":1}));
        select_item(&mut app, "A");
        send(&mut app, "newChecklistItemChange", json!({"value":"x"}));
        assert!(
            app.dispatch(event(1, "addChecklistItemYes", json!({})))
                .is_err(),
            "A is a step"
        );
    }

    #[test]
    fn restore_drops_a_dangling_outline_selection() {
        let mut app = checklists_app();
        template(&mut app, "T", &["A"]);
        select_item(&mut app, "A");
        let snapshot = app.snapshot().unwrap().unwrap();
        let mut state: Value = serde_json::from_slice(&snapshot.bytes).unwrap();
        state["selectedOutlineItem"] = json!("missing");
        let props = app
            .restore(Snapshot {
                bytes: serde_json::to_vec(&state).unwrap(),
                ..snapshot
            })
            .unwrap()
            .props;
        assert_eq!(props["selected-outline-key"], "");
    }
}
