#![forbid(unsafe_code)]
//! # task-core
//!
//! The headless engine behind **task-app**: a general task- and project-management
//! model, designed for the *hardest* case (Microsoft Project-class scheduling) so
//! that every simpler tool — a checklist, a todo list, a kanban board, a Gantt
//! chart, a flowchart — is a *restriction* of one rich `Task` entity rather than a
//! separate data model.
//!
//! ## What lives here
//!
//! This crate is **pure**: no I/O, no system clock (the current time is always
//! passed in as `now: u64`), and no id generation (ids are minted by the facade and
//! passed in — see the note on [`ids`]). `serde` is behind a feature flag, so the
//! model has zero external dependencies by default. This mirrors the house style of
//! `engram-core` and `spreadsheet-core`.
//!
//! Everything a scheduler *derives* (early/late dates, slack, the critical flag,
//! rollups, formula values) is **computed, never stored as source of truth** — the
//! stored state is the minimal set of inputs, and the schedule is reproducible from
//! it. See `code/specs/task-app-data-model.md` for the full design.
//!
//! ## Module map
//!
//! - [`ids`] — typed, string-backed entity identifiers.
//! - [`primitives`] — `Date`, `Duration`, `Work`, `Money`: the units the model
//!   measures in, with civil-date arithmetic delegated to `datetime-core`.
//! - [`model`] — the entities: `Task`, links, resources, calendars, fields,
//!   workflow, baselines, views, and the root `ProjectState`.
//! - [`calendar`] — the working-time engine: resolve calendars, snap into working
//!   time, add working durations, and count working minutes. The unit the scheduler
//!   measures in.
//!
//! The CPM scheduler (a forward/backward pass over `directed-graph`, built on
//! [`calendar`]) and the command/reducer surface land in follow-up modules; the
//! types here define the *shape* of the world they operate on.

pub mod calendar;
mod ids;
mod model;
mod primitives;

pub use ids::*;
pub use model::*;
pub use primitives::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_a_minimal_project() {
        // A project with one task — the smallest meaningful state.
        let mut project = ProjectState::empty(ProjectId::from_raw("p1"));
        let task = Task::new(TaskId::from_raw("t1"), "Write the spec");
        project.tasks.insert(task.id.clone(), task);

        assert_eq!(project.tasks.len(), 1);
        let t = project.tasks.get(&TaskId::from_raw("t1")).unwrap();
        assert_eq!(t.name, "Write the spec");
        assert!(!t.completed);
        assert_eq!(t.kind, TaskKind::Leaf);
        assert!(t.schedule.is_none(), "a bare task carries no scheduling");
    }

    #[test]
    fn empty_project_seeds_a_standard_calendar() {
        let project = ProjectState::empty(ProjectId::from_raw("p1"));
        let cal = project
            .calendars
            .get(&project.project_calendar)
            .expect("project calendar exists");
        // Monday (index 0) is a working day; Sunday (index 6) is not.
        assert!(cal.work_week[0].working);
        assert!(!cal.work_week[6].working);
        assert_eq!(cal.work_week[0].intervals[0].start_min, 9 * 60);
        assert_eq!(cal.work_week[0].intervals[0].end_min, 17 * 60);
    }

    #[test]
    fn date_arithmetic_delegates_to_datetime_core() {
        // 2026-07-10 is a Friday (ISO weekday 5).
        let d = Date::from_ymd(2026, 7, 10).unwrap();
        assert_eq!(d.weekday(), 5);
        assert_eq!(d.to_ymd(), (2026, 7, 10));
        // Adding one day lands on Saturday.
        assert_eq!(d.add_days(1).weekday(), 6);
        assert_eq!(d.days_until(d.add_days(7)), 7);
        assert!(
            Date::from_ymd(2026, 13, 1).is_none(),
            "invalid month rejected"
        );
    }

    #[test]
    fn primitive_helpers() {
        assert!(Duration::zero().is_zero());
        assert_eq!(Duration::minutes(480).working_minutes, 480);
        assert!(!Duration::minutes(1).elapsed);
        assert_eq!(Work::minutes(960).minutes, 960);
        assert_eq!(Work::zero().minutes, 0);
        let m = Money::zero("USD");
        assert_eq!(m.minor_units, 0);
        assert_eq!(m.currency, "USD");
    }

    #[test]
    fn a_fully_scheduled_task_carries_the_whole_model() {
        // Exercise the scheduling block, a decision, a dependency, and an assignment
        // together — the shapes a Gantt/flowchart projection reads.
        let mut project = ProjectState::empty(ProjectId::from_raw("p1"));

        let mut design = Task::new(TaskId::from_raw("t1"), "Design");
        design.schedule = Some(TaskSchedule {
            duration: Duration::minutes(3 * 8 * 60),
            work: Work::minutes(3 * 8 * 60),
            constraint: Constraint::StartNoEarlierThan(Date::from_ymd(2026, 7, 13).unwrap()),
            deadline: Some(Date::from_ymd(2026, 7, 20).unwrap()),
            ..TaskSchedule::default()
        });
        let build = Task::new(TaskId::from_raw("t2"), "Build");

        project.tasks.insert(design.id.clone(), design);
        project.tasks.insert(build.id.clone(), build);
        project.dependencies.push(DependencyLink {
            id: LinkId::from_raw("l1"),
            predecessor: TaskId::from_raw("t1"),
            successor: TaskId::from_raw("t2"),
            kind: DependencyKind::FinishToStart,
            lag: Duration::zero(),
        });

        let dev = Resource {
            id: ResourceId::from_raw("r1"),
            name: "Dev".into(),
            kind: ResourceKind::Work,
            calendar: None,
            max_units: 1.0,
            std_rate: Money::zero("USD"),
            cost_per_use: Money::zero("USD"),
        };
        project.resources.insert(dev.id.clone(), dev);
        project.assignments.push(Assignment {
            task: TaskId::from_raw("t1"),
            resource: ResourceId::from_raw("r1"),
            units: 1.0,
            work: Work::minutes(3 * 8 * 60),
            contour: WorkContour::Flat,
        });

        assert_eq!(project.tasks.len(), 2);
        assert_eq!(project.dependencies[0].kind, DependencyKind::FinishToStart);
        let sched = project.tasks[&TaskId::from_raw("t1")]
            .schedule
            .as_ref()
            .unwrap();
        assert!(matches!(
            sched.constraint,
            Constraint::StartNoEarlierThan(_)
        ));
        assert_eq!(project.assignments[0].units, 1.0);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn project_json_round_trips_with_camelcase_and_string_ids() {
        let mut project = ProjectState::empty(ProjectId::from_raw("p1"));
        let mut t = Task::new(TaskId::from_raw("t1"), "Ship it");
        t.percent_complete = 40;
        t.schedule = Some(TaskSchedule {
            task_type: TaskType::FixedWork,
            ..TaskSchedule::default()
        });
        project.tasks.insert(t.id.clone(), t);

        let json = serde_json::to_string(&project).unwrap();
        // Wire contract: camelCase field names …
        assert!(json.contains("\"percentComplete\":40"));
        assert!(json.contains("\"taskType\":\"fixedWork\""));
        // … and ids serialise as bare strings (serde transparent), so the tasks map
        // is keyed by the plain id string.
        assert!(json.contains("\"t1\":{"));

        // And it deserialises back to an equal value.
        let back: ProjectState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, project);
    }
}
