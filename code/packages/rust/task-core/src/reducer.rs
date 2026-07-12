//! The command/reducer surface.
//!
//! Every mutation to a [`ProjectState`] flows through one pure function:
//!
//! ```text
//! reduce(&state, command) -> new state
//! ```
//!
//! The reducer never mutates its input — it returns a fresh state — which is what
//! makes undo/redo a snapshot stack and keeps the WASM/host boundary simple (this is
//! the `engram-core` house pattern). It is also **total**: an invalid command (a
//! reparent that would form a cycle, a self-dependency, an out-of-range value) is a
//! no-op or is clamped rather than an error, so a host can dispatch freely without a
//! fallible contract. Where a value has a documented range, the reducer is the trust
//! boundary that enforces it — this is where the input validation the scheduler's
//! security review deferred lives (percent 0..=100, calendar interval bounds, cycle
//! rejection).

use crate::ids::*;
use crate::model::*;
use crate::primitives::{Date, Duration};
use std::collections::BTreeMap;

/// The largest number of working intervals we accept on a single day. A real day has
/// a handful; capping defends the working-time walk against a crafted calendar.
const MAX_INTERVALS_PER_DAY: usize = 48;

/// Every mutation the model supports. Deserialised from the host as JSON (behind the
/// `serde` feature); dispatched through [`reduce`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase", tag = "type"))]
pub enum TaskCommand {
    /// Rename the whole project.
    SetProjectName { name: String },
    /// Replace the entire state (load a document / import).
    LoadState { state: Box<ProjectState> },

    // ── tasks ───────────────────────────────────────────────────────────────────
    /// Create a leaf task (id minted by the facade).
    CreateTask {
        /// The new task's id.
        id: TaskId,
        /// Its name.
        name: String,
        /// Optional parent in the outline.
        parent: Option<TaskId>,
    },
    /// Rename a task.
    RenameTask {
        /// The task.
        id: TaskId,
        /// The new name.
        name: String,
    },
    /// Set a task's notes.
    SetNotes {
        /// The task.
        id: TaskId,
        /// The new notes.
        notes: String,
    },
    /// Delete a task; its children are reparented to its parent, and links,
    /// dependencies, and assignments referencing it are removed.
    DeleteTask {
        /// The task.
        id: TaskId,
    },
    /// Move a task under a new parent (rejected if it would form a cycle).
    Reparent {
        /// The task.
        id: TaskId,
        /// The new parent, or `None` for top level.
        new_parent: Option<TaskId>,
    },
    /// Set a task's sibling ordering key.
    SetOrder {
        /// The task.
        id: TaskId,
        /// The new order.
        order: i64,
    },
    /// Set a task's kind (leaf/summary/milestone).
    SetKind {
        /// The task.
        id: TaskId,
        /// The new kind.
        kind: TaskKind,
    },
    /// Toggle a summary's collapsed state.
    ToggleCollapsed {
        /// The task.
        id: TaskId,
    },

    // ── progress / workflow ──────────────────────────────────────────────────────
    /// Set a task's workflow status.
    SetStatus {
        /// The task.
        id: TaskId,
        /// The status, or `None` for the board default.
        status: Option<StatusId>,
    },
    /// Set a task's completion flag.
    SetCompleted {
        /// The task.
        id: TaskId,
        /// Whether it is done.
        completed: bool,
    },
    /// Set percent complete (**clamped to 0..=100**).
    SetPercentComplete {
        /// The task.
        id: TaskId,
        /// The percentage (values above 100 are clamped).
        percent: u8,
    },

    // ── scheduling ───────────────────────────────────────────────────────────────
    /// Set or clear a task's whole scheduling block.
    SetSchedule {
        /// The task.
        id: TaskId,
        /// The block, or `None` to make the task unscheduled.
        schedule: Option<Box<TaskSchedule>>,
    },
    /// Set a task's duration (creating a default schedule block if absent).
    SetDuration {
        /// The task.
        id: TaskId,
        /// The new duration.
        duration: Duration,
    },
    /// Set a task's date constraint (creating a schedule block if absent).
    SetConstraint {
        /// The task.
        id: TaskId,
        /// The constraint.
        constraint: Constraint,
    },
    /// Set or clear a task's deadline.
    SetDeadline {
        /// The task.
        id: TaskId,
        /// The deadline, or `None` to clear.
        deadline: Option<Date>,
    },

    // ── relations ────────────────────────────────────────────────────────────────
    /// Add a dependency (rejected if it is a self-link, a duplicate, or would form a
    /// cycle in the dependency network).
    LinkDependency {
        /// The dependency link.
        link: DependencyLink,
    },
    /// Remove a dependency by id.
    UnlinkDependency {
        /// The link id.
        id: LinkId,
    },
    /// Add a non-scheduling link.
    AddLink {
        /// The generic link.
        link: GenericLink,
    },
    /// Remove a non-scheduling link by id.
    RemoveLink {
        /// The link id.
        id: LinkId,
    },

    // ── resources / assignments ──────────────────────────────────────────────────
    /// Create or replace a resource.
    UpsertResource {
        /// The resource.
        resource: Resource,
    },
    /// Delete a resource and its assignments.
    DeleteResource {
        /// The resource id.
        id: ResourceId,
    },
    /// Assign a resource to a task (replaces an existing assignment of the same pair).
    Assign {
        /// The assignment.
        assignment: Assignment,
    },
    /// Remove an assignment.
    Unassign {
        /// The task.
        task: TaskId,
        /// The resource.
        resource: ResourceId,
    },

    // ── calendars ────────────────────────────────────────────────────────────────
    /// Create or replace a calendar (day schedules with invalid intervals are
    /// rejected).
    UpsertCalendar {
        /// The calendar.
        calendar: Calendar,
    },
    /// Set the project's default calendar (must exist).
    SetProjectCalendar {
        /// The calendar id.
        id: CalendarId,
    },
    /// Add a dated exception to a calendar (rejected if its intervals are invalid).
    AddException {
        /// The calendar.
        calendar: CalendarId,
        /// The exception.
        exception: CalendarException,
    },

    // ── fields ───────────────────────────────────────────────────────────────────
    /// Create or replace a custom field definition.
    UpsertFieldDef {
        /// The field definition.
        field: FieldDef,
    },
    /// Delete a custom field and its stored values.
    DeleteFieldDef {
        /// The field id.
        id: FieldId,
    },
    /// Set or clear a task's value for a field.
    SetFieldValue {
        /// The task.
        task: TaskId,
        /// The field.
        field: FieldId,
        /// The value, or `None` to clear.
        value: Option<FieldValue>,
    },

    // ── decisions (checklist) ────────────────────────────────────────────────────
    /// Set or clear a task's decision (branch point).
    SetDecision {
        /// The task.
        id: TaskId,
        /// The decision, or `None` to clear.
        decision: Option<Decision>,
    },
    /// Answer a task's decision.
    AnswerDecision {
        /// The task.
        id: TaskId,
        /// The answer.
        answer: bool,
    },

    // ── baselines / views ────────────────────────────────────────────────────────
    /// Capture a baseline of the current task durations and work.
    CaptureBaseline {
        /// The baseline id.
        id: BaselineId,
        /// A display name.
        name: String,
        /// The capture time (injected epoch millis).
        now: u64,
    },
    /// Delete a baseline.
    DeleteBaseline {
        /// The baseline id.
        id: BaselineId,
    },
    /// Create or replace a saved view.
    UpsertView {
        /// The view.
        view: View,
    },
    /// Delete a saved view.
    DeleteView {
        /// The view id.
        id: ViewId,
    },
}

/// Apply `cmd` to `state`, returning the new state. Pure and total: invalid commands
/// are no-ops (or clamped), never errors.
pub fn reduce(state: &ProjectState, cmd: TaskCommand) -> ProjectState {
    let mut s = state.clone();
    match cmd {
        TaskCommand::SetProjectName { name } => s.name = name,
        TaskCommand::LoadState { state } => return *state,

        TaskCommand::CreateTask { id, name, parent } => {
            // Ignore a parent that doesn't exist; ignore a duplicate id (use the entry
            // API so we test-and-insert in one lookup).
            let parent = parent.filter(|p| s.tasks.contains_key(p));
            if let std::collections::btree_map::Entry::Vacant(slot) = s.tasks.entry(id) {
                let mut t = Task::new(slot.key().clone(), name);
                t.parent = parent;
                slot.insert(t);
            }
        }
        TaskCommand::RenameTask { id, name } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                t.name = name;
            }
        }
        TaskCommand::SetNotes { id, notes } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                t.notes = notes;
            }
        }
        TaskCommand::DeleteTask { id } => {
            if let Some(removed) = s.tasks.remove(&id) {
                // Reparent orphaned children to the deleted task's parent.
                for t in s.tasks.values_mut() {
                    if t.parent.as_ref() == Some(&id) {
                        t.parent = removed.parent.clone();
                    }
                }
                s.dependencies
                    .retain(|d| d.predecessor != id && d.successor != id);
                s.links.retain(|l| l.from != id && l.to != id);
                s.assignments.retain(|a| a.task != id);
            }
        }
        TaskCommand::Reparent { id, new_parent } => {
            let ok = match &new_parent {
                None => true,
                Some(p) => s.tasks.contains_key(p) && *p != id && !is_ancestor(&s, &id, p),
            };
            if ok {
                if let Some(t) = s.tasks.get_mut(&id) {
                    t.parent = new_parent;
                }
            }
        }
        TaskCommand::SetOrder { id, order } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                t.order = order;
            }
        }
        TaskCommand::SetKind { id, kind } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                t.kind = kind;
            }
        }
        TaskCommand::ToggleCollapsed { id } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                t.collapsed = !t.collapsed;
            }
        }

        TaskCommand::SetStatus { id, status } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                t.status = status;
            }
        }
        TaskCommand::SetCompleted { id, completed } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                t.completed = completed;
            }
        }
        TaskCommand::SetPercentComplete { id, percent } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                t.percent_complete = percent.min(100); // clamp — the trust boundary
            }
        }

        TaskCommand::SetSchedule { id, schedule } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                t.schedule = schedule.map(|b| *b);
            }
        }
        TaskCommand::SetDuration { id, duration } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                t.schedule
                    .get_or_insert_with(TaskSchedule::default)
                    .duration = duration;
            }
        }
        TaskCommand::SetConstraint { id, constraint } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                t.schedule
                    .get_or_insert_with(TaskSchedule::default)
                    .constraint = constraint;
            }
        }
        TaskCommand::SetDeadline { id, deadline } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                t.schedule
                    .get_or_insert_with(TaskSchedule::default)
                    .deadline = deadline;
            }
        }

        TaskCommand::LinkDependency { link } => {
            let valid = link.predecessor != link.successor
                && s.tasks.contains_key(&link.predecessor)
                && s.tasks.contains_key(&link.successor)
                && !s
                    .dependencies
                    .iter()
                    .any(|d| d.predecessor == link.predecessor && d.successor == link.successor)
                && !would_cycle(&s, &link);
            if valid {
                s.dependencies.push(link);
            }
        }
        TaskCommand::UnlinkDependency { id } => s.dependencies.retain(|d| d.id != id),
        TaskCommand::AddLink { link } => {
            if s.tasks.contains_key(&link.from) && s.tasks.contains_key(&link.to) {
                s.links.push(link);
            }
        }
        TaskCommand::RemoveLink { id } => s.links.retain(|l| l.id != id),

        TaskCommand::UpsertResource { resource } => {
            s.resources.insert(resource.id.clone(), resource);
        }
        TaskCommand::DeleteResource { id } => {
            s.resources.remove(&id);
            s.assignments.retain(|a| a.resource != id);
        }
        TaskCommand::Assign { assignment } => {
            if s.tasks.contains_key(&assignment.task)
                && s.resources.contains_key(&assignment.resource)
            {
                s.assignments
                    .retain(|a| !(a.task == assignment.task && a.resource == assignment.resource));
                s.assignments.push(assignment);
            }
        }
        TaskCommand::Unassign { task, resource } => {
            s.assignments
                .retain(|a| !(a.task == task && a.resource == resource));
        }

        TaskCommand::UpsertCalendar { calendar } => {
            if calendar.work_week.iter().all(valid_day_schedule)
                && calendar
                    .exceptions
                    .iter()
                    .all(|e| valid_day_schedule(&e.schedule))
            {
                s.calendars.insert(calendar.id.clone(), calendar);
            }
        }
        TaskCommand::SetProjectCalendar { id } => {
            if s.calendars.contains_key(&id) {
                s.project_calendar = id;
            }
        }
        TaskCommand::AddException {
            calendar,
            exception,
        } => {
            if valid_day_schedule(&exception.schedule) {
                if let Some(c) = s.calendars.get_mut(&calendar) {
                    c.exceptions.retain(|e| e.date != exception.date);
                    c.exceptions.push(exception);
                }
            }
        }

        TaskCommand::UpsertFieldDef { field } => {
            s.fields.insert(field.id.clone(), field);
        }
        TaskCommand::DeleteFieldDef { id } => {
            s.fields.remove(&id);
            for t in s.tasks.values_mut() {
                t.fields.remove(&id);
            }
        }
        TaskCommand::SetFieldValue { task, field, value } => {
            if s.fields.contains_key(&field) {
                if let Some(t) = s.tasks.get_mut(&task) {
                    match value {
                        Some(v) => {
                            t.fields.insert(field, v);
                        }
                        None => {
                            t.fields.remove(&field);
                        }
                    }
                }
            }
        }

        TaskCommand::SetDecision { id, decision } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                t.decision = decision;
            }
        }
        TaskCommand::AnswerDecision { id, answer } => {
            if let Some(t) = s.tasks.get_mut(&id) {
                if let Some(d) = t.decision.as_mut() {
                    d.answer = Some(answer);
                }
            }
        }

        TaskCommand::CaptureBaseline { id, name, now } => {
            let tasks: BTreeMap<TaskId, BaselineTask> = s
                .tasks
                .iter()
                .filter_map(|(tid, t)| {
                    t.schedule.as_ref().map(|sc| {
                        (
                            tid.clone(),
                            BaselineTask {
                                start: sc.actual_start,
                                finish: sc.actual_finish,
                                duration: sc.duration,
                                work: sc.work,
                            },
                        )
                    })
                })
                .collect();
            s.baselines.insert(
                id.clone(),
                Baseline {
                    id,
                    name,
                    captured_at: now,
                    tasks,
                },
            );
        }
        TaskCommand::DeleteBaseline { id } => {
            s.baselines.remove(&id);
        }
        TaskCommand::UpsertView { view } => {
            s.views.insert(view.id.clone(), view);
        }
        TaskCommand::DeleteView { id } => {
            s.views.remove(&id);
        }
    }
    s
}

// ── validation helpers ───────────────────────────────────────────────────────────

/// A day schedule is valid when every interval is well-formed (`start < end <= 1440`)
/// and there are not pathologically many of them.
fn valid_day_schedule(sched: &DaySchedule) -> bool {
    sched.intervals.len() <= MAX_INTERVALS_PER_DAY
        && sched
            .intervals
            .iter()
            .all(|iv| iv.start_min < iv.end_min && iv.end_min <= 1440)
}

/// Whether `ancestor` appears on the parent chain of `task` (so making `task` the
/// parent of `ancestor` would create a cycle). Bounded by the task count.
fn is_ancestor(state: &ProjectState, ancestor: &TaskId, task: &TaskId) -> bool {
    let mut cur = state.tasks.get(task).and_then(|t| t.parent.clone());
    let mut guard = 0;
    while let Some(p) = cur {
        if &p == ancestor {
            return true;
        }
        cur = state.tasks.get(&p).and_then(|t| t.parent.clone());
        guard += 1;
        if guard > state.tasks.len() {
            break;
        }
    }
    false
}

/// Whether adding `link` would introduce a cycle in the dependency network. Reuses
/// `directed-graph`'s cycle detector over the existing edges plus the candidate.
fn would_cycle(state: &ProjectState, link: &DependencyLink) -> bool {
    let mut g = directed_graph::Graph::new();
    for t in state.tasks.keys() {
        g.add_node(t.as_str());
    }
    for d in &state.dependencies {
        let _ = g.add_edge(d.predecessor.as_str(), d.successor.as_str());
    }
    let _ = g.add_edge(link.predecessor.as_str(), link.successor.as_str());
    g.has_cycle()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> ProjectState {
        ProjectState::empty(ProjectId::from_raw("p1"))
    }
    fn tid(s: &str) -> TaskId {
        TaskId::from_raw(s)
    }

    fn with_tasks(ids: &[&str]) -> ProjectState {
        let mut s = base();
        for id in ids {
            s = reduce(
                &s,
                TaskCommand::CreateTask {
                    id: tid(id),
                    name: id.to_string(),
                    parent: None,
                },
            );
        }
        s
    }

    #[test]
    fn reduce_is_immutable() {
        let s0 = base();
        let s1 = reduce(
            &s0,
            TaskCommand::CreateTask {
                id: tid("a"),
                name: "A".into(),
                parent: None,
            },
        );
        assert_eq!(s0.tasks.len(), 0, "input state is untouched");
        assert_eq!(s1.tasks.len(), 1);
    }

    #[test]
    fn create_rename_delete_task() {
        let mut s = with_tasks(&["a", "b"]);
        s = reduce(
            &s,
            TaskCommand::RenameTask {
                id: tid("a"),
                name: "Alpha".into(),
            },
        );
        assert_eq!(s.tasks[&tid("a")].name, "Alpha");
        s = reduce(&s, TaskCommand::DeleteTask { id: tid("a") });
        assert!(!s.tasks.contains_key(&tid("a")));
        assert!(s.tasks.contains_key(&tid("b")));
    }

    #[test]
    fn percent_complete_is_clamped() {
        let mut s = with_tasks(&["a"]);
        s = reduce(
            &s,
            TaskCommand::SetPercentComplete {
                id: tid("a"),
                percent: 250,
            },
        );
        assert_eq!(s.tasks[&tid("a")].percent_complete, 100);
    }

    #[test]
    fn reparent_rejects_cycles() {
        // a → b (b under a). Trying to put a under b must be rejected.
        let mut s = with_tasks(&["a", "b"]);
        s = reduce(
            &s,
            TaskCommand::Reparent {
                id: tid("b"),
                new_parent: Some(tid("a")),
            },
        );
        assert_eq!(s.tasks[&tid("b")].parent, Some(tid("a")));
        let before = s.clone();
        s = reduce(
            &s,
            TaskCommand::Reparent {
                id: tid("a"),
                new_parent: Some(tid("b")),
            },
        );
        assert_eq!(s, before, "cycle-forming reparent is a no-op");
    }

    #[test]
    fn dependency_rejects_self_duplicate_and_cycle() {
        let s = with_tasks(&["a", "b"]);
        let mk = |id: &str, p: &str, q: &str| DependencyLink {
            id: LinkId::from_raw(id),
            predecessor: tid(p),
            successor: tid(q),
            kind: DependencyKind::FinishToStart,
            lag: Duration::zero(),
        };
        // self-link rejected
        let s1 = reduce(
            &s,
            TaskCommand::LinkDependency {
                link: mk("l0", "a", "a"),
            },
        );
        assert_eq!(s1.dependencies.len(), 0);
        // valid link accepted
        let s2 = reduce(
            &s,
            TaskCommand::LinkDependency {
                link: mk("l1", "a", "b"),
            },
        );
        assert_eq!(s2.dependencies.len(), 1);
        // duplicate rejected
        let s3 = reduce(
            &s2,
            TaskCommand::LinkDependency {
                link: mk("l2", "a", "b"),
            },
        );
        assert_eq!(s3.dependencies.len(), 1);
        // cycle-forming link rejected
        let s4 = reduce(
            &s2,
            TaskCommand::LinkDependency {
                link: mk("l3", "b", "a"),
            },
        );
        assert_eq!(s4.dependencies.len(), 1, "b→a would cycle with a→b");
    }

    #[test]
    fn calendar_exception_with_bad_interval_is_rejected() {
        let mut s = with_tasks(&["a"]);
        let cal = s.project_calendar.clone();
        let bad = CalendarException {
            date: Date::from_ymd(2026, 7, 10).unwrap(),
            schedule: DaySchedule {
                working: true,
                // start >= end is invalid
                intervals: vec![MinuteInterval {
                    start_min: 600,
                    end_min: 500,
                }],
            },
        };
        let before = s.clone();
        s = reduce(
            &s,
            TaskCommand::AddException {
                calendar: cal,
                exception: bad,
            },
        );
        assert_eq!(s, before, "invalid interval rejected");
    }

    #[test]
    fn set_duration_creates_a_schedule_block() {
        let mut s = with_tasks(&["a"]);
        assert!(s.tasks[&tid("a")].schedule.is_none());
        s = reduce(
            &s,
            TaskCommand::SetDuration {
                id: tid("a"),
                duration: Duration::minutes(480),
            },
        );
        assert_eq!(
            s.tasks[&tid("a")].schedule.as_ref().unwrap().duration,
            Duration::minutes(480)
        );
    }

    #[test]
    fn assign_and_unassign_replace_by_pair() {
        let mut s = with_tasks(&["a"]);
        s = reduce(
            &s,
            TaskCommand::UpsertResource {
                resource: Resource {
                    id: ResourceId::from_raw("r1"),
                    name: "Dev".into(),
                    kind: ResourceKind::Work,
                    calendar: None,
                    max_units: 1.0,
                    std_rate: crate::primitives::Money::zero("USD"),
                    cost_per_use: crate::primitives::Money::zero("USD"),
                },
            },
        );
        let asn = |units: f64| Assignment {
            task: tid("a"),
            resource: ResourceId::from_raw("r1"),
            units,
            work: crate::primitives::Work::minutes(480),
            contour: WorkContour::Flat,
        };
        s = reduce(
            &s,
            TaskCommand::Assign {
                assignment: asn(1.0),
            },
        );
        s = reduce(
            &s,
            TaskCommand::Assign {
                assignment: asn(0.5),
            },
        );
        assert_eq!(s.assignments.len(), 1, "same pair replaces");
        assert_eq!(s.assignments[0].units, 0.5);
        s = reduce(
            &s,
            TaskCommand::Unassign {
                task: tid("a"),
                resource: ResourceId::from_raw("r1"),
            },
        );
        assert!(s.assignments.is_empty());
    }

    #[test]
    fn answer_decision_records_the_answer() {
        let mut s = with_tasks(&["a"]);
        s = reduce(
            &s,
            TaskCommand::SetDecision {
                id: tid("a"),
                decision: Some(Decision {
                    question: "Ready?".into(),
                    answer: None,
                    yes_children: vec![],
                    no_children: vec![],
                }),
            },
        );
        s = reduce(
            &s,
            TaskCommand::AnswerDecision {
                id: tid("a"),
                answer: true,
            },
        );
        assert_eq!(
            s.tasks[&tid("a")].decision.as_ref().unwrap().answer,
            Some(true)
        );
    }
}
