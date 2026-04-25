use diagram_ir::{
    GanttDiagram, GanttTask, LayoutedTemporalDiagram, LayoutedTemporalItem, TaskStart,
    TaskStatus, TemporalBody, TemporalDiagram,
};

pub const VERSION: &str = "0.1.0";

const MARGIN: f64 = 20.0;
const TITLE_H: f64 = 30.0;
const AXIS_H: f64 = 28.0;
const SECTION_H: f64 = 24.0;
const TASK_H: f64 = 20.0;
const TASK_GAP: f64 = 4.0;
const LABEL_W: f64 = 120.0;
const TICK_DAYS: f64 = 7.0;
const LANE_H: f64 = 60.0;
const COMMIT_SPACING: f64 = 80.0;
const BRANCH_COLORS: &[&str] = &["#3b82f6","#ef4444","#22c55e","#f59e0b","#a855f7","#14b8a6"];

pub fn layout_temporal_diagram(diagram: &TemporalDiagram, canvas_width: f64) -> LayoutedTemporalDiagram {
    match &diagram.body {
        TemporalBody::Gantt(g) => layout_gantt(diagram.title.as_deref(), g, canvas_width),
        TemporalBody::Git(g) => layout_git(diagram.title.as_deref(), g, canvas_width),
    }
}

struct ResolvedTask { id: String, label: String, start_day: f64, duration: f64, status: TaskStatus }

fn parse_date(date: &str) -> f64 {
    let p: Vec<&str> = date.trim().split('-').collect();
    if p.len() != 3 { return 0.0; }
    let y: f64 = p[0].parse().unwrap_or(2026.0);
    let m: f64 = p[1].parse().unwrap_or(1.0);
    let d: f64 = p[2].parse().unwrap_or(1.0);
    y * 365.25 + (m-1.0)*30.44 + d
}

fn resolve_gantt_tasks(gantt: &GanttDiagram) -> Vec<ResolvedTask> {
    let all: Vec<&GanttTask> = gantt.sections.iter().flat_map(|s| s.tasks.iter()).collect();
    let mut starts: std::collections::HashMap<String,f64> = std::collections::HashMap::new();
    for _ in 0..all.len()+1 {
        for t in &all {
            let s = match &t.start {
                TaskStart::Date(d) => parse_date(d),
                TaskStart::After(dep) => {
                    let ds = starts.get(dep.as_str()).copied().unwrap_or(0.0);
                    let dd = all.iter().find(|dt| dt.id == *dep).map(|dt| dt.duration_days).unwrap_or(0.0);
                    ds + dd
                }
            };
            starts.insert(t.id.clone(), s);
        }
    }
    all.iter().map(|t| ResolvedTask { id:t.id.clone(), label:t.label.clone(), start_day:*starts.get(&t.id).unwrap_or(&0.0), duration:t.duration_days, status:t.status.clone() }).collect()
}

fn layout_gantt(title: Option<&str>, gantt: &GanttDiagram, cw: f64) -> LayoutedTemporalDiagram {
    let resolved = resolve_gantt_tasks(gantt);
    let t_min = resolved.iter().map(|t| t.start_day).fold(f64::INFINITY, f64::min);
    let t_max = resolved.iter().map(|t| t.start_day+t.duration).fold(f64::NEG_INFINITY, f64::max);
    let t_min = if t_min.is_infinite() { 0.0 } else { t_min };
    let t_max = if t_max.is_infinite() { t_min+30.0 } else { t_max };
    let t_range = (t_max-t_min).max(1.0);
    let plot_left = MARGIN+LABEL_W; let plot_right = cw-MARGIN; let plot_w = (plot_right-plot_left).max(1.0);
    let x_scale = plot_w/t_range;
    let day_to_x = |day: f64| plot_left + (day-t_min)*x_scale;
    let has_title = title.is_some();
    let mut y = MARGIN + if has_title { TITLE_H } else { 0.0 };
    let mut items: Vec<LayoutedTemporalItem> = Vec::new();
    if let Some(t) = title { items.push(LayoutedTemporalItem::TimeAxisTick { x:cw/2.0, y:MARGIN+TITLE_H/2.0, label:t.to_string() }); }
    let axis_y = y+AXIS_H/2.0;
    items.push(LayoutedTemporalItem::TimeAxisSpine { x1:plot_left, y1:axis_y, x2:plot_right, y2:axis_y });
    let first_tick = (t_min/TICK_DAYS).ceil()*TICK_DAYS;
    let mut tick = first_tick;
    while tick <= t_max { items.push(LayoutedTemporalItem::TimeAxisTick { x:day_to_x(tick), y:y+4.0, label:format!("+{}d",(tick-t_min).round() as i64) }); tick += TICK_DAYS; }
    y += AXIS_H;
    let all_rows: usize = gantt.sections.iter().map(|s| 1+s.tasks.len()).sum();
    let total_h = MARGIN + if has_title { TITLE_H } else { 0.0 } + all_rows as f64*(TASK_H+TASK_GAP) + AXIS_H + MARGIN;
    for section in &gantt.sections {
        if let Some(ref lbl) = section.label {
            items.push(LayoutedTemporalItem::SectionHeader { x:MARGIN, y, width:cw-MARGIN*2.0, height:SECTION_H, label:lbl.clone() });
            y += SECTION_H+TASK_GAP;
        }
        for task in &section.tasks {
            if let Some(rt) = resolved.iter().find(|r| r.id == task.id) {
                let bar_x = day_to_x(rt.start_day); let bar_w = (rt.duration*x_scale).max(2.0);
                if rt.status == TaskStatus::Milestone {
                    items.push(LayoutedTemporalItem::MilestoneMarker { x:bar_x, y:y+TASK_H/2.0, label:rt.label.clone() });
                } else {
                    items.push(LayoutedTemporalItem::TaskBar { x:bar_x, y, width:bar_w, height:TASK_H, status:rt.status.clone(), label:rt.label.clone() });
                }
            }
            y += TASK_H+TASK_GAP;
        }
    }
    LayoutedTemporalDiagram { width:cw, height:total_h, items }
}

fn layout_git(title: Option<&str>, git: &diagram_ir::GitDiagram, cw: f64) -> LayoutedTemporalDiagram {
    use diagram_ir::GitEvent;
    let has_title = title.is_some();
    let top = MARGIN + if has_title { TITLE_H } else { 0.0 };
    let mut items: Vec<LayoutedTemporalItem> = Vec::new();
    if let Some(t) = title { items.push(LayoutedTemporalItem::TimeAxisTick { x:cw/2.0, y:MARGIN+TITLE_H/2.0, label:t.to_string() }); }
    let branch_names: Vec<String> = git.branches.iter().map(|b| b.name.clone()).collect();
    for (i, name) in branch_names.iter().enumerate() {
        items.push(LayoutedTemporalItem::BranchLane { y:top+i as f64*LANE_H+LANE_H/2.0, color:BRANCH_COLORS[i%BRANCH_COLORS.len()].to_string(), label:name.clone() });
    }
    let mut active_branch = branch_names.first().cloned().unwrap_or_default();
    let mut cx_per: std::collections::HashMap<String,f64> = branch_names.iter().map(|b| (b.clone(), MARGIN+LABEL_W)).collect();
    let mut last_pos: std::collections::HashMap<String,(f64,f64)> = std::collections::HashMap::new();
    for event in &git.events {
        match event {
            GitEvent::Checkout { branch } => { active_branch = branch.clone(); }
            GitEvent::Commit { id, message, tag, branch } => {
                let b = if branch.is_empty() { &active_branch } else { branch };
                let li = branch_names.iter().position(|n| n == b).unwrap_or(0);
                let ly = top + li as f64 * LANE_H + LANE_H/2.0;
                let cx = cx_per.entry(b.clone()).or_insert(MARGIN+LABEL_W);
                let cid = id.clone().unwrap_or_else(|| format!("c{}", *cx as i64));
                items.push(LayoutedTemporalItem::CommitNode { x:*cx, y:ly, id:cid, message:message.clone(), tag:tag.clone() });
                last_pos.insert(b.clone(), (*cx, ly));
                *cx += COMMIT_SPACING;
            }
            GitEvent::Merge { from, id, tag } => {
                let tli = branch_names.iter().position(|n| n == &active_branch).unwrap_or(0);
                let fli = branch_names.iter().position(|n| n == from).unwrap_or(0);
                let ty = top+tli as f64*LANE_H+LANE_H/2.0; let fy = top+fli as f64*LANE_H+LANE_H/2.0;
                let (fx, _) = last_pos.get(from.as_str()).copied().unwrap_or((MARGIN+LABEL_W,fy));
                let to_cx = cx_per.entry(active_branch.clone()).or_insert(MARGIN+LABEL_W);
                let tx = *to_cx;
                items.push(LayoutedTemporalItem::MergeArc { from_x:fx, from_y:fy, to_x:tx, to_y:ty });
                items.push(LayoutedTemporalItem::CommitNode { x:tx, y:ty, id:id.clone().unwrap_or("merge".into()), message:None, tag:tag.clone() });
                last_pos.insert(active_branch.clone(), (tx, ty));
                *to_cx += COMMIT_SPACING;
            }
        }
    }
    let total_h = top + branch_names.len().max(1) as f64 * LANE_H + MARGIN;
    LayoutedTemporalDiagram { width:cw, height:total_h, items }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::{GanttDiagram,GanttSection,GanttTask,TaskStart,TaskStatus,TemporalBody,TemporalDiagram,TemporalKind};
    fn simple_gantt() -> TemporalDiagram {
        TemporalDiagram { kind:TemporalKind::Gantt, title:Some("Test".into()), body:TemporalBody::Gantt(GanttDiagram { date_format:"YYYY-MM-DD".into(), sections:vec![GanttSection { label:Some("P1".into()), tasks:vec![
            GanttTask{id:"t1".into(),label:"Design".into(),start:TaskStart::Date("2026-01-01".into()),duration_days:5.0,status:TaskStatus::Done,dependencies:vec![]},
            GanttTask{id:"t2".into(),label:"Implement".into(),start:TaskStart::After("t1".into()),duration_days:10.0,status:TaskStatus::Active,dependencies:vec!["t1".into()]},
        ]}]})}
    }
    #[test] fn version_exists() { assert_eq!(VERSION, "0.1.0"); }
    #[test] fn gantt_has_task_bars() { let out = layout_temporal_diagram(&simple_gantt(), 800.0); assert_eq!(out.items.iter().filter(|i| matches!(i,LayoutedTemporalItem::TaskBar{..})).count(), 2); }
    #[test] fn gantt_has_axis_spine() { let out = layout_temporal_diagram(&simple_gantt(), 800.0); assert!(out.items.iter().any(|i| matches!(i,LayoutedTemporalItem::TimeAxisSpine{..}))); }
    #[test] fn after_dependency_resolves_correctly() {
        let out = layout_temporal_diagram(&simple_gantt(), 800.0);
        let bars: Vec<_> = out.items.iter().filter_map(|i| if let LayoutedTemporalItem::TaskBar{x,label,..}=i { Some((*x,label.clone())) } else { None }).collect();
        let t1x = bars.iter().find(|(_,l)| l=="Design").map(|(x,_)| *x).unwrap();
        let t2x = bars.iter().find(|(_,l)| l=="Implement").map(|(x,_)| *x).unwrap();
        assert!(t2x > t1x, "t2_x={t2x} should be > t1_x={t1x}");
    }
    #[test] fn gantt_has_section_header() { let out = layout_temporal_diagram(&simple_gantt(), 800.0); assert!(out.items.iter().any(|i| matches!(i,LayoutedTemporalItem::SectionHeader{..}))); }
    #[test] fn git_layout_produces_commits() {
        use diagram_ir::{DiagramDirection,GitBranch,GitDiagram,GitEvent,TemporalDiagram};
        let d = TemporalDiagram { kind:TemporalKind::Git, title:None, body:TemporalBody::Git(GitDiagram { direction:DiagramDirection::Lr, branches:vec![GitBranch{name:"main".into()},GitBranch{name:"feat".into()}],
            events:vec![GitEvent::Commit{id:Some("c1".into()),message:None,tag:None,branch:"main".into()},
                        GitEvent::Commit{id:Some("c2".into()),message:None,tag:None,branch:"feat".into()},
                        GitEvent::Checkout{branch:"main".into()},
                        GitEvent::Merge{from:"feat".into(),id:None,tag:None}] }) };
        let out = layout_temporal_diagram(&d, 600.0);
        assert!(out.items.iter().filter(|i| matches!(i,LayoutedTemporalItem::CommitNode{..})).count() >= 2);
        assert_eq!(out.items.iter().filter(|i| matches!(i,LayoutedTemporalItem::MergeArc{..})).count(), 1);
    }
}
