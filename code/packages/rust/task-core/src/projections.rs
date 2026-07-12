//! View projections — the "one model, many views" thesis as pure queries.
//!
//! Each task tool is a *projection* of the same `ProjectState`: a checklist flattens
//! the decision-visible items, a kanban groups by status, a Gantt reads the schedule,
//! a flowchart reads the relation graph. Every projection here is a pure `&self`
//! method returning plain data — no state, no I/O — so a host can render any view from
//! the one engine.

use crate::ids::*;
use crate::model::*;
use crate::primitives::Date;
use crate::scheduler::{self, ScheduleResult, SchedulingError};
use std::collections::{HashMap, HashSet};

impl ProjectState {
    /// Run the CPM scheduler (convenience wrapper over [`scheduler::schedule`]).
    pub fn schedule(&self, project_start: Date) -> Result<ScheduleResult, SchedulingError> {
        scheduler::schedule(self, project_start)
    }

    // ── checklist ────────────────────────────────────────────────────────────────

    /// Flatten the tasks into a checklist, honouring decision branches: a decision
    /// task reveals only its answered branch (nothing until answered), exactly like a
    /// pilot's checklist hides the irrelevant path.
    pub fn checklist(&self) -> Vec<ChecklistRow> {
        // Precompute outline children per parent once (O(n)) so the whole walk is
        // O(n log n) rather than O(n²) from a per-node scan.
        let mut children_index: HashMap<TaskId, Vec<TaskId>> = HashMap::new();
        for t in self.tasks.values() {
            if let Some(p) = &t.parent {
                children_index
                    .entry(p.clone())
                    .or_default()
                    .push(t.id.clone());
            }
        }
        for kids in children_index.values_mut() {
            kids.sort_by_key(|id| self.order_key(id));
        }

        let mut rows = Vec::new();
        let mut visited = HashSet::new();
        // Iterative DFS so traversal depth is heap-bounded, not stack-bounded — a
        // recursive walk would overflow the stack on a deep hierarchy (an uncatchable
        // abort, as `formula.rs` notes for its parser). Roots are pushed reversed so
        // the pop order matches outline order.
        let mut stack: Vec<(TaskId, u32)> =
            self.roots().into_iter().rev().map(|id| (id, 0)).collect();
        while let Some((id, depth)) = stack.pop() {
            if !visited.insert(id.clone()) {
                continue; // guard against shared/cyclic references
            }
            let Some(t) = self.tasks.get(&id) else {
                continue;
            };
            rows.push(ChecklistRow {
                task: id.clone(),
                name: t.name.clone(),
                depth,
                completed: t.completed,
                is_decision: t.decision.is_some(),
                answered: t.decision.as_ref().and_then(|d| d.answer),
            });
            // Visible children: a decision shows only its answered branch; a plain task
            // shows its outline children.
            let children: Vec<TaskId> = match &t.decision {
                Some(d) => match d.answer {
                    Some(true) => d.yes_children.clone(),
                    Some(false) => d.no_children.clone(),
                    None => Vec::new(),
                },
                None => children_index.get(&id).cloned().unwrap_or_default(),
            };
            for c in children.into_iter().rev() {
                stack.push((c, depth + 1));
            }
        }
        rows
    }

    // ── todos ────────────────────────────────────────────────────────────────────

    /// A flat list of leaf tasks, sorted by deadline (soonest first) then name.
    pub fn todos(&self) -> Vec<TodoRow> {
        let mut rows: Vec<TodoRow> = self
            .tasks
            .values()
            .filter(|t| t.kind == TaskKind::Leaf)
            .map(|t| TodoRow {
                task: t.id.clone(),
                name: t.name.clone(),
                completed: t.completed,
                deadline: t.schedule.as_ref().and_then(|s| s.deadline),
                percent_complete: t.percent_complete,
            })
            .collect();
        rows.sort_by(|a, b| match (a.deadline, b.deadline) {
            (Some(x), Some(y)) => x.0.cmp(&y.0).then_with(|| a.name.cmp(&b.name)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.name.cmp(&b.name),
        });
        rows
    }

    // ── kanban ───────────────────────────────────────────────────────────────────

    /// Group tasks into board columns by workflow status. Columns are ordered by
    /// lifecycle category (Todo → In-Progress → Done) then name; tasks with no status
    /// go in a leading "No status" column.
    pub fn kanban(&self, workflow: &Workflow) -> Vec<KanbanColumn> {
        let card = |t: &Task| KanbanCard {
            task: t.id.clone(),
            name: t.name.clone(),
            percent_complete: t.percent_complete,
        };

        let mut columns = Vec::new();
        // Leading column for unassigned tasks.
        let no_status: Vec<KanbanCard> = self
            .tasks
            .values()
            .filter(|t| t.kind != TaskKind::Summary && t.status.is_none())
            .map(&card)
            .collect();
        if !no_status.is_empty() {
            columns.push(KanbanColumn {
                status: None,
                name: "No status".into(),
                cards: no_status,
            });
        }

        // Workflow statuses, ordered by category then name.
        let mut statuses: Vec<&Status> = workflow.statuses.values().collect();
        statuses.sort_by_key(|s| (category_rank(s.category), s.name.clone()));
        for st in statuses {
            let cards: Vec<KanbanCard> = self
                .tasks
                .values()
                .filter(|t| t.kind != TaskKind::Summary && t.status.as_ref() == Some(&st.id))
                .map(&card)
                .collect();
            columns.push(KanbanColumn {
                status: Some(st.id.clone()),
                name: st.name.clone(),
                cards,
            });
        }
        columns
    }

    // ── gantt ────────────────────────────────────────────────────────────────────

    /// Timeline bars from the CPM schedule, sorted by start date. A cyclic network
    /// yields an empty view (the cycle is surfaced elsewhere).
    pub fn gantt(&self, project_start: Date) -> GanttView {
        let Ok(res) = self.schedule(project_start) else {
            return GanttView {
                bars: Vec::new(),
                project_finish: None,
            };
        };
        let mut bars: Vec<GanttBar> = res
            .dates
            .iter()
            .filter_map(|(id, d)| {
                self.tasks.get(id).map(|t| GanttBar {
                    task: id.clone(),
                    name: t.name.clone(),
                    start: d.scheduled_start,
                    finish: d.scheduled_finish,
                    critical: d.critical,
                    percent_complete: t.percent_complete,
                    depth: self.depth_of(id),
                })
            })
            .collect();
        bars.sort_by(|a, b| a.start.0.cmp(&b.start.0).then_with(|| a.name.cmp(&b.name)));
        GanttView {
            bars,
            project_finish: res.project_finish,
        }
    }

    // ── flowchart ────────────────────────────────────────────────────────────────

    /// The relation graph: tasks as nodes, dependencies and generic links as labelled
    /// edges.
    pub fn flowchart(&self) -> FlowGraph {
        let nodes = self
            .tasks
            .values()
            .map(|t| FlowNode {
                task: t.id.clone(),
                name: t.name.clone(),
                kind: t.kind,
            })
            .collect();
        let mut edges = Vec::new();
        for d in &self.dependencies {
            edges.push(FlowEdge {
                from: d.predecessor.clone(),
                to: d.successor.clone(),
                label: dep_label(d.kind).into(),
                scheduling: true,
            });
        }
        for l in &self.links {
            edges.push(FlowEdge {
                from: l.from.clone(),
                to: l.to.clone(),
                label: link_label(&l.kind),
                scheduling: false,
            });
        }
        FlowGraph { nodes, edges }
    }

    // ── shared helpers ───────────────────────────────────────────────────────────

    /// Top-level tasks, in outline order. Excludes tasks *owned by a decision branch*
    /// (they are reached only through their decision, never as roots).
    fn roots(&self) -> Vec<TaskId> {
        let mut decision_children: HashSet<TaskId> = HashSet::new();
        for t in self.tasks.values() {
            if let Some(d) = &t.decision {
                decision_children.extend(d.yes_children.iter().cloned());
                decision_children.extend(d.no_children.iter().cloned());
            }
        }
        let mut roots: Vec<&Task> = self
            .tasks
            .values()
            .filter(|t| t.parent.is_none() && !decision_children.contains(&t.id))
            .collect();
        roots.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.id.0.cmp(&b.id.0)));
        roots.into_iter().map(|t| t.id.clone()).collect()
    }

    /// The outline sort key for a task id: `(order, id)`.
    fn order_key(&self, id: &TaskId) -> (i64, String) {
        self.tasks
            .get(id)
            .map(|t| (t.order, t.id.0.clone()))
            .unwrap_or((0, id.0.clone()))
    }

    /// Outline depth (number of ancestors), bounded against a malformed parent chain.
    fn depth_of(&self, id: &TaskId) -> u32 {
        let mut depth = 0;
        let mut cur = self.tasks.get(id).and_then(|t| t.parent.clone());
        let mut guard = 0;
        while let Some(p) = cur {
            depth += 1;
            cur = self.tasks.get(&p).and_then(|t| t.parent.clone());
            guard += 1;
            if guard > self.tasks.len() {
                break;
            }
        }
        depth
    }
}

// ── projection output types ──────────────────────────────────────────────────────

/// One row in the flattened checklist.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ChecklistRow {
    /// The task.
    pub task: TaskId,
    /// Its name.
    pub name: String,
    /// Indentation depth in the tree.
    pub depth: u32,
    /// Whether it is ticked off.
    pub completed: bool,
    /// Whether this row is a yes/no decision point.
    pub is_decision: bool,
    /// The decision's answer, if answered.
    pub answered: Option<bool>,
}

/// One row in the flat todo list.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TodoRow {
    /// The task.
    pub task: TaskId,
    /// Its name.
    pub name: String,
    /// Whether it is done.
    pub completed: bool,
    /// Its due date, if any.
    pub deadline: Option<Date>,
    /// Progress 0..=100.
    pub percent_complete: u8,
}

/// A card on a kanban board.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct KanbanCard {
    /// The task.
    pub task: TaskId,
    /// Its name.
    pub name: String,
    /// Progress 0..=100.
    pub percent_complete: u8,
}

/// A column on a kanban board.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct KanbanColumn {
    /// The status this column represents (`None` = the unassigned column).
    pub status: Option<StatusId>,
    /// The column heading.
    pub name: String,
    /// The cards in it.
    pub cards: Vec<KanbanCard>,
}

/// A bar on a Gantt timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GanttBar {
    /// The task.
    pub task: TaskId,
    /// Its name.
    pub name: String,
    /// Scheduled start.
    pub start: Date,
    /// Scheduled finish.
    pub finish: Date,
    /// On the critical path.
    pub critical: bool,
    /// Progress 0..=100.
    pub percent_complete: u8,
    /// Outline depth (for indentation).
    pub depth: u32,
}

/// A Gantt timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GanttView {
    /// The bars, sorted by start.
    pub bars: Vec<GanttBar>,
    /// The project finish date, if scheduled.
    pub project_finish: Option<Date>,
}

/// A node in the flowchart.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FlowNode {
    /// The task.
    pub task: TaskId,
    /// Its name.
    pub name: String,
    /// Leaf/summary/milestone.
    pub kind: TaskKind,
}

/// An edge in the flowchart.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FlowEdge {
    /// Source task.
    pub from: TaskId,
    /// Target task.
    pub to: TaskId,
    /// A short label (dependency type or link kind).
    pub label: String,
    /// True for a scheduling dependency, false for a generic link.
    pub scheduling: bool,
}

/// The flowchart graph.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FlowGraph {
    /// The nodes.
    pub nodes: Vec<FlowNode>,
    /// The edges.
    pub edges: Vec<FlowEdge>,
}

fn category_rank(c: StatusCategory) -> u8 {
    match c {
        StatusCategory::Todo => 0,
        StatusCategory::InProgress => 1,
        StatusCategory::Done => 2,
    }
}

fn dep_label(kind: DependencyKind) -> &'static str {
    match kind {
        DependencyKind::FinishToStart => "FS",
        DependencyKind::StartToStart => "SS",
        DependencyKind::FinishToFinish => "FF",
        DependencyKind::StartToFinish => "SF",
    }
}

fn link_label(kind: &LinkKind) -> String {
    match kind {
        LinkKind::Blocks => "blocks".into(),
        LinkKind::Relates => "relates".into(),
        LinkKind::Duplicates => "duplicates".into(),
        LinkKind::Causes => "causes".into(),
        LinkKind::Custom(s) => s.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{Duration, Work};

    fn tid(s: &str) -> TaskId {
        TaskId::from_raw(s)
    }
    fn day(y: i32, m: u32, d: u32) -> Date {
        Date::from_ymd(y, m, d).unwrap()
    }

    #[test]
    fn checklist_hides_the_unanswered_and_unchosen_branches() {
        let mut s = ProjectState::empty(ProjectId::from_raw("p1"));
        s.create_task(tid("q"), "Ready to deploy?", None).unwrap();
        s.create_task(tid("yes1"), "Deploy", None).unwrap();
        s.create_task(tid("no1"), "Fix first", None).unwrap();
        s.set_decision(
            &tid("q"),
            Some(Decision {
                question: "Ready?".into(),
                answer: None,
                yes_children: vec![tid("yes1")],
                no_children: vec![tid("no1")],
            }),
        )
        .unwrap();

        // Unanswered: only the question shows.
        let rows = s.checklist();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].is_decision && rows[0].answered.is_none());

        // Answer yes → the question + the yes branch (not the no branch).
        s.answer_decision(&tid("q"), true).unwrap();
        let names: Vec<_> = s.checklist().into_iter().map(|r| r.name).collect();
        assert!(names.contains(&"Deploy".to_string()));
        assert!(!names.contains(&"Fix first".to_string()));
    }

    #[test]
    fn deep_hierarchy_does_not_overflow_the_stack() {
        // A deep linear chain would blow a recursive walk's stack; the iterative
        // checklist must handle it. 20_000 levels is well past the recursion limit.
        let mut s = ProjectState::empty(ProjectId::from_raw("p1"));
        s.create_task(tid("t0"), "t0", None).unwrap();
        for i in 1..20_000u32 {
            s.create_task(
                tid(&format!("t{i}")),
                "t",
                Some(tid(&format!("t{}", i - 1))),
            )
            .unwrap();
        }
        let rows = s.checklist();
        assert_eq!(rows.len(), 20_000);
        assert_eq!(rows.last().unwrap().depth, 19_999);
    }

    #[test]
    fn kanban_groups_by_status() {
        let mut s = ProjectState::empty(ProjectId::from_raw("p1"));
        let mut statuses = std::collections::BTreeMap::new();
        for (id, name, cat) in [
            ("todo", "To Do", StatusCategory::Todo),
            ("doing", "Doing", StatusCategory::InProgress),
        ] {
            statuses.insert(
                StatusId::from_raw(id),
                Status {
                    id: StatusId::from_raw(id),
                    name: name.into(),
                    category: cat,
                    color: "#fff".into(),
                },
            );
        }
        let wf = Workflow {
            id: WorkflowId::from_raw("w"),
            name: "Board".into(),
            statuses,
            transitions: vec![],
            done_status: StatusId::from_raw("doing"),
        };
        s.create_task(tid("a"), "A", None).unwrap();
        s.set_status(&tid("a"), Some(StatusId::from_raw("doing")))
            .unwrap();
        s.create_task(tid("b"), "B", None).unwrap(); // no status

        let cols = s.kanban(&wf);
        // "No status" column + 2 workflow columns, in category order.
        assert_eq!(cols[0].name, "No status");
        assert_eq!(cols[0].cards.len(), 1);
        let doing = cols.iter().find(|c| c.name == "Doing").unwrap();
        assert_eq!(doing.cards[0].task, tid("a"));
    }

    #[test]
    fn gantt_reads_the_schedule_with_critical_flags() {
        let mut s = ProjectState::empty(ProjectId::from_raw("p1"));
        for id in ["a", "b"] {
            s.create_task(tid(id), id, None).unwrap();
            s.set_schedule(
                &tid(id),
                Some(TaskSchedule {
                    duration: Duration::minutes(8 * 60),
                    work: Work::minutes(8 * 60),
                    ..TaskSchedule::default()
                }),
            )
            .unwrap();
        }
        s.link_dependency(DependencyLink {
            id: LinkId::from_raw("l1"),
            predecessor: tid("a"),
            successor: tid("b"),
            kind: DependencyKind::FinishToStart,
            lag: Duration::zero(),
        })
        .unwrap();

        let g = s.gantt(day(2026, 7, 13)); // Monday
        assert_eq!(g.bars.len(), 2);
        assert_eq!(g.project_finish, Some(day(2026, 7, 14)));
        assert!(
            g.bars.iter().all(|b| b.critical),
            "an FS chain is all-critical"
        );
    }

    #[test]
    fn flowchart_has_nodes_and_labelled_edges() {
        let mut s = ProjectState::empty(ProjectId::from_raw("p1"));
        s.create_task(tid("a"), "A", None).unwrap();
        s.create_task(tid("b"), "B", None).unwrap();
        s.link_dependency(DependencyLink {
            id: LinkId::from_raw("l1"),
            predecessor: tid("a"),
            successor: tid("b"),
            kind: DependencyKind::StartToStart,
            lag: Duration::zero(),
        })
        .unwrap();
        s.add_link(GenericLink {
            id: LinkId::from_raw("l2"),
            from: tid("b"),
            to: tid("a"),
            kind: LinkKind::Relates,
        })
        .unwrap();

        let g = s.flowchart();
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.edges.len(), 2);
        assert!(g.edges.iter().any(|e| e.label == "SS" && e.scheduling));
        assert!(g
            .edges
            .iter()
            .any(|e| e.label == "relates" && !e.scheduling));
    }

    #[test]
    fn todos_sort_by_deadline_then_name() {
        let mut s = ProjectState::empty(ProjectId::from_raw("p1"));
        s.create_task(tid("late"), "Late", None).unwrap();
        s.set_deadline(&tid("late"), Some(day(2026, 8, 1))).unwrap();
        s.create_task(tid("soon"), "Soon", None).unwrap();
        s.set_deadline(&tid("soon"), Some(day(2026, 7, 15)))
            .unwrap();
        s.create_task(tid("none"), "NoDeadline", None).unwrap();

        let todos = s.todos();
        assert_eq!(todos[0].task, tid("soon"));
        assert_eq!(todos[1].task, tid("late"));
        assert_eq!(todos[2].task, tid("none"), "no-deadline sorts last");
    }
}
