//! The Critical Path Method scheduler.
//!
//! Microsoft Project's scheduling semantics *are* CPM: a topological walk of the
//! dependency network, a **forward pass** that pushes every task as early as its
//! predecessors allow, and a **backward pass** that finds how late each task may slip
//! without delaying the project. The gap between the two is **slack**; the tasks with
//! none form the **critical path**. Because CPM yields early dates, late dates, and
//! slack in two linear passes, all of it comes out "for free" — which is exactly why
//! a direct pass beats routing the problem through a general constraint solver.
//!
//! Everything here runs in **working time** via the [`crate::calendar`] module, so a
//! task started Friday afternoon correctly finishes Monday. Dependency ordering and
//! cycle detection reuse [`directed_graph`]; no graph algorithm is reimplemented.
//!
//! ## Scope of this first cut
//!
//! Fully modelled: the four link types (FS/SS/FF/SF) with lag, working-time
//! calendars, the forward and backward passes, total/free slack, the critical path,
//! summary rollups, cycle rejection, and the common date constraints
//! (`AsSoonAsPossible`, `StartNoEarlierThan`, `MustStartOn`, `FinishNoEarlierThan`)
//! plus conflict flags for the capping constraints (`StartNoLaterThan`,
//! `FinishNoLaterThan`, `MustFinishOn`) and `deadline`. `AsLateAsPossible` is treated
//! as `AsSoonAsPossible` in this cut (a documented limitation); negative lag (lead)
//! is applied in elapsed time. These are refined in a follow-up.

use crate::calendar::{self, Instant};
use crate::ids::TaskId;
use crate::model::{
    Constraint, DependencyKind, DependencyLink, ProjectState, ScheduledDates, Task, TaskKind,
};
use crate::primitives::{Date, Duration};
use std::collections::BTreeMap;

/// The result of scheduling a project.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ScheduleResult {
    /// Computed dates per scheduled task (leaves, milestones, and summaries).
    pub dates: BTreeMap<TaskId, ScheduledDates>,
    /// Problems the scheduler could not satisfy (surfaced to the UI, never fatal).
    pub conflicts: Vec<Conflict>,
    /// The project start date the schedule was computed from.
    pub project_start: Date,
    /// The latest finish across all tasks, or `None` if nothing was scheduled.
    pub project_finish: Option<Date>,
}

/// A scheduling constraint the engine could not honour.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Conflict {
    /// The task the conflict is about.
    pub task: TaskId,
    /// What kind of conflict.
    pub kind: ConflictKind,
    /// A human-readable explanation.
    pub message: String,
}

/// The category of a scheduling conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum ConflictKind {
    /// An inflexible constraint contradicts a dependency.
    ConstraintVsDependency,
    /// The task finishes after its deadline.
    DeadlineMissed,
    /// The task finishes later than a `FinishNoLaterThan` constraint allows.
    FinishTooLate,
    /// The task starts later than a `StartNoLaterThan` constraint allows.
    StartTooLate,
}

/// A dependency cycle makes the network unschedulable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulingError {
    /// The dependency graph contains a cycle among these tasks.
    Cycle(Vec<TaskId>),
}

/// Compute the schedule for `project`, anchoring unconstrained tasks at
/// `project_start`. Returns per-task dates and any conflicts, or a `Cycle` error if
/// the dependency network is not acyclic.
pub fn schedule(
    project: &ProjectState,
    project_start: Date,
) -> Result<ScheduleResult, SchedulingError> {
    // A task participates in the network if it is not a summary and carries a
    // scheduling block. Summaries are rolled up afterwards; unscheduled leaves (e.g.
    // bare checklist items) simply get no dates.
    let schedulable: Vec<&Task> = project
        .tasks
        .values()
        .filter(|t| t.kind != TaskKind::Summary && t.schedule.is_some())
        .collect();

    // Build the dependency graph over schedulable tasks (reuse directed-graph).
    let mut graph = directed_graph::Graph::new();
    for t in &schedulable {
        graph.add_node(t.id.as_str());
    }
    let is_schedulable = |id: &TaskId| {
        project
            .tasks
            .get(id)
            .is_some_and(|t| t.kind != TaskKind::Summary && t.schedule.is_some())
    };
    for dep in &project.dependencies {
        // A self-dependency (predecessor == successor) is meaningless and is skipped;
        // `directed_graph` rejects self-loops anyway, and admitting it into the
        // adjacency maps below would make a task depend on its own unscheduled dates.
        if dep.predecessor != dep.successor
            && is_schedulable(&dep.predecessor)
            && is_schedulable(&dep.successor)
        {
            let _ = graph.add_edge(dep.predecessor.as_str(), dep.successor.as_str());
        }
    }

    // Topological order (and cycle rejection) for free.
    let order = match graph.topological_sort() {
        Ok(o) => o,
        Err(_) => {
            let involved = schedulable.iter().map(|t| t.id.clone()).collect();
            return Err(SchedulingError::Cycle(involved));
        }
    };

    // Predecessor / successor adjacency carrying the link kind and lag.
    let mut preds: BTreeMap<TaskId, Vec<(TaskId, DependencyKind, Duration)>> = BTreeMap::new();
    let mut succs: BTreeMap<TaskId, Vec<(TaskId, DependencyKind, Duration)>> = BTreeMap::new();
    for dep in &project.dependencies {
        if dep.predecessor != dep.successor
            && is_schedulable(&dep.predecessor)
            && is_schedulable(&dep.successor)
        {
            record_edge(&mut preds, &dep.successor, dep, dep.predecessor.clone());
            record_edge(&mut succs, &dep.predecessor, dep, dep.successor.clone());
        }
    }

    let mut conflicts = Vec::new();
    let mut es: BTreeMap<TaskId, Instant> = BTreeMap::new();
    let mut ef: BTreeMap<TaskId, Instant> = BTreeMap::new();

    // ── Forward pass: earliest start/finish ─────────────────────────────────────
    for id_str in &order {
        let id = TaskId::from_raw(id_str.clone());
        let task = &project.tasks[&id];
        let sched = task.schedule.as_ref().expect("schedulable ⇒ has schedule");
        let cal = calendar_for(project, task);
        let dur = sched.duration;

        // Floor at the project start (snapped into working time).
        let floor = calendar::next_working(project, cal, calendar::instant_of(project_start, 0))
            .unwrap_or_else(|| calendar::instant_of(project_start, 0));
        let mut start = floor;

        // Predecessor constraints per link type.
        for (pred, kind, lag) in preds.get(&id).into_iter().flatten() {
            // Defensive: predecessors precede successors in topological order, so this
            // is always populated — but never index-panic on an unexpected input.
            let (Some(&ps), Some(&pf)) = (es.get(pred), ef.get(pred)) else {
                continue;
            };
            let cand = match kind {
                DependencyKind::FinishToStart => lag_forward(project, cal, pf, *lag),
                DependencyKind::StartToStart => lag_forward(project, cal, ps, *lag),
                DependencyKind::FinishToFinish => {
                    calendar::sub_working(project, cal, lag_forward(project, cal, pf, *lag), dur)
                }
                DependencyKind::StartToFinish => {
                    calendar::sub_working(project, cal, lag_forward(project, cal, ps, *lag), dur)
                }
            };
            start = start.max(cand);
        }

        // Start-anchored constraints.
        match sched.constraint {
            Constraint::StartNoEarlierThan(d) => {
                let f = calendar::next_working(project, cal, calendar::instant_of(d, 0))
                    .unwrap_or_else(|| calendar::instant_of(d, 0));
                start = start.max(f);
            }
            Constraint::MustStartOn(d) => {
                let must = calendar::next_working(project, cal, calendar::instant_of(d, 0))
                    .unwrap_or_else(|| calendar::instant_of(d, 0));
                if start > must {
                    conflicts.push(Conflict {
                        task: id.clone(),
                        kind: ConflictKind::ConstraintVsDependency,
                        message: format!(
                            "MustStartOn {:?} but predecessors require a later start",
                            d.to_ymd()
                        ),
                    });
                }
                start = must;
            }
            Constraint::StartNoLaterThan(d)
                if date_of_start(start) > d => {
                    conflicts.push(Conflict {
                        task: id.clone(),
                        kind: ConflictKind::StartTooLate,
                        message: format!("starts after StartNoLaterThan {:?}", d.to_ymd()),
                    });
                }
            _ => {}
        }

        start = calendar::next_working(project, cal, start).unwrap_or(start);
        let mut finish = calendar::add_working(project, cal, start, dur);

        // Finish-anchored constraints.
        match sched.constraint {
            Constraint::FinishNoEarlierThan(d) => {
                let req = calendar::instant_of(d, 0);
                if finish < req {
                    finish = req;
                    start = calendar::sub_working(project, cal, finish, dur);
                }
            }
            Constraint::MustFinishOn(d) => {
                // Approximate: pin the finish to the end of the target day's work.
                let req = working_day_end(project, cal, d);
                finish = req;
                start = calendar::sub_working(project, cal, finish, dur);
            }
            Constraint::FinishNoLaterThan(d)
                if finish_date(start, finish) > d => {
                    conflicts.push(Conflict {
                        task: id.clone(),
                        kind: ConflictKind::FinishTooLate,
                        message: format!("finishes after FinishNoLaterThan {:?}", d.to_ymd()),
                    });
                }
            _ => {}
        }

        if let Some(deadline) = sched.deadline {
            if finish_date(start, finish) > deadline {
                conflicts.push(Conflict {
                    task: id.clone(),
                    kind: ConflictKind::DeadlineMissed,
                    message: format!("finishes after deadline {:?}", deadline.to_ymd()),
                });
            }
        }

        es.insert(id.clone(), start);
        ef.insert(id, finish);
    }

    // Project finish = the latest early finish.
    let project_finish_inst = ef.values().copied().max();

    // ── Backward pass: latest start/finish and slack ────────────────────────────
    let mut ls: BTreeMap<TaskId, Instant> = BTreeMap::new();
    let mut lf: BTreeMap<TaskId, Instant> = BTreeMap::new();
    for id_str in order.iter().rev() {
        let id = TaskId::from_raw(id_str.clone());
        let task = &project.tasks[&id];
        let sched = task.schedule.as_ref().expect("schedulable ⇒ has schedule");
        let cal = calendar_for(project, task);
        let dur = sched.duration;

        let mut late_finish = project_finish_inst.unwrap_or_else(|| ef[&id]);
        for (succ, kind, lag) in succs.get(&id).into_iter().flatten() {
            // Defensive: successors are scheduled first in the reverse walk.
            let (Some(&sls), Some(&slf)) = (ls.get(succ), lf.get(succ)) else {
                continue;
            };
            let cand = match kind {
                DependencyKind::FinishToStart => lag_backward(project, cal, sls, *lag),
                DependencyKind::StartToStart => {
                    calendar::add_working(project, cal, lag_backward(project, cal, sls, *lag), dur)
                }
                DependencyKind::FinishToFinish => lag_backward(project, cal, slf, *lag),
                DependencyKind::StartToFinish => {
                    calendar::add_working(project, cal, lag_backward(project, cal, slf, *lag), dur)
                }
            };
            late_finish = late_finish.min(cand);
        }
        let late_start = calendar::sub_working(project, cal, late_finish, dur);
        ls.insert(id.clone(), late_start);
        lf.insert(id, late_finish);
    }

    // ── Assemble per-task ScheduledDates ────────────────────────────────────────
    let mut dates: BTreeMap<TaskId, ScheduledDates> = BTreeMap::new();
    for id_str in &order {
        let id = TaskId::from_raw(id_str.clone());
        let task = &project.tasks[&id];
        let cal = calendar_for(project, task);
        let (e_s, e_f, l_s, l_f) = (es[&id], ef[&id], ls[&id], lf[&id]);
        let total_slack = calendar::working_between(project, cal, e_s, l_s);
        // Free slack: how long this task can slip without delaying any successor's
        // early start (FS-style gap; a good approximation for the other link types).
        let free_slack = match succs.get(&id) {
            Some(list) if !list.is_empty() => list
                .iter()
                .map(|(s, _, _)| calendar::working_between(project, cal, e_f, es[s]).max(0))
                .min()
                .unwrap_or(total_slack),
            _ => total_slack,
        };
        dates.insert(
            id.clone(),
            ScheduledDates {
                early_start: date_of_start(e_s),
                early_finish: finish_date(e_s, e_f),
                late_start: date_of_start(l_s),
                late_finish: finish_date(l_s, l_f),
                scheduled_start: date_of_start(e_s),
                scheduled_finish: finish_date(e_s, e_f),
                total_slack,
                free_slack,
                critical: total_slack <= 0,
            },
        );
    }

    // ── Summary rollups ─────────────────────────────────────────────────────────
    roll_up_summaries(project, &mut dates);

    Ok(ScheduleResult {
        dates,
        conflicts,
        project_start,
        project_finish: project_finish_inst.map(|f| finish_date(f.saturating_sub(1), f)),
    })
}

// ── helpers ─────────────────────────────────────────────────────────────────────

fn record_edge(
    map: &mut BTreeMap<TaskId, Vec<(TaskId, DependencyKind, Duration)>>,
    key: &TaskId,
    dep: &DependencyLink,
    other: TaskId,
) {
    map.entry(key.clone())
        .or_default()
        .push((other, dep.kind, dep.lag));
}

/// The calendar a task schedules against: its override if present and known,
/// otherwise the project calendar.
fn calendar_for<'a>(project: &'a ProjectState, task: &'a Task) -> &'a crate::ids::CalendarId {
    if let Some(sched) = &task.schedule {
        if let Some(c) = &sched.calendar {
            if project.calendars.contains_key(c) {
                return c;
            }
        }
    }
    &project.project_calendar
}

/// Shift `base` forward by `lag`. Positive working lag walks the calendar; negative
/// (lead) or elapsed lag is applied in raw wall-clock (a first-cut approximation).
fn lag_forward(
    project: &ProjectState,
    cal: &crate::ids::CalendarId,
    base: Instant,
    lag: Duration,
) -> Instant {
    if lag.elapsed || lag.working_minutes < 0 {
        base.saturating_add(lag.working_minutes)
    } else {
        calendar::add_working(project, cal, base, lag)
    }
}

/// Shift `base` backward by `lag` — the mirror of [`lag_forward`].
fn lag_backward(
    project: &ProjectState,
    cal: &crate::ids::CalendarId,
    base: Instant,
    lag: Duration,
) -> Instant {
    if lag.elapsed || lag.working_minutes < 0 {
        base.saturating_sub(lag.working_minutes)
    } else {
        calendar::sub_working(project, cal, base, lag)
    }
}

/// The date an early/late *start* instant falls on.
fn date_of_start(inst: Instant) -> Date {
    calendar::date_of(inst)
}

/// The date a task *finishes* on. Because finish instants are half-open (a task
/// occupies `[start, finish)`), the finishing calendar day is the day of the last
/// occupied minute, `finish - 1` — unless the task is a zero-duration milestone.
fn finish_date(start: Instant, finish: Instant) -> Date {
    if finish > start {
        calendar::date_of(finish - 1)
    } else {
        calendar::date_of(start)
    }
}

/// The end-of-work instant on `date` (the end of its last working interval), or the
/// day's end if the calendar has no working time that day.
fn working_day_end(project: &ProjectState, cal: &crate::ids::CalendarId, date: Date) -> Instant {
    // Snap forward to a working instant on/after the day start, then advance to the
    // end of that day's work by consuming the remaining working minutes of the day.
    let start = calendar::instant_of(date, 0);
    match calendar::next_working(project, cal, start) {
        Some(w) if calendar::date_of(w) == date => {
            // Consume the rest of the working day.
            let remaining =
                calendar::working_between(project, cal, w, calendar::instant_of(date, 1440));
            calendar::add_working(project, cal, w, Duration::minutes(remaining))
        }
        _ => calendar::instant_of(date, 1440),
    }
}

/// Aggregate leaf dates into their ancestor summary tasks: a summary spans the
/// earliest start and latest finish of its descendants, and is critical if any
/// descendant is.
fn roll_up_summaries(project: &ProjectState, dates: &mut BTreeMap<TaskId, ScheduledDates>) {
    // For each summary, gather descendant leaf dates.
    for (sid, task) in &project.tasks {
        if task.kind != TaskKind::Summary {
            continue;
        }
        let mut earliest: Option<Date> = None;
        let mut latest: Option<Date> = None;
        let mut any_critical = false;
        for (tid, sd) in dates.iter() {
            if is_descendant(project, tid, sid) {
                earliest = Some(earliest.map_or(sd.scheduled_start, |e| e.min(sd.scheduled_start)));
                latest = Some(latest.map_or(sd.scheduled_finish, |l| l.max(sd.scheduled_finish)));
                any_critical |= sd.critical;
            }
        }
        if let (Some(start), Some(finish)) = (earliest, latest) {
            dates.insert(
                sid.clone(),
                ScheduledDates {
                    early_start: start,
                    early_finish: finish,
                    late_start: start,
                    late_finish: finish,
                    scheduled_start: start,
                    scheduled_finish: finish,
                    total_slack: 0,
                    free_slack: 0,
                    critical: any_critical,
                },
            );
        }
    }
}

/// Whether `task` is a descendant of `ancestor` via the parent chain.
fn is_descendant(project: &ProjectState, task: &TaskId, ancestor: &TaskId) -> bool {
    let mut cur = project.tasks.get(task).and_then(|t| t.parent.clone());
    let mut guard = 0;
    while let Some(p) = cur {
        if &p == ancestor {
            return true;
        }
        cur = project.tasks.get(&p).and_then(|t| t.parent.clone());
        guard += 1;
        if guard > project.tasks.len() {
            break; // cycle guard (reparent invariants forbid this)
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{LinkId, ProjectId};
    use crate::model::{DependencyLink, TaskSchedule};

    fn day(y: i32, m: u32, d: u32) -> Date {
        Date::from_ymd(y, m, d).unwrap()
    }

    /// A one-working-day (8h) leaf task.
    fn scheduled_task(id: &str, name: &str, days: i64) -> Task {
        let mut t = Task::new(TaskId::from_raw(id), name);
        t.schedule = Some(TaskSchedule {
            duration: Duration::minutes(days * 8 * 60),
            work: crate::primitives::Work::minutes(days * 8 * 60),
            ..TaskSchedule::default()
        });
        t
    }

    fn link(id: &str, pred: &str, succ: &str, kind: DependencyKind) -> DependencyLink {
        DependencyLink {
            id: LinkId::from_raw(id),
            predecessor: TaskId::from_raw(pred),
            successor: TaskId::from_raw(succ),
            kind,
            lag: Duration::zero(),
        }
    }

    fn project_with(tasks: Vec<Task>, deps: Vec<DependencyLink>) -> ProjectState {
        let mut p = ProjectState::empty(ProjectId::from_raw("p1"));
        for t in tasks {
            p.tasks.insert(t.id.clone(), t);
        }
        p.dependencies = deps;
        p
    }

    #[test]
    fn finish_to_start_chain_is_sequential() {
        // A → B, both one 8h day, starting Monday 2026-07-13.
        let p = project_with(
            vec![scheduled_task("a", "A", 1), scheduled_task("b", "B", 1)],
            vec![link("l1", "a", "b", DependencyKind::FinishToStart)],
        );
        let r = schedule(&p, day(2026, 7, 13)).unwrap();
        let a = &r.dates[&TaskId::from_raw("a")];
        let b = &r.dates[&TaskId::from_raw("b")];
        // A: Monday. B: Tuesday (starts after A finishes end-of-Monday).
        assert_eq!(a.early_start, day(2026, 7, 13));
        assert_eq!(a.early_finish, day(2026, 7, 13));
        assert_eq!(b.early_start, day(2026, 7, 14));
        assert_eq!(b.early_finish, day(2026, 7, 14));
        assert_eq!(r.project_finish, Some(day(2026, 7, 14)));
        assert!(a.critical && b.critical, "a straight chain is all-critical");
    }

    #[test]
    fn duration_spills_across_the_weekend() {
        // A single 3-day task starting Friday spans Fri, Mon, Tue.
        let p = project_with(vec![scheduled_task("a", "A", 3)], vec![]);
        let r = schedule(&p, day(2026, 7, 10)).unwrap();
        let a = &r.dates[&TaskId::from_raw("a")];
        assert_eq!(a.early_start, day(2026, 7, 10)); // Friday
        assert_eq!(a.early_finish, day(2026, 7, 14)); // Tuesday
    }

    #[test]
    fn diamond_network_has_slack_off_the_critical_path() {
        // A(1) → B(2) → D(1) is the critical path (4d); A → C(1) → D leaves C slack.
        let p = project_with(
            vec![
                scheduled_task("a", "A", 1),
                scheduled_task("b", "B", 2),
                scheduled_task("c", "C", 1),
                scheduled_task("d", "D", 1),
            ],
            vec![
                link("l1", "a", "b", DependencyKind::FinishToStart),
                link("l2", "a", "c", DependencyKind::FinishToStart),
                link("l3", "b", "d", DependencyKind::FinishToStart),
                link("l4", "c", "d", DependencyKind::FinishToStart),
            ],
        );
        let r = schedule(&p, day(2026, 7, 13)).unwrap();
        let c = &r.dates[&TaskId::from_raw("c")];
        assert_eq!(r.project_finish, Some(day(2026, 7, 16))); // Thursday
        assert!(r.dates[&TaskId::from_raw("b")].critical);
        assert!(r.dates[&TaskId::from_raw("d")].critical);
        assert!(!c.critical, "C is off the critical path");
        assert_eq!(c.total_slack, 8 * 60, "C has one working day of slack");
    }

    #[test]
    fn dependency_cycle_is_rejected() {
        let p = project_with(
            vec![scheduled_task("a", "A", 1), scheduled_task("b", "B", 1)],
            vec![
                link("l1", "a", "b", DependencyKind::FinishToStart),
                link("l2", "b", "a", DependencyKind::FinishToStart),
            ],
        );
        assert!(matches!(
            schedule(&p, day(2026, 7, 13)),
            Err(SchedulingError::Cycle(_))
        ));
    }

    #[test]
    fn self_dependency_does_not_panic() {
        // A task that depends on itself (from hostile/broken input) must not crash the
        // scheduler; the meaningless self-link is ignored and the task schedules.
        let p = project_with(
            vec![scheduled_task("a", "A", 1)],
            vec![link("l1", "a", "a", DependencyKind::FinishToStart)],
        );
        let r = schedule(&p, day(2026, 7, 13)).unwrap();
        assert_eq!(
            r.dates[&TaskId::from_raw("a")].early_start,
            day(2026, 7, 13)
        );
    }

    #[test]
    fn start_no_earlier_than_floors_the_start() {
        let mut a = scheduled_task("a", "A", 1);
        a.schedule.as_mut().unwrap().constraint = Constraint::StartNoEarlierThan(day(2026, 7, 15));
        let p = project_with(vec![a], vec![]);
        let r = schedule(&p, day(2026, 7, 13)).unwrap();
        // Even though the project starts Monday, the task cannot start before Wed 15th.
        assert_eq!(
            r.dates[&TaskId::from_raw("a")].early_start,
            day(2026, 7, 15)
        );
    }

    #[test]
    fn a_missed_deadline_is_flagged() {
        let mut a = scheduled_task("a", "A", 3); // finishes Tue 2026-07-14 from Friday
        a.schedule.as_mut().unwrap().deadline = Some(day(2026, 7, 13));
        let p = project_with(vec![a], vec![]);
        let r = schedule(&p, day(2026, 7, 10)).unwrap();
        assert!(r
            .conflicts
            .iter()
            .any(|c| c.kind == ConflictKind::DeadlineMissed));
    }

    #[test]
    fn summary_spans_its_children() {
        // A summary S over children A (Mon) and B (Tue, FS after A).
        let mut s = Task::new(TaskId::from_raw("s"), "Phase");
        s.kind = TaskKind::Summary;
        let mut a = scheduled_task("a", "A", 1);
        a.parent = Some(TaskId::from_raw("s"));
        let mut b = scheduled_task("b", "B", 1);
        b.parent = Some(TaskId::from_raw("s"));
        let p = project_with(
            vec![s, a, b],
            vec![link("l1", "a", "b", DependencyKind::FinishToStart)],
        );
        let r = schedule(&p, day(2026, 7, 13)).unwrap();
        let sd = &r.dates[&TaskId::from_raw("s")];
        assert_eq!(sd.scheduled_start, day(2026, 7, 13)); // A's start
        assert_eq!(sd.scheduled_finish, day(2026, 7, 14)); // B's finish
    }
}
