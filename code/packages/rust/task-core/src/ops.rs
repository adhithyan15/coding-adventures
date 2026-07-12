//! The pure operations API — every mutation as a validated method on `ProjectState`.
//!
//! There is **no command enum and no dispatch loop** (those are Flux/React idioms we
//! deliberately avoid — see `task-app-architecture.md`). A mutation is an ordinary
//! method that mutates the value in place and returns `Result<(), OpError>`. Methods
//! are *pure*: deterministic, no I/O, no globals, no clock. A backend that wants value
//! semantics simply `clone()`s first.
//!
//! This is the **single trust boundary** for the model's invariants — the validation
//! (percent clamping, calendar interval bounds, reparent- and dependency-cycle
//! rejection) lives here, so every platform gets it for free by calling the engine.

use crate::ids::*;
use crate::model::*;
use crate::primitives::{Date, Duration};
use std::collections::BTreeMap;

/// The largest number of working intervals accepted on a single day — a real day has
/// a handful; capping defends the working-time walk against a crafted calendar.
const MAX_INTERVALS_PER_DAY: usize = 48;

/// Why an operation was rejected. Operations never panic and never do I/O; an invalid
/// request is a typed error, not a crash.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum OpError {
    /// A referenced entity (task, resource, calendar, field) does not exist.
    NotFound,
    /// The entity already exists (e.g. a duplicate id or dependency).
    Duplicate,
    /// The operation would create a cycle (task hierarchy or dependency network).
    WouldCycle,
    /// The input violated a constraint, with a short reason.
    Invalid(&'static str),
}

impl OpError {
    /// A stable integer code for the C ABI (0 = success is returned by the ABI, not here).
    pub fn code(&self) -> i32 {
        match self {
            OpError::NotFound => 1,
            OpError::Duplicate => 2,
            OpError::WouldCycle => 3,
            OpError::Invalid(_) => 4,
        }
    }
}

impl ProjectState {
    // ── project ──────────────────────────────────────────────────────────────────

    /// Rename the whole project.
    pub fn set_project_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    // ── tasks ────────────────────────────────────────────────────────────────────

    /// Create a leaf task. Rejects a duplicate id or a non-existent parent.
    pub fn create_task(
        &mut self,
        id: TaskId,
        name: impl Into<String>,
        parent: Option<TaskId>,
    ) -> Result<(), OpError> {
        if self.tasks.contains_key(&id) {
            return Err(OpError::Duplicate);
        }
        if let Some(p) = &parent {
            if !self.tasks.contains_key(p) {
                return Err(OpError::NotFound);
            }
        }
        let mut t = Task::new(id.clone(), name);
        t.parent = parent;
        self.tasks.insert(id, t);
        Ok(())
    }

    /// Rename a task.
    pub fn rename_task(&mut self, id: &TaskId, name: impl Into<String>) -> Result<(), OpError> {
        self.task_mut(id)?.name = name.into();
        Ok(())
    }

    /// Set a task's notes.
    pub fn set_notes(&mut self, id: &TaskId, notes: impl Into<String>) -> Result<(), OpError> {
        self.task_mut(id)?.notes = notes.into();
        Ok(())
    }

    /// Delete a task; its children are reparented to its parent, and links,
    /// dependencies, and assignments referencing it are removed.
    pub fn delete_task(&mut self, id: &TaskId) -> Result<(), OpError> {
        let removed = self.tasks.remove(id).ok_or(OpError::NotFound)?;
        for t in self.tasks.values_mut() {
            if t.parent.as_ref() == Some(id) {
                t.parent = removed.parent.clone();
            }
        }
        self.dependencies
            .retain(|d| &d.predecessor != id && &d.successor != id);
        self.links.retain(|l| &l.from != id && &l.to != id);
        self.assignments.retain(|a| &a.task != id);
        Ok(())
    }

    /// Move a task under a new parent. Rejects a missing target or a cycle.
    pub fn reparent(&mut self, id: &TaskId, new_parent: Option<TaskId>) -> Result<(), OpError> {
        if !self.tasks.contains_key(id) {
            return Err(OpError::NotFound);
        }
        if let Some(p) = &new_parent {
            if !self.tasks.contains_key(p) {
                return Err(OpError::NotFound);
            }
            if p == id || is_ancestor(self, id, p) {
                return Err(OpError::WouldCycle);
            }
        }
        self.tasks.get_mut(id).unwrap().parent = new_parent;
        Ok(())
    }

    /// Set a task's sibling ordering key.
    pub fn set_order(&mut self, id: &TaskId, order: i64) -> Result<(), OpError> {
        self.task_mut(id)?.order = order;
        Ok(())
    }

    /// Set a task's kind (leaf/summary/milestone).
    pub fn set_kind(&mut self, id: &TaskId, kind: TaskKind) -> Result<(), OpError> {
        self.task_mut(id)?.kind = kind;
        Ok(())
    }

    /// Toggle a summary's collapsed state.
    pub fn toggle_collapsed(&mut self, id: &TaskId) -> Result<(), OpError> {
        let t = self.task_mut(id)?;
        t.collapsed = !t.collapsed;
        Ok(())
    }

    // ── progress / workflow ──────────────────────────────────────────────────────

    /// Set a task's workflow status.
    pub fn set_status(&mut self, id: &TaskId, status: Option<StatusId>) -> Result<(), OpError> {
        self.task_mut(id)?.status = status;
        Ok(())
    }

    /// Set a task's completion flag.
    pub fn set_completed(&mut self, id: &TaskId, completed: bool) -> Result<(), OpError> {
        self.task_mut(id)?.completed = completed;
        Ok(())
    }

    /// Set percent complete, **clamped to 0..=100**.
    pub fn set_percent_complete(&mut self, id: &TaskId, pct: u8) -> Result<(), OpError> {
        self.task_mut(id)?.percent_complete = pct.min(100);
        Ok(())
    }

    // ── scheduling ───────────────────────────────────────────────────────────────

    /// Set or clear a task's whole scheduling block.
    pub fn set_schedule(
        &mut self,
        id: &TaskId,
        schedule: Option<TaskSchedule>,
    ) -> Result<(), OpError> {
        self.task_mut(id)?.schedule = schedule;
        Ok(())
    }

    /// Set a task's duration (creating a default schedule block if absent).
    pub fn set_duration(&mut self, id: &TaskId, duration: Duration) -> Result<(), OpError> {
        self.task_mut(id)?
            .schedule
            .get_or_insert_with(TaskSchedule::default)
            .duration = duration;
        Ok(())
    }

    /// Set a task's date constraint (creating a schedule block if absent).
    pub fn set_constraint(&mut self, id: &TaskId, constraint: Constraint) -> Result<(), OpError> {
        self.task_mut(id)?
            .schedule
            .get_or_insert_with(TaskSchedule::default)
            .constraint = constraint;
        Ok(())
    }

    /// Set or clear a task's deadline (creating a schedule block if absent).
    pub fn set_deadline(&mut self, id: &TaskId, deadline: Option<Date>) -> Result<(), OpError> {
        self.task_mut(id)?
            .schedule
            .get_or_insert_with(TaskSchedule::default)
            .deadline = deadline;
        Ok(())
    }

    // ── relations ────────────────────────────────────────────────────────────────

    /// Add a dependency. Rejects a self-link, a duplicate, unknown endpoints, or a
    /// link that would make the dependency network cyclic.
    pub fn link_dependency(&mut self, link: DependencyLink) -> Result<(), OpError> {
        if link.predecessor == link.successor {
            return Err(OpError::Invalid("self-dependency"));
        }
        if !self.tasks.contains_key(&link.predecessor) || !self.tasks.contains_key(&link.successor)
        {
            return Err(OpError::NotFound);
        }
        if self
            .dependencies
            .iter()
            .any(|d| d.predecessor == link.predecessor && d.successor == link.successor)
        {
            return Err(OpError::Duplicate);
        }
        if would_cycle(self, &link) {
            return Err(OpError::WouldCycle);
        }
        self.dependencies.push(link);
        Ok(())
    }

    /// Remove a dependency by id.
    pub fn unlink_dependency(&mut self, id: &LinkId) {
        self.dependencies.retain(|d| &d.id != id);
    }

    /// Add a non-scheduling link. Rejects unknown endpoints.
    pub fn add_link(&mut self, link: GenericLink) -> Result<(), OpError> {
        if !self.tasks.contains_key(&link.from) || !self.tasks.contains_key(&link.to) {
            return Err(OpError::NotFound);
        }
        self.links.push(link);
        Ok(())
    }

    /// Remove a non-scheduling link by id.
    pub fn remove_link(&mut self, id: &LinkId) {
        self.links.retain(|l| &l.id != id);
    }

    // ── resources / assignments ──────────────────────────────────────────────────

    /// Create or replace a resource.
    pub fn upsert_resource(&mut self, resource: Resource) {
        self.resources.insert(resource.id.clone(), resource);
    }

    /// Delete a resource and its assignments.
    pub fn delete_resource(&mut self, id: &ResourceId) {
        self.resources.remove(id);
        self.assignments.retain(|a| &a.resource != id);
    }

    /// Assign a resource to a task (replacing an existing assignment of the same pair).
    /// Rejects unknown task or resource.
    pub fn assign(&mut self, assignment: Assignment) -> Result<(), OpError> {
        if !self.tasks.contains_key(&assignment.task)
            || !self.resources.contains_key(&assignment.resource)
        {
            return Err(OpError::NotFound);
        }
        self.assignments
            .retain(|a| !(a.task == assignment.task && a.resource == assignment.resource));
        self.assignments.push(assignment);
        Ok(())
    }

    /// Remove an assignment.
    pub fn unassign(&mut self, task: &TaskId, resource: &ResourceId) {
        self.assignments
            .retain(|a| !(&a.task == task && &a.resource == resource));
    }

    // ── calendars ────────────────────────────────────────────────────────────────

    /// Create or replace a calendar. Rejects invalid day schedules.
    pub fn upsert_calendar(&mut self, calendar: Calendar) -> Result<(), OpError> {
        if !calendar.work_week.iter().all(valid_day_schedule)
            || !calendar
                .exceptions
                .iter()
                .all(|e| valid_day_schedule(&e.schedule))
        {
            return Err(OpError::Invalid("invalid calendar interval"));
        }
        self.calendars.insert(calendar.id.clone(), calendar);
        Ok(())
    }

    /// Set the project's default calendar (must exist).
    pub fn set_project_calendar(&mut self, id: CalendarId) -> Result<(), OpError> {
        if !self.calendars.contains_key(&id) {
            return Err(OpError::NotFound);
        }
        self.project_calendar = id;
        Ok(())
    }

    /// Add a dated exception to a calendar. Rejects invalid intervals or unknown calendar.
    pub fn add_calendar_exception(
        &mut self,
        cal: &CalendarId,
        exception: CalendarException,
    ) -> Result<(), OpError> {
        if !valid_day_schedule(&exception.schedule) {
            return Err(OpError::Invalid("invalid calendar interval"));
        }
        let c = self.calendars.get_mut(cal).ok_or(OpError::NotFound)?;
        c.exceptions.retain(|e| e.date != exception.date);
        c.exceptions.push(exception);
        Ok(())
    }

    // ── fields ───────────────────────────────────────────────────────────────────

    /// Create or replace a custom field definition.
    pub fn upsert_field(&mut self, field: FieldDef) {
        self.fields.insert(field.id.clone(), field);
    }

    /// Delete a custom field and its stored values.
    pub fn delete_field(&mut self, id: &FieldId) {
        self.fields.remove(id);
        for t in self.tasks.values_mut() {
            t.fields.remove(id);
        }
    }

    /// Set or clear a task's value for a field. Rejects unknown field or task.
    pub fn set_field_value(
        &mut self,
        task: &TaskId,
        field: &FieldId,
        value: Option<FieldValue>,
    ) -> Result<(), OpError> {
        if !self.fields.contains_key(field) {
            return Err(OpError::NotFound);
        }
        let t = self.task_mut(task)?;
        match value {
            Some(v) => {
                t.fields.insert(field.clone(), v);
            }
            None => {
                t.fields.remove(field);
            }
        }
        Ok(())
    }

    // ── decisions ────────────────────────────────────────────────────────────────

    /// Set or clear a task's decision (branch point).
    pub fn set_decision(&mut self, id: &TaskId, decision: Option<Decision>) -> Result<(), OpError> {
        self.task_mut(id)?.decision = decision;
        Ok(())
    }

    /// Answer a task's decision. Rejects a task with no decision.
    pub fn answer_decision(&mut self, id: &TaskId, answer: bool) -> Result<(), OpError> {
        let d = self
            .task_mut(id)?
            .decision
            .as_mut()
            .ok_or(OpError::Invalid("task has no decision"))?;
        d.answer = Some(answer);
        Ok(())
    }

    // ── baselines / views ────────────────────────────────────────────────────────

    /// Capture a baseline of the current task durations and work.
    pub fn capture_baseline(&mut self, id: BaselineId, name: String, now: u64) {
        let tasks: BTreeMap<TaskId, BaselineTask> = self
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
        self.baselines.insert(
            id.clone(),
            Baseline {
                id,
                name,
                captured_at: now,
                tasks,
            },
        );
    }

    /// Delete a baseline.
    pub fn delete_baseline(&mut self, id: &BaselineId) {
        self.baselines.remove(id);
    }

    /// Create or replace a saved view.
    pub fn upsert_view(&mut self, view: View) {
        self.views.insert(view.id.clone(), view);
    }

    /// Delete a saved view.
    pub fn delete_view(&mut self, id: &ViewId) {
        self.views.remove(id);
    }

    // ── internal ─────────────────────────────────────────────────────────────────

    fn task_mut(&mut self, id: &TaskId) -> Result<&mut Task, OpError> {
        self.tasks.get_mut(id).ok_or(OpError::NotFound)
    }
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

    #[test]
    fn create_reject_duplicate_and_bad_parent() {
        let mut s = base();
        assert!(s.create_task(tid("a"), "A", None).is_ok());
        assert_eq!(s.create_task(tid("a"), "A2", None), Err(OpError::Duplicate));
        assert_eq!(
            s.create_task(tid("b"), "B", Some(tid("missing"))),
            Err(OpError::NotFound)
        );
        assert!(s.create_task(tid("b"), "B", Some(tid("a"))).is_ok());
    }

    #[test]
    fn rename_missing_is_not_found() {
        let mut s = base();
        assert_eq!(s.rename_task(&tid("x"), "X"), Err(OpError::NotFound));
    }

    #[test]
    fn percent_complete_is_clamped() {
        let mut s = base();
        s.create_task(tid("a"), "A", None).unwrap();
        s.set_percent_complete(&tid("a"), 250).unwrap();
        assert_eq!(s.tasks[&tid("a")].percent_complete, 100);
    }

    #[test]
    fn reparent_rejects_cycles() {
        let mut s = base();
        s.create_task(tid("a"), "A", None).unwrap();
        s.create_task(tid("b"), "B", Some(tid("a"))).unwrap(); // b under a
        assert_eq!(
            s.reparent(&tid("a"), Some(tid("b"))),
            Err(OpError::WouldCycle)
        );
        assert_eq!(
            s.reparent(&tid("a"), Some(tid("a"))),
            Err(OpError::WouldCycle)
        );
    }

    #[test]
    fn delete_reparents_children_and_prunes_links() {
        let mut s = base();
        s.create_task(tid("a"), "A", None).unwrap();
        s.create_task(tid("b"), "B", Some(tid("a"))).unwrap();
        s.create_task(tid("c"), "C", None).unwrap();
        s.link_dependency(DependencyLink {
            id: LinkId::from_raw("l1"),
            predecessor: tid("a"),
            successor: tid("c"),
            kind: DependencyKind::FinishToStart,
            lag: Duration::zero(),
        })
        .unwrap();
        s.delete_task(&tid("a")).unwrap();
        assert!(!s.tasks.contains_key(&tid("a")));
        assert_eq!(
            s.tasks[&tid("b")].parent,
            None,
            "child reparented to grandparent"
        );
        assert!(s.dependencies.is_empty(), "dangling dependency pruned");
    }

    #[test]
    fn dependency_rejects_self_dup_and_cycle() {
        let mut s = base();
        s.create_task(tid("a"), "A", None).unwrap();
        s.create_task(tid("b"), "B", None).unwrap();
        let mk = |id: &str, p: &str, q: &str| DependencyLink {
            id: LinkId::from_raw(id),
            predecessor: tid(p),
            successor: tid(q),
            kind: DependencyKind::FinishToStart,
            lag: Duration::zero(),
        };
        assert_eq!(
            s.link_dependency(mk("l0", "a", "a")),
            Err(OpError::Invalid("self-dependency"))
        );
        assert!(s.link_dependency(mk("l1", "a", "b")).is_ok());
        assert_eq!(
            s.link_dependency(mk("l2", "a", "b")),
            Err(OpError::Duplicate)
        );
        assert_eq!(
            s.link_dependency(mk("l3", "b", "a")),
            Err(OpError::WouldCycle)
        );
    }

    #[test]
    fn calendar_exception_with_bad_interval_is_rejected() {
        let mut s = base();
        let cal = s.project_calendar.clone();
        let bad = CalendarException {
            date: Date::from_ymd(2026, 7, 10).unwrap(),
            schedule: DaySchedule {
                working: true,
                intervals: vec![MinuteInterval {
                    start_min: 600,
                    end_min: 500,
                }],
            },
        };
        assert_eq!(
            s.add_calendar_exception(&cal, bad),
            Err(OpError::Invalid("invalid calendar interval"))
        );
    }

    #[test]
    fn assign_replaces_by_pair_and_needs_known_entities() {
        let mut s = base();
        s.create_task(tid("a"), "A", None).unwrap();
        let asn = |units: f64| Assignment {
            task: tid("a"),
            resource: ResourceId::from_raw("r1"),
            units,
            work: crate::primitives::Work::minutes(480),
            contour: WorkContour::Flat,
        };
        assert_eq!(
            s.assign(asn(1.0)),
            Err(OpError::NotFound),
            "unknown resource"
        );
        s.upsert_resource(Resource {
            id: ResourceId::from_raw("r1"),
            name: "Dev".into(),
            kind: ResourceKind::Work,
            calendar: None,
            max_units: 1.0,
            std_rate: crate::primitives::Money::zero("USD"),
            cost_per_use: crate::primitives::Money::zero("USD"),
        });
        s.assign(asn(1.0)).unwrap();
        s.assign(asn(0.5)).unwrap();
        assert_eq!(s.assignments.len(), 1);
        assert_eq!(s.assignments[0].units, 0.5);
    }

    #[test]
    fn methods_are_pure_no_hidden_state() {
        // Two independent projects don't interfere — the "engine" holds no globals.
        let mut a = base();
        let b = base();
        a.create_task(tid("x"), "X", None).unwrap();
        assert_eq!(a.tasks.len(), 1);
        assert_eq!(b.tasks.len(), 0);
    }
}
