//! # diagram-layout-temporal
//!
//! Layout engine for temporal diagrams (DG04): Gantt and git-graph.
//!
//! ## Gantt
//! Maps each task to a horizontal bar on a time axis.  Task dates are
//! parsed as `YYYY-MM-DD`; `After(id)` dependencies are resolved in a
//! second pass after all absolute starts are known.
//!
//! ## Git-graph
//! Assigns each branch a horizontal lane and replays commit events to
//! place commit nodes and merge arcs.

use diagram_ir::{
    DiagramDirection, GanttSection, GitDiagram, GitEvent,
    LayoutedTemporalDiagram, LayoutedTemporalItem, TaskStart, TaskStatus,
    TemporalBody, TemporalDiagram,
};
use std::collections::HashMap;

pub const VERSION: &str = "0.1.0";

// ── Constants ─────────────────────────────────────────────────────────────

const AXIS_H:         f64 = 28.0;
const TASK_H:         f64 = 20.0;
const TASK_GAP:       f64 = 4.0;
const LABEL_W:        f64 = 120.0;
const TICK_DAYS:      f64 = 7.0;
const SECTION_H:      f64 = 24.0;
const LANE_H:         f64 = 60.0;
const COMMIT_SPACING: f64 = 80.0;

const BRANCH_COLORS: &[&str] = &[
    "#3b82f6", "#ef4444", "#22c55e", "#f59e0b", "#a855f7", "#14b8a6",
];

/// Lay out a `TemporalDiagram` on a canvas of `cw` pixels wide.
pub fn layout_temporal_diagram(d: &TemporalDiagram, cw: f64) -> LayoutedTemporalDiagram {
    match &d.body {
        TemporalBody::Gantt(g) => layout_gantt(&d.title, g, cw),
        TemporalBody::Git(g)   => layout_git(g, cw),
    }
}

// ── Date helpers ──────────────────────────────────────────────────────────

/// Approximate days since a fixed epoch for `YYYY-MM-DD` strings.
/// Uses 365.25 d/yr and 30.44 d/month — good enough for Gantt bar widths.
fn date_to_days(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 { return None; }
    let y: f64 = parts[0].parse().ok()?;
    let m: f64 = parts[1].parse().ok()?;
    let d: f64 = parts[2].parse().ok()?;
    Some(y * 365.25 + (m - 1.0) * 30.44 + d)
}

// ── Gantt layout ──────────────────────────────────────────────────────────

fn layout_gantt(
    title: &Option<String>,
    diagram: &diagram_ir::GanttDiagram,
    cw: f64,
) -> LayoutedTemporalDiagram {
    // First pass: collect all task absolute starts (Date variant only).
    let mut starts: HashMap<String, f64> = HashMap::new();
    for section in &diagram.sections {
        for task in &section.tasks {
            if let TaskStart::Date(ref ds) = task.start {
                if let Some(d) = date_to_days(ds) {
                    starts.insert(task.id.clone(), d);
                }
            }
        }
    }
    // Second pass: resolve After dependencies.
    for section in &diagram.sections {
        for task in &section.tasks {
            if let TaskStart::After(ref dep_id) = task.start {
                if !starts.contains_key(&task.id) {
                    if let Some(&dep_end) = starts.get(dep_id) {
                        // Find duration of the dep task.
                        let dep_dur = diagram.sections.iter()
                            .flat_map(|s| s.tasks.iter())
                            .find(|t| &t.id == dep_id)
                            .map(|t| t.duration_days)
                            .unwrap_or(0.0);
                        starts.insert(task.id.clone(), dep_end + dep_dur);
                    }
                }
            }
        }
    }

    // Determine time range.
    let t_min = starts.values().cloned().fold(f64::INFINITY, f64::min);
    let t_max = diagram.sections.iter()
        .flat_map(|s| s.tasks.iter())
        .filter_map(|t| starts.get(&t.id).map(|&s| s + t.duration_days))
        .fold(f64::NEG_INFINITY, f64::max);
    let t_min = if t_min.is_infinite() { 0.0 } else { t_min };
    let t_max = if t_max.is_infinite() { t_min + 30.0 } else { t_max };
    let t_range = (t_max - t_min).max(1.0);

    let plot_w = (cw - LABEL_W - 32.0).max(100.0);
    let x_scale = plot_w / t_range;

    let mut items: Vec<LayoutedTemporalItem> = Vec::new();
    let mut y = AXIS_H;

    // Title
    if let Some(ref t) = title {
        items.push(LayoutedTemporalItem::SectionHeader {
            x: 0.0, y: 0.0, width: cw, height: AXIS_H, label: t.clone(),
        });
        y += AXIS_H;
    }

    // Time axis spine.
    items.push(LayoutedTemporalItem::TimeAxisSpine {
        x1: LABEL_W, y1: y, x2: cw - 16.0, y2: y,
    });

    // Axis ticks (weekly).
    let mut tick_day = 0.0;
    while tick_day <= t_range {
        let tx = LABEL_W + tick_day * x_scale;
        items.push(LayoutedTemporalItem::TimeAxisTick {
            x: tx, y: y + 4.0,
            label: format!("d{:.0}", tick_day),
        });
        tick_day += TICK_DAYS;
    }
    y += AXIS_H;

    // Sections and tasks.
    for section in &diagram.sections {
        if let Some(ref lbl) = section.label {
            items.push(LayoutedTemporalItem::SectionHeader {
                x: 0.0, y, width: cw, height: SECTION_H, label: lbl.clone(),
            });
            y += SECTION_H;
        }
        for task in &section.tasks {
            let start_day = starts.get(&task.id).copied().unwrap_or(t_min) - t_min;
            let bx = LABEL_W + start_day * x_scale;
            let bw = (task.duration_days * x_scale).max(4.0);
            if task.status == TaskStatus::Milestone {
                items.push(LayoutedTemporalItem::MilestoneMarker {
                    x: bx, y: y + TASK_H / 2.0, label: task.label.clone(),
                });
            } else {
                items.push(LayoutedTemporalItem::TaskBar {
                    x: bx, y, width: bw, height: TASK_H,
                    status: task.status.clone(),
                    label: task.label.clone(),
                });
            }
            y += TASK_H + TASK_GAP;
        }
    }

    LayoutedTemporalDiagram { width: cw, height: y + 16.0, items }
}

// ── Git layout ────────────────────────────────────────────────────────────

fn layout_git(diagram: &GitDiagram, cw: f64) -> LayoutedTemporalDiagram {
    let mut items: Vec<LayoutedTemporalItem> = Vec::new();
    let mut branch_lanes: HashMap<String, usize> = HashMap::new();
    let mut commit_x: HashMap<String, (f64, f64)> = HashMap::new(); // id -> (x,y)
    let mut x_cursor = 60.0_f64;
    let mut current_branch = "main".to_string();

    // Pre-assign lanes in declaration order; default "main" to lane 0.
    if !diagram.branches.is_empty() {
        for (i, b) in diagram.branches.iter().enumerate() {
            branch_lanes.insert(b.name.clone(), i);
        }
    } else {
        branch_lanes.insert("main".into(), 0);
    }
    let mut next_lane = diagram.branches.len().max(1);

    let lane_y = |lane: usize| -> f64 { 30.0 + lane as f64 * LANE_H };

    // Emit branch lane labels.
    for (name, &lane) in &branch_lanes {
        let color = BRANCH_COLORS[lane % BRANCH_COLORS.len()].to_string();
        items.push(LayoutedTemporalItem::BranchLane {
            y: lane_y(lane), color, label: name.clone(),
        });
    }

    for event in &diagram.events {
        match event {
            GitEvent::Commit { id, message, tag, branch } => {
                let lane = *branch_lanes.entry(branch.clone()).or_insert_with(|| {
                    let l = next_lane; next_lane += 1; l
                });
                let cy = lane_y(lane);
                let cx = x_cursor;
                let commit_id = id.clone().unwrap_or_else(|| format!("c{:.0}", cx));
                commit_x.insert(commit_id.clone(), (cx, cy));
                items.push(LayoutedTemporalItem::CommitNode {
                    x: cx, y: cy,
                    id: commit_id,
                    message: message.clone(),
                    tag: tag.clone(),
                });
                x_cursor += COMMIT_SPACING;
            }
            GitEvent::Checkout { branch } => {
                current_branch = branch.clone();
                // Ensure lane exists.
                branch_lanes.entry(branch.clone()).or_insert_with(|| {
                    let l = next_lane; next_lane += 1; l
                });
            }
            GitEvent::Merge { from, id, tag } => {
                let from_lane = *branch_lanes.get(from).unwrap_or(&0);
                let to_lane   = *branch_lanes.get(&current_branch).unwrap_or(&0);
                let from_y    = lane_y(from_lane);
                let to_y      = lane_y(to_lane);
                items.push(LayoutedTemporalItem::MergeArc {
                    from_x: x_cursor - COMMIT_SPACING, from_y,
                    to_x:   x_cursor, to_y,
                });
                // The merge itself is a commit on the target branch.
                let commit_id = id.clone().unwrap_or_else(|| format!("m{:.0}", x_cursor));
                items.push(LayoutedTemporalItem::CommitNode {
                    x: x_cursor, y: to_y,
                    id: commit_id,
                    message: Some(format!("merge {from}")),
                    tag: tag.clone(),
                });
                x_cursor += COMMIT_SPACING;
            }
        }
    }

    let ch = lane_y(next_lane - 1) + LANE_H;
    LayoutedTemporalDiagram { width: cw.max(x_cursor + 60.0), height: ch, items }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::*;

    fn simple_gantt() -> TemporalDiagram {
        TemporalDiagram {
            kind: TemporalKind::Gantt,
            title: Some("Project".into()),
            body: TemporalBody::Gantt(GanttDiagram {
                date_format: "YYYY-MM-DD".into(),
                sections: vec![GanttSection {
                    label: Some("Phase 1".into()),
                    tasks: vec![
                        GanttTask {
                            id: "t1".into(), label: "Design".into(),
                            start: TaskStart::Date("2026-01-01".into()),
                            duration_days: 5.0,
                            status: TaskStatus::Done,
                            dependencies: vec![],
                        },
                        GanttTask {
                            id: "t2".into(), label: "Build".into(),
                            start: TaskStart::After("t1".into()),
                            duration_days: 3.0,
                            status: TaskStatus::Active,
                            dependencies: vec!["t1".into()],
                        },
                    ],
                }],
            }),
        }
    }

    fn simple_git() -> TemporalDiagram {
        TemporalDiagram {
            kind: TemporalKind::Git,
            title: None,
            body: TemporalBody::Git(GitDiagram {
                direction: DiagramDirection::LeftRight,
                branches: vec![GitBranch { name: "main".into() }],
                events: vec![
                    GitEvent::Commit {
                        id: Some("a1".into()), message: Some("init".into()),
                        tag: None, branch: "main".into(),
                    },
                    GitEvent::Commit {
                        id: Some("a2".into()), message: Some("feature".into()),
                        tag: None, branch: "main".into(),
                    },
                ],
            }),
        }
    }

    #[test] fn version_exists() { assert_eq!(crate::VERSION, "0.1.0"); }

    #[test]
    fn gantt_has_task_bars() {
        let d = layout_temporal_diagram(&simple_gantt(), 800.0);
        let bars = d.items.iter().filter(|i| matches!(i, LayoutedTemporalItem::TaskBar{..})).count();
        assert_eq!(bars, 2);
    }

    #[test]
    fn gantt_has_axis_spine() {
        let d = layout_temporal_diagram(&simple_gantt(), 800.0);
        assert!(d.items.iter().any(|i| matches!(i, LayoutedTemporalItem::TimeAxisSpine{..})));
    }

    #[test]
    fn after_dependency_resolves_correctly() {
        let d = layout_temporal_diagram(&simple_gantt(), 800.0);
        // t2 starts after t1 ends (5 days in), so its bar x should be > t1's bar x.
        let bars: Vec<_> = d.items.iter().filter_map(|i| {
            if let LayoutedTemporalItem::TaskBar { x, label, .. } = i { Some((*x, label.clone())) } else { None }
        }).collect();
        let t1_x = bars.iter().find(|(_, l)| l == "Design").unwrap().0;
        let t2_x = bars.iter().find(|(_, l)| l == "Build").unwrap().0;
        assert!(t2_x > t1_x, "Build should start after Design");
    }

    #[test]
    fn gantt_has_section_header() {
        let d = layout_temporal_diagram(&simple_gantt(), 800.0);
        assert!(d.items.iter().any(|i| {
            if let LayoutedTemporalItem::SectionHeader { label, .. } = i {
                label == "Phase 1"
            } else { false }
        }));
    }

    #[test]
    fn git_layout_produces_commits() {
        let d = layout_temporal_diagram(&simple_git(), 800.0);
        let commits = d.items.iter().filter(|i| matches!(i, LayoutedTemporalItem::CommitNode{..})).count();
        assert_eq!(commits, 2);
    }

    #[test]
    fn git_canvas_width_covers_commits() {
        let d = layout_temporal_diagram(&simple_git(), 800.0);
        assert!(d.width >= 2.0 * COMMIT_SPACING);
    }
}
