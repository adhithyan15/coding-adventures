//! Deterministic, backend-neutral hierarchy layout.

use std::collections::{BTreeMap, HashMap};

use diagram_ir::{
    DiagramDirection, DiagramIconGlyph, LayoutedSwimlaneDiagram, LayoutedSwimlaneEdge, LayoutedSwimlaneLane,
    LayoutedSwimlaneNode, LayoutedTreeViewConnector, LayoutedTreeViewDiagram, LayoutedTreeViewNode, LayoutedTreemapDiagram,
    LayoutedTreemapNode, Point, SwimlaneDiagram, TreeViewDiagram, TreemapDiagram,
    LayoutedRailroadDiagram, LayoutedRailroadElement, LayoutedRailroadPath, LayoutedRailroadRule,
    RailroadDiagram, RailroadElementKind, RailroadExpression,
};

pub const VERSION: &str = "0.4.0";

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
            let sizes: Vec<_> = items.iter().map(measure_railroad_expression).collect();
            let max_height = sizes.iter().map(|size| size.1).fold(36.0, f64::max);
            let center = y + max_height / 2.0;
            let mut cursor = x; let mut previous_end = None;
            for (item, (_, item_height)) in items.iter().zip(sizes) {
                let (item_width, _) = layout_railroad_expression(item, cursor, y + (max_height - item_height) / 2.0, elements, paths);
                if let Some(end) = previous_end { paths.push(LayoutedRailroadPath { points: vec![Point { x: end, y: center }, Point { x: cursor, y: center }], loopback: false }); }
                previous_end = Some(cursor + item_width); cursor += item_width + 24.0;
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
            let inner_y = y + 14.0;
            let (inner_width, inner_height) = layout_railroad_expression(element, x + 24.0, inner_y, elements, paths);
            let height = inner_height + 28.0; let middle = inner_y + inner_height / 2.0; let bypass_y = y + 4.0;
            let exit_x = x + inner_width + 48.0;
            paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + 24.0, y: middle }], loopback: false });
            paths.push(LayoutedRailroadPath { points: vec![Point { x: x + 24.0 + inner_width, y: middle }, Point { x: exit_x, y: middle }], loopback: false });
            paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + 12.0, y: bypass_y }, Point { x: exit_x - 12.0, y: bypass_y }, Point { x: exit_x, y: middle }], loopback: false });
            (inner_width + 48.0, height)
        }
        RailroadExpression::Repetition { element, min, .. } => {
            let inner_y = y + 15.0;
            let (inner_width, inner_height) = layout_railroad_expression(element, x, inner_y, elements, paths);
            let height = inner_height + 30.0; let middle = inner_y + inner_height / 2.0; let loop_y = inner_y + inner_height + 9.0;
            paths.push(LayoutedRailroadPath { points: vec![Point { x: x + inner_width, y: middle }, Point { x: x + inner_width, y: loop_y }, Point { x, y: loop_y }, Point { x, y: middle }], loopback: true });
            if *min == 0 { paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + 12.0, y: y + 4.0 }, Point { x: x + inner_width - 12.0, y: y + 4.0 }, Point { x: x + inner_width, y: middle }], loopback: false }); }
            (inner_width, height)
        }
    }
}

fn measure_railroad_expression(expr: &RailroadExpression) -> (f64, f64) {
    match expr {
        RailroadExpression::Terminal(label) | RailroadExpression::NonTerminal(label) | RailroadExpression::Special(label) => {
            ((label.chars().count() as f64 * 8.5 + 28.0).clamp(64.0, 220.0), 36.0)
        }
        RailroadExpression::Sequence(items) => {
            let sizes: Vec<_> = items.iter().map(measure_railroad_expression).collect();
            let width = sizes.iter().map(|size| size.0).sum::<f64>() + 24.0 * items.len().saturating_sub(1) as f64;
            let height = sizes.iter().map(|size| size.1).fold(36.0, f64::max);
            (width, height)
        }
        RailroadExpression::Choice(alternatives) => {
            let sizes: Vec<_> = alternatives.iter().map(measure_railroad_expression).collect();
            let width = sizes.iter().map(|size| size.0).fold(0.0, f64::max) + 56.0;
            let height = sizes.iter().map(|size| size.1).sum::<f64>()
                + 14.0 * alternatives.len().saturating_sub(1) as f64;
            (width, height.max(36.0))
        }
        RailroadExpression::Optional(element) => {
            let (width, height) = measure_railroad_expression(element);
            (width + 48.0, height + 28.0)
        }
        RailroadExpression::Repetition { element, .. } => {
            let (width, height) = measure_railroad_expression(element);
            (width, height + 30.0)
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
    let lane_along = if horizontal_flow {
        diagram.lanes.iter().map(|lane| {
            let widths = lane.node_ids.iter().filter_map(|node_id| {
                diagram.nodes.iter().find(|node| &node.id == node_id)
                    .map(|node| swimlane_node_width(&node.label))
            }).collect::<Vec<_>>();
            lane_header + widths.iter().sum::<f64>()
                + node_gap * widths.len().saturating_sub(1) as f64 + 40.0
        }).fold(0.0, f64::max)
    } else {
        lane_header + max_nodes as f64 * (node_height + node_gap) + 28.0
    };
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
        let mut horizontal_cursor = x + lane_header + 20.0;
        for (node_index, node_id) in member_ids.iter().enumerate() {
            let Some(node) = diagram.nodes.iter().find(|node| &node.id == node_id) else {
                continue;
            };
            let resolved_node_width = if horizontal_flow {
                swimlane_node_width(&node.label)
            } else {
                node_width
            };
            let (node_x, node_y) = if horizontal_flow {
                (
                    horizontal_cursor,
                    y + (lane_cross - node_height) / 2.0,
                )
            } else {
                (
                    x + (lane_cross - resolved_node_width) / 2.0,
                    y + lane_header + 20.0 + node_index as f64 * (node_height + node_gap),
                )
            };
            nodes.push(LayoutedSwimlaneNode {
                id: node.id.clone(),
                label: node.label.clone(),
                shape: node.shape.clone(),
                x: node_x,
                y: node_y,
                width: resolved_node_width,
                height: node_height,
            });
            horizontal_cursor += resolved_node_width + node_gap;
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

fn swimlane_node_width(label: &str) -> f64 {
    (label.chars().count() as f64 * 7.2 + 28.0).clamp(132.0, 260.0)
}
/// Lay out a treemap using stable alternating slice-and-dice partitions.
pub fn layout_treemap(diagram: &TreemapDiagram, _canvas_width: f64) -> LayoutedTreemapDiagram {
    let width = (diagram.config.node_width * 10.0).max(1.0);
    let height = (diagram.config.node_height * 10.0).max(1.0);
    let outer_padding = diagram.config.diagram_padding;
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

    let mut color_domain = HashMap::<String, usize>::new();
    let mut next_color = 0usize;
    for node in &diagram.nodes {
        if children.get(&Some(node.id.as_str())).is_some_and(|children| !children.is_empty()) {
            color_domain.entry(node.label.clone()).or_insert_with(|| {
                let index = next_color;
                next_color += 1;
                index
            });
        }
    }
    let palette_indices = diagram.nodes.iter().map(|node| {
        let key = node.parent_id.as_deref()
            .and_then(|parent_id| diagram.nodes.iter().find(|candidate| candidate.id == parent_id))
            .map_or(node.label.as_str(), |parent| parent.label.as_str());
        let ordinal = *color_domain.entry(key.to_string()).or_insert_with(|| {
            let index = next_color;
            next_color += 1;
            index
        });
        ordinal.checked_sub(1).map(|index| index % 12)
    }).collect::<Vec<_>>();

    let mut nodes = Vec::new();
    let roots = children.get(&None).cloned().unwrap_or_default();
    layout_siblings(
        diagram,
        &children,
        &totals,
        &palette_indices,
        &roots,
        0,
        outer_padding,
        title_height + outer_padding,
        (width - outer_padding * 2.0).max(0.0),
        (height - title_height - outer_padding * 2.0).max(0.0),
        &mut nodes,
    );
    LayoutedTreemapDiagram {
        width,
        height,
        config: diagram.config.clone(),
        title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        nodes,
    }
}

/// Integrator-owned icon glyphs keyed by fully qualified Mermaid icon identity.
#[derive(Clone, Debug, Default)]
pub struct TreeViewLayoutOptions {
    pub icon_glyphs: BTreeMap<String, DiagramIconGlyph>,
}

/// Lay out a TreeView as deterministic indented rows.
pub fn layout_treeview(diagram: &TreeViewDiagram, canvas_width: f64) -> LayoutedTreeViewDiagram {
    layout_treeview_with_options(diagram, canvas_width, None)
}

/// Lay out a TreeView while resolving registered external icon identities.
pub fn layout_treeview_with_options(
    diagram: &TreeViewDiagram,
    _canvas_width: f64,
    options: Option<&TreeViewLayoutOptions>,
) -> LayoutedTreeViewDiagram {
    let row_height = diagram.config.theme.label_font_size * 1.2 + diagram.config.padding_y * 2.0;
    let mut nodes = diagram.nodes.iter().enumerate().map(|(index, node)| {
        let x = 26.0 + node.depth as f64 * (diagram.config.row_indent + diagram.config.padding_x);
        let show_icon = match node.icon.as_deref() { Some("none") => false, Some(_) => true, None => diagram.config.show_icons };
        let label_x = x + diagram.config.padding_x + if show_icon { 18.0 } else { 0.0 };
        let label_width = node.label.chars().count() as f64 * diagram.config.theme.label_font_size * 0.62 + 8.0;
        LayoutedTreeViewNode {
            id: node.id.clone(),
            is_implicit_root: node.is_implicit_root,
            parent_id: node.parent_id.clone(),
            depth: node.depth,
            label: node.label.clone(),
            kind: node.kind.clone(),
            class_selector: node.class_selector.clone(),
            icon: node.icon.clone(),
            icon_glyph: node.icon.as_ref().and_then(|icon|
                options.and_then(|options| options.icon_glyphs.get(icon))).cloned(),
            description: node.description.clone(),
            x,
            y: index as f64 * row_height,
            width: 0.0,
            height: row_height,
            label_x,
            label_width,
            description_x: None,
            description_width: node.description.as_ref().map(|description|
                description.chars().count() as f64 * diagram.config.theme.label_font_size * 0.58 + 8.0),
        }
    }).collect::<Vec<_>>();
    if nodes.iter().any(|node| node.description.is_some()) {
        let description_x = nodes.iter().map(|node| node.label_x + node.label_width).fold(0.0, f64::max) + 16.0;
        for node in &mut nodes {
            if node.description.is_some() { node.description_x = Some(description_x); }
        }
    }
    let content_width = nodes.iter().map(|node| match (node.description_x, node.description_width) {
        (Some(x), Some(width)) => x + width + diagram.config.padding_x,
        _ => node.label_x + node.label_width + diagram.config.padding_x,
    }).fold(diagram.config.line_thickness.max(1.0), f64::max);
    let has_highlight = nodes.iter().any(|node| node.class_selector.as_deref()
        .is_some_and(|classes| classes.split_whitespace().any(|class| class == "highlight")));
    let width = content_width + if has_highlight { 10.0 } else { 0.0 };
    for node in &mut nodes {
        node.width = (width - node.x - 2.0).max(0.0);
    }
    let mut connectors = Vec::new();
    for node in &nodes {
        let center_y = node.y + node.height / 2.0;
        connectors.push(LayoutedTreeViewConnector { node_id: node.id.clone(), points: vec![
            Point { x: node.x - diagram.config.row_indent, y: center_y }, Point { x: node.x, y: center_y },
        ] });
        if let Some(last_child) = nodes.iter().rev().find(|candidate| candidate.parent_id.as_deref() == Some(node.id.as_str())) {
            connectors.push(LayoutedTreeViewConnector { node_id: node.id.clone(), points: vec![
                Point { x: node.x + diagram.config.padding_x, y: node.y + node.height },
                Point { x: node.x + diagram.config.padding_x,
                    y: last_child.y + last_child.height / 2.0 + diagram.config.line_thickness / 2.0 },
            ] });
        }
    }
    LayoutedTreeViewDiagram {
        width,
        height: diagram.nodes.len() as f64 * row_height,
        title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        config: diagram.config.clone(),
        nodes,
        connectors,
    }
}

#[allow(clippy::too_many_arguments)]
fn layout_siblings(
    diagram: &TreemapDiagram,
    children: &HashMap<Option<&str>, Vec<usize>>,
    totals: &[f64],
    palette_indices: &[Option<usize>],
    siblings: &[usize],
    depth: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    output: &mut Vec<LayoutedTreemapNode>,
) {
    let node_gap = diagram.config.padding / 2.0;
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
            has_children: child_indices.is_some_and(|children| !children.is_empty()),
            palette_index: palette_indices[*index],
            x: node_x + node_gap,
            y: node_y + node_gap,
            width: (node_width - node_gap * 2.0).max(0.0),
            height: (node_height - node_gap * 2.0).max(0.0),
            class_selector: node.class_selector.clone(),
            style: node.style.clone(),
        });
        if let Some(child_indices) = child_indices {
            layout_siblings(
                diagram,
                children,
                totals,
                palette_indices,
                child_indices,
                depth + 1,
                node_x + node_gap * 2.0,
                node_y + PARENT_HEADER,
                (node_width - node_gap * 4.0).max(0.0),
                (node_height - PARENT_HEADER - node_gap).max(0.0),
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
            config: Default::default(),
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            nodes: vec![
                TreemapNode {
                    id: "root".into(),
                    label: "Root".into(),
                    value: None,
                    class_selector: None,
                    style: None,
                    parent_id: None,
                },
                TreemapNode {
                    id: "a".into(),
                    label: "A".into(),
                    value: Some(1.0),
                    class_selector: None,
                    style: None,
                    parent_id: Some("root".into()),
                },
                TreemapNode {
                    id: "b".into(),
                    label: "B".into(),
                    value: Some(3.0),
                    class_selector: None,
                    style: None,
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
        assert_eq!((layout.width, layout.height), (1000.0, 400.0));
    }

    #[test]
    fn treemap_configuration_controls_canvas_and_spacing() {
        let diagram = TreemapDiagram {
            config: diagram_ir::TreemapConfig { node_width: 64.0, node_height: 48.0, diagram_padding: 20.0, padding: 4.0, ..Default::default() },
            title: None, accessibility_title: None, accessibility_description: None,
            nodes: vec![TreemapNode { id: "root".into(), label: "Root".into(), value: Some(1.0), class_selector: None, style: None, parent_id: None }],
        };
        let layout = layout_treemap(&diagram, 999.0);
        assert_eq!((layout.width, layout.height), (640.0, 480.0));
        assert_eq!((layout.nodes[0].x, layout.nodes[0].y), (22.0, 22.0));
        assert_eq!(layout.config.padding, 4.0);
    }

    #[test]
    fn treeview_layout_indents_children_and_preserves_rows() {
        use diagram_ir::{TreeViewDiagram, TreeViewNode, TreeViewNodeKind};
        let diagram = TreeViewDiagram {
            title: None, accessibility_title: None, accessibility_description: None,
            config: diagram_ir::TreeViewConfig::default(),
            nodes: vec![
                TreeViewNode { id: "root".into(), is_implicit_root: true, parent_id: None, depth: 0, label: "/".into(), kind: TreeViewNodeKind::Directory, class_selector: Some("selected highlight".into()), icon: None, description: None },
                TreeViewNode { id: "child".into(), is_implicit_root: false, parent_id: Some("root".into()), depth: 1, label: "a.rs".into(), kind: TreeViewNodeKind::File, class_selector: None, icon: None, description: Some("short".into()) },
                TreeViewNode { id: "sibling".into(), is_implicit_root: false, parent_id: Some("root".into()), depth: 1, label: "longer-name.rs".into(), kind: TreeViewNodeKind::File, class_selector: None, icon: None, description: Some("long".into()) },
            ],
        };
        let layout = layout_treeview(&diagram, 500.0);
        assert!(layout.nodes[1].x > layout.nodes[0].x);
        assert!(layout.nodes[1].y > layout.nodes[0].y);
        assert_eq!(layout.nodes[1].parent_id.as_deref(), Some("root"));
        assert_eq!(layout.nodes[1].x - layout.nodes[0].x, 15.0);
        assert_eq!(layout.nodes[0].height, 29.2);
        assert_eq!(layout.connectors.len(), 4);
        assert_eq!(layout.connectors[0].points[0].x, 16.0);
        assert_eq!(layout.connectors[1].points[0].x, 31.0);
        assert_eq!(layout.nodes[1].description_x, layout.nodes[2].description_x);
        assert!(layout.nodes[1].description_x.unwrap() > layout.nodes[2].label_x + layout.nodes[2].label_width);
        assert_eq!(layout.nodes[1].description_width, Some(54.4));
        assert_eq!(layout.width, layout_treeview(&diagram, 900.0).width);
        assert!(layout.width > layout.nodes[2].description_x.unwrap() + layout.nodes[2].description_width.unwrap());
        assert_eq!(layout.nodes[0].x + layout.nodes[0].width + 2.0, layout.width);
    }

    #[test]
    fn treeview_layout_resolves_registered_external_icon_glyphs() {
        use diagram_ir::{DiagramIconGlyph, TreeViewDiagram, TreeViewNode, TreeViewNodeKind};
        let config = diagram_ir::TreeViewConfig { show_icons: true, ..Default::default() };
        let diagram = TreeViewDiagram {
            title: None, accessibility_title: None, accessibility_description: None, config,
            nodes: vec![
                TreeViewNode { id: "known".into(), is_implicit_root: false, parent_id: None, depth: 1,
                    label: "app.ts".into(), kind: TreeViewNodeKind::File, class_selector: None,
                    icon: Some("logos:typescript".into()), description: None },
                TreeViewNode { id: "missing".into(), is_implicit_root: false, parent_id: None, depth: 1,
                    label: "app.rs".into(), kind: TreeViewNodeKind::File, class_selector: None,
                    icon: Some("logos:rust".into()), description: None },
            ],
        };
        let options = TreeViewLayoutOptions { icon_glyphs: BTreeMap::from([(
            "logos:typescript".into(), DiagramIconGlyph { text: "T".into(), font_family: "Icon Font".into() },
        )]) };
        let layout = layout_treeview_with_options(&diagram, 500.0, Some(&options));
        assert_eq!(layout.nodes[0].icon_glyph.as_ref().map(|glyph| glyph.text.as_str()), Some("T"));
        assert_eq!(layout.nodes[0].icon_glyph.as_ref().map(|glyph| glyph.font_family.as_str()), Some("Icon Font"));
        assert_eq!(layout.nodes[1].icon_glyph, None);
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
    fn swimlane_layout_expands_horizontal_nodes_for_long_labels() {
        use diagram_ir::{DiagramShape, SwimlaneDiagram, SwimlaneLane, SwimlaneNode};
        let diagram = SwimlaneDiagram {
            direction: DiagramDirection::Lr, title: None, accessibility_title: None,
            accessibility_description: None,
            lanes: vec![SwimlaneLane { id: "lane".into(), label: "Lane".into(), node_ids: vec!["long".into()] }],
            nodes: vec![SwimlaneNode { id: "long".into(),
                label: "A substantially longer process description".into(),
                lane_id: Some("lane".into()), shape: DiagramShape::Rect }],
            edges: vec![],
        };
        let layout = layout_swimlane(&diagram);
        assert!(layout.nodes[0].width > 132.0);
        assert!(layout.nodes[0].x + layout.nodes[0].width < layout.lanes[0].x + layout.lanes[0].width);
    }

    #[test]
    fn railroad_layout_preserves_choice_branches_and_repetition_loopbacks() {
        use diagram_ir::{RailroadDiagram, RailroadExpression, RailroadRule};
        let diagram = RailroadDiagram {
            title: Some("Number".into()), accessibility_title: None, accessibility_description: None,
            rules: vec![RailroadRule { name: "number".into(), definition: RailroadExpression::Sequence(vec![
                RailroadExpression::Choice(vec![RailroadExpression::Terminal("0".into()), RailroadExpression::Terminal("1".into())]),
                RailroadExpression::Repetition { element: Box::new(RailroadExpression::NonTerminal("digit".into())), min: 1, max: None },
            ]) }],
        };
        let layout = layout_railroad(&diagram);
        assert_eq!(layout.rules.len(), 1);
        assert_eq!(layout.rules[0].elements.len(), 3);
        assert!(layout.rules[0].paths.iter().any(|path| path.loopback));
        assert!(layout.rules[0].elements[1].y > layout.rules[0].elements[0].y);
        assert!(layout.width >= 420.0 && layout.height > 100.0);
    }

    #[test]
    fn railroad_layout_sizes_nested_repetition_without_clipping() {
        use diagram_ir::{RailroadDiagram, RailroadExpression, RailroadRule};
        let diagram = RailroadDiagram {
            title: None, accessibility_title: None, accessibility_description: None,
            rules: vec![RailroadRule { name: "value".into(), definition: RailroadExpression::Sequence(vec![
                RailroadExpression::NonTerminal("prefix".into()),
                RailroadExpression::Repetition {
                    element: Box::new(RailroadExpression::Choice(vec![
                        RailroadExpression::Terminal("a".into()),
                        RailroadExpression::Terminal("b".into()),
                        RailroadExpression::Terminal("c".into()),
                    ])),
                    min: 0,
                    max: None,
                },
            ]) }],
        };
        let layout = layout_railroad(&diagram);
        let rule = &layout.rules[0];
        let rule_bottom = rule.y + rule.height;
        assert!(rule.elements.iter().all(|element| element.y + element.height <= rule_bottom));
        assert!(rule.paths.iter().flat_map(|path| &path.points).all(|point| point.y <= rule_bottom));
        assert!(rule.height > 150.0);
    }
}
