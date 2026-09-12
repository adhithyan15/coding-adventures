//! Deterministic, backend-neutral hierarchy layout.

use std::collections::HashMap;

use diagram_ir::{
    DiagramDirection, LayoutedSwimlaneDiagram, LayoutedSwimlaneEdge, LayoutedSwimlaneLane,
    LayoutedSwimlaneNode, LayoutedTreeViewDiagram, LayoutedTreeViewNode, LayoutedTreemapDiagram,
    LayoutedTreemapNode, Point, SwimlaneDiagram, TreeViewDiagram, TreemapDiagram,
    LayoutedRailroadDiagram, LayoutedRailroadElement, LayoutedRailroadPath, LayoutedRailroadRule,
    RailroadDiagram, RailroadElementKind, RailroadExpression,
};

pub const VERSION: &str = "0.1.0";

const OUTER_PADDING: f64 = 8.0;
const NODE_GAP: f64 = 3.0;
const PARENT_HEADER: f64 = 24.0;

/// Lay out explicit Railroad constructors into rule rows and branch paths.
pub fn layout_railroad(diagram: &RailroadDiagram) -> LayoutedRailroadDiagram {
    let title_height = if diagram.title.is_some() { 46.0 } else { 18.0 };
    let mut rules = Vec::new(); let mut y = title_height; let mut width: f64 = 420.0;
    for rule in &diagram.rules {
        let mut elements = Vec::new(); let mut paths = Vec::new();
        let (expr_width, expr_height) = layout_railroad_expression(&rule.definition, 140.0, y + 18.0, &mut elements, &mut paths);
        let center = y + 18.0 + expr_height / 2.0;
        paths.push(LayoutedRailroadPath { points: vec![Point { x: 116.0, y: center }, Point { x: 140.0, y: center }], loopback: false });
        paths.push(LayoutedRailroadPath { points: vec![Point { x: 140.0 + expr_width, y: center }, Point { x: 164.0 + expr_width, y: center }], loopback: false });
        let height = expr_height + 44.0; width = width.max(expr_width + 188.0);
        rules.push(LayoutedRailroadRule { name: rule.name.clone(), y, height, elements, paths }); y += height + 14.0;
    }
    LayoutedRailroadDiagram { width, height: y + 12.0, title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(), accessibility_description: diagram.accessibility_description.clone(), rules }
}

fn layout_railroad_expression(expr: &RailroadExpression, x: f64, y: f64,
    elements: &mut Vec<LayoutedRailroadElement>, paths: &mut Vec<LayoutedRailroadPath>) -> (f64, f64) {
    match expr {
        RailroadExpression::Terminal(label) | RailroadExpression::NonTerminal(label) | RailroadExpression::Special(label) => {
            let kind = match expr { RailroadExpression::Terminal(_) => RailroadElementKind::Terminal,
                RailroadExpression::NonTerminal(_) => RailroadElementKind::NonTerminal, _ => RailroadElementKind::Special };
            let width = (label.chars().count() as f64 * 8.5 + 28.0).clamp(64.0, 220.0);
            elements.push(LayoutedRailroadElement { kind, label: label.clone(), x, y, width, height: 36.0 }); (width, 36.0)
        }
        RailroadExpression::Sequence(items) => {
            let mut cursor = x; let mut max_height: f64 = 36.0; let mut previous_end = None;
            for item in items {
                let (item_width, item_height) = layout_railroad_expression(item, cursor, y, elements, paths);
                let center = y + item_height / 2.0;
                if let Some(end) = previous_end { paths.push(LayoutedRailroadPath { points: vec![Point { x: end, y: center }, Point { x: cursor, y: center }], loopback: false }); }
                previous_end = Some(cursor + item_width); cursor += item_width + 24.0; max_height = max_height.max(item_height);
            }
            ((cursor - x - 24.0).max(0.0), max_height)
        }
        RailroadExpression::Choice(alternatives) => {
            let gap = 14.0; let mut cursor_y = y; let mut max_width: f64 = 0.0; let mut sizes = Vec::new();
            for alternative in alternatives { let size = layout_railroad_expression(alternative, x + 28.0, cursor_y, elements, paths); sizes.push((cursor_y, size)); cursor_y += size.1 + gap; max_width = max_width.max(size.0); }
            let height = (cursor_y - y - gap).max(36.0); let middle = y + height / 2.0;
            for (alt_y, (alt_width, alt_height)) in sizes { let alt_center = alt_y + alt_height / 2.0;
                paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + 14.0, y: alt_center }, Point { x: x + 28.0, y: alt_center }], loopback: false });
                paths.push(LayoutedRailroadPath { points: vec![Point { x: x + 28.0 + alt_width, y: alt_center }, Point { x: x + 42.0 + max_width, y: alt_center }, Point { x: x + 56.0 + max_width, y: middle }], loopback: false }); }
            (max_width + 56.0, height)
        }
        RailroadExpression::Optional(element) => {
            let height = 64.0; let middle = y + height / 2.0; let bypass_y = y + 4.0;
            let (inner_width, _) = layout_railroad_expression(element, x + 24.0, y + 14.0, elements, paths);
            let exit_x = x + inner_width + 48.0;
            paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + 24.0, y: middle }], loopback: false });
            paths.push(LayoutedRailroadPath { points: vec![Point { x: x + 24.0 + inner_width, y: middle }, Point { x: exit_x, y: middle }], loopback: false });
            paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + 12.0, y: bypass_y }, Point { x: exit_x - 12.0, y: bypass_y }, Point { x: exit_x, y: middle }], loopback: false });
            (inner_width + 48.0, height)
        }
        RailroadExpression::Repetition { element, min } => {
            let height = 66.0; let middle = y + height / 2.0;
            let (inner_width, _) = layout_railroad_expression(element, x, y + 15.0, elements, paths); let loop_y = y + 60.0;
            paths.push(LayoutedRailroadPath { points: vec![Point { x: x + inner_width, y: middle }, Point { x: x + inner_width, y: loop_y }, Point { x, y: loop_y }, Point { x, y: middle }], loopback: true });
            if *min == 0 { paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + 12.0, y: y + 4.0 }, Point { x: x + inner_width - 12.0, y: y + 4.0 }, Point { x: x + inner_width, y: middle }], loopback: false }); }
            (inner_width, height)
        }
    }
}

/// Lay out top-level Mermaid subgraphs as stable ownership lanes.
pub fn layout_swimlane(diagram: &SwimlaneDiagram) -> LayoutedSwimlaneDiagram {
    let title_height = if diagram.title.is_some() { 44.0 } else { 16.0 };
    let horizontal_flow = matches!(
        diagram.direction,
        DiagramDirection::Lr | DiagramDirection::Rl
    );
    let lane_header = 38.0;
    let lane_gap = 12.0;
    let node_width = 132.0;
    let node_height = 54.0;
    let node_gap = 42.0;
    let max_nodes = diagram
        .lanes
        .iter()
        .map(|lane| lane.node_ids.len())
        .max()
        .unwrap_or(1)
        .max(1);
    let lane_cross = 150.0;
    let lane_along = lane_header + max_nodes as f64 * (node_width + node_gap) + 28.0;
    let (width, height) = if horizontal_flow {
        (
            lane_along.max(420.0),
            title_height + diagram.lanes.len() as f64 * (lane_cross + lane_gap) + 12.0,
        )
    } else {
        (
            diagram.lanes.len() as f64 * (lane_cross + lane_gap) + 12.0,
            title_height + lane_along.max(420.0),
        )
    };
    let mut lanes = Vec::new();
    let mut nodes = Vec::new();
    for (lane_index, lane) in diagram.lanes.iter().enumerate() {
        let (x, y, lane_width, lane_height) = if horizontal_flow {
            (
                12.0,
                title_height + lane_index as f64 * (lane_cross + lane_gap),
                width - 24.0,
                lane_cross,
            )
        } else {
            (
                12.0 + lane_index as f64 * (lane_cross + lane_gap),
                title_height,
                lane_cross,
                height - title_height - 12.0,
            )
        };
        lanes.push(LayoutedSwimlaneLane {
            id: lane.id.clone(),
            label: lane.label.clone(),
            x,
            y,
            width: lane_width,
            height: lane_height,
        });
        let mut member_ids = lane.node_ids.clone();
        if matches!(
            diagram.direction,
            DiagramDirection::Rl | DiagramDirection::Bt
        ) {
            member_ids.reverse();
        }
        for (node_index, node_id) in member_ids.iter().enumerate() {
            let Some(node) = diagram.nodes.iter().find(|node| &node.id == node_id) else {
                continue;
            };
            let (node_x, node_y) = if horizontal_flow {
                (
                    x + lane_header + 20.0 + node_index as f64 * (node_width + node_gap),
                    y + (lane_cross - node_height) / 2.0,
                )
            } else {
                (
                    x + (lane_cross - node_width) / 2.0,
                    y + lane_header + 20.0 + node_index as f64 * (node_height + node_gap),
                )
            };
            nodes.push(LayoutedSwimlaneNode {
                id: node.id.clone(),
                label: node.label.clone(),
                shape: node.shape.clone(),
                x: node_x,
                y: node_y,
                width: node_width,
                height: node_height,
            });
        }
    }
    let centers = nodes
        .iter()
        .map(|node| {
            (
                node.id.as_str(),
                Point {
                    x: node.x + node.width / 2.0,
                    y: node.y + node.height / 2.0,
                },
            )
        })
        .collect::<HashMap<_, _>>();
    let edges = diagram
        .edges
        .iter()
        .filter_map(|edge| {
            Some(LayoutedSwimlaneEdge {
                from: centers.get(edge.from.as_str())?.clone(),
                to: centers.get(edge.to.as_str())?.clone(),
                label: edge.label.clone(),
                kind: edge.kind,
            })
        })
        .collect();
    LayoutedSwimlaneDiagram {
        width,
        height,
        direction: diagram.direction.clone(),
        title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        lanes,
        nodes,
        edges,
    }
}
/// Lay out a treemap using stable alternating slice-and-dice partitions.
pub fn layout_treemap(diagram: &TreemapDiagram, canvas_width: f64) -> LayoutedTreemapDiagram {
    let width = canvas_width.max(320.0);
    let height = (width * 0.62).max(240.0);
    let title_height = if diagram.title.is_some() { 40.0 } else { 0.0 };
    let mut children = HashMap::<Option<&str>, Vec<usize>>::new();
    for (index, node) in diagram.nodes.iter().enumerate() {
        children
            .entry(node.parent_id.as_deref())
            .or_default()
            .push(index);
    }
    let mut totals = vec![0.0; diagram.nodes.len()];
    for index in (0..diagram.nodes.len()).rev() {
        let child_total = children
            .get(&Some(diagram.nodes[index].id.as_str()))
            .into_iter()
            .flatten()
            .map(|child| totals[*child])
            .sum::<f64>();
        totals[index] = diagram.nodes[index].value.unwrap_or(child_total).max(0.0);
        if totals[index] == 0.0 {
            totals[index] = child_total.max(1.0);
        }
    }

    let mut nodes = Vec::new();
    let roots = children.get(&None).cloned().unwrap_or_default();
    layout_siblings(
        diagram,
        &children,
        &totals,
        &roots,
        0,
        OUTER_PADDING,
        title_height + OUTER_PADDING,
        width - OUTER_PADDING * 2.0,
        height - title_height - OUTER_PADDING * 2.0,
        &mut nodes,
    );
    LayoutedTreemapDiagram {
        width,
        height,
        title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        nodes,
    }
}

/// Lay out a TreeView as deterministic indented rows.
pub fn layout_treeview(diagram: &TreeViewDiagram, canvas_width: f64) -> LayoutedTreeViewDiagram {
    let title_height = if diagram.title.is_some() { 42.0 } else { 12.0 };
    let row_height = 34.0;
    let width = canvas_width.max(360.0);
    let nodes = diagram.nodes.iter().enumerate().map(|(index, node)| {
        let x = 26.0 + node.depth as f64 * 42.0;
        LayoutedTreeViewNode {
            id: node.id.clone(),
            parent_id: node.parent_id.clone(),
            depth: node.depth,
            label: node.label.clone(),
            kind: node.kind.clone(),
            class_selector: node.class_selector.clone(),
            icon: node.icon.clone(),
            description: node.description.clone(),
            x,
            y: title_height + index as f64 * row_height,
            width: (width - x - 18.0).max(80.0),
            height: 28.0,
        }
    }).collect();
    LayoutedTreeViewDiagram {
        width,
        height: title_height + diagram.nodes.len() as f64 * row_height + 12.0,
        title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        nodes,
    }
}

#[allow(clippy::too_many_arguments)]
fn layout_siblings(
    diagram: &TreemapDiagram,
    children: &HashMap<Option<&str>, Vec<usize>>,
    totals: &[f64],
    siblings: &[usize],
    depth: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    output: &mut Vec<LayoutedTreemapNode>,
) {
    let total = siblings
        .iter()
        .map(|index| totals[*index])
        .sum::<f64>()
        .max(1.0);
    let mut cursor = if depth.is_multiple_of(2) { x } else { y };
    for (position, index) in siblings.iter().enumerate() {
        let share = totals[*index] / total;
        let remaining = position + 1 == siblings.len();
        let (node_x, node_y, node_width, node_height) = if depth.is_multiple_of(2) {
            let extent = if remaining {
                x + width - cursor
            } else {
                width * share
            };
            let result = (cursor, y, extent, height);
            cursor += extent;
            result
        } else {
            let extent = if remaining {
                y + height - cursor
            } else {
                height * share
            };
            let result = (x, cursor, width, extent);
            cursor += extent;
            result
        };
        let node = &diagram.nodes[*index];
        let child_indices = children.get(&Some(node.id.as_str()));
        output.push(LayoutedTreemapNode {
            id: node.id.clone(),
            label: node.label.clone(),
            value: totals[*index],
            depth,
            x: node_x + NODE_GAP,
            y: node_y + NODE_GAP,
            width: (node_width - NODE_GAP * 2.0).max(0.0),
            height: (node_height - NODE_GAP * 2.0).max(0.0),
            class_selector: node.class_selector.clone(),
        });
        if let Some(child_indices) = child_indices {
            layout_siblings(
                diagram,
                children,
                totals,
                child_indices,
                depth + 1,
                node_x + NODE_GAP * 2.0,
                node_y + PARENT_HEADER,
                (node_width - NODE_GAP * 4.0).max(0.0),
                (node_height - PARENT_HEADER - NODE_GAP).max(0.0),
                output,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::TreemapNode;

    #[test]
    fn child_areas_follow_values_and_stay_inside_parent() {
        let diagram = TreemapDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            nodes: vec![
                TreemapNode {
                    id: "root".into(),
                    label: "Root".into(),
                    value: None,
                    class_selector: None,
                    parent_id: None,
                },
                TreemapNode {
                    id: "a".into(),
                    label: "A".into(),
                    value: Some(1.0),
                    class_selector: None,
                    parent_id: Some("root".into()),
                },
                TreemapNode {
                    id: "b".into(),
                    label: "B".into(),
                    value: Some(3.0),
                    class_selector: None,
                    parent_id: Some("root".into()),
                },
            ],
        };
        let layout = layout_treemap(&diagram, 400.0);
        assert_eq!(layout.nodes.len(), 3);
        assert!(layout.nodes[2].height > layout.nodes[1].height * 2.5);
        assert!(layout
            .nodes
            .iter()
            .all(|node| node.width >= 0.0 && node.height >= 0.0));
    }

    #[test]
    fn treeview_layout_indents_children_and_preserves_rows() {
        use diagram_ir::{TreeViewDiagram, TreeViewNode, TreeViewNodeKind};
        let diagram = TreeViewDiagram {
            title: None, accessibility_title: None, accessibility_description: None,
            nodes: vec![
                TreeViewNode { id: "root".into(), parent_id: None, depth: 0, label: "src".into(), kind: TreeViewNodeKind::Directory, class_selector: None, icon: None, description: None },
                TreeViewNode { id: "child".into(), parent_id: Some("root".into()), depth: 1, label: "main.rs".into(), kind: TreeViewNodeKind::File, class_selector: None, icon: None, description: None },
            ],
        };
        let layout = layout_treeview(&diagram, 500.0);
        assert!(layout.nodes[1].x > layout.nodes[0].x);
        assert!(layout.nodes[1].y > layout.nodes[0].y);
        assert_eq!(layout.nodes[1].parent_id.as_deref(), Some("root"));
    }

    #[test]
    fn swimlane_layout_keeps_nodes_inside_ownership_bands() {
        use diagram_ir::{DiagramShape, SwimlaneDiagram, SwimlaneLane, SwimlaneNode};
        let diagram = SwimlaneDiagram {
            direction: DiagramDirection::Lr, title: Some("Handoff".into()), accessibility_title: None,
            accessibility_description: None,
            lanes: vec![
                SwimlaneLane { id: "buyer".into(), label: "Buyer".into(), node_ids: vec!["choose".into()] },
                SwimlaneLane { id: "store".into(), label: "Store".into(), node_ids: vec!["ship".into()] },
            ],
            nodes: vec![
                SwimlaneNode { id: "choose".into(), label: "Choose".into(), lane_id: Some("buyer".into()), shape: DiagramShape::Rect },
                SwimlaneNode { id: "ship".into(), label: "Ship".into(), lane_id: Some("store".into()), shape: DiagramShape::RoundedRect },
            ], edges: Vec::new(),
        };
        let layout = layout_swimlane(&diagram);
        assert!(layout.lanes[1].y > layout.lanes[0].y);
        for (node, lane) in layout.nodes.iter().zip(layout.lanes.iter()) {
            assert!(node.x >= lane.x && node.x + node.width <= lane.x + lane.width);
            assert!(node.y >= lane.y && node.y + node.height <= lane.y + lane.height);
        }
    }

    #[test]
    fn railroad_layout_preserves_choice_branches_and_repetition_loopbacks() {
        use diagram_ir::{RailroadDiagram, RailroadExpression, RailroadRule};
        let diagram = RailroadDiagram {
            title: Some("Number".into()), accessibility_title: None, accessibility_description: None,
            rules: vec![RailroadRule { name: "number".into(), definition: RailroadExpression::Sequence(vec![
                RailroadExpression::Choice(vec![RailroadExpression::Terminal("0".into()), RailroadExpression::Terminal("1".into())]),
                RailroadExpression::Repetition { element: Box::new(RailroadExpression::NonTerminal("digit".into())), min: 1 },
            ]) }],
        };
        let layout = layout_railroad(&diagram);
        assert_eq!(layout.rules.len(), 1);
        assert_eq!(layout.rules[0].elements.len(), 3);
        assert!(layout.rules[0].paths.iter().any(|path| path.loopback));
        assert!(layout.rules[0].elements[1].y > layout.rules[0].elements[0].y);
        assert!(layout.width >= 420.0 && layout.height > 100.0);
    }
}
