//! Deterministic, backend-neutral hierarchy layout.

use std::collections::{BTreeMap, HashMap};

use diagram_ir::{
    DiagramDirection, DiagramIconGlyph, DiagramShape, LayoutedSwimlaneDiagram, LayoutedSwimlaneEdge, LayoutedSwimlaneGroup, LayoutedSwimlaneLane,
    LayoutedSwimlaneNode, LayoutedTreeViewConnector, LayoutedTreeViewDiagram, LayoutedTreeViewNode, LayoutedTreemapDiagram,
    LayoutedTreemapNode, Point, SwimlaneDiagram, TreeViewDiagram, TreemapDiagram,
    LayoutedRailroadDiagram, LayoutedRailroadElement, LayoutedRailroadPath, LayoutedRailroadRule,
    RailroadDiagram, RailroadElementKind, RailroadExpression, SwimlaneEdgeKind,
};

pub const VERSION: &str = "0.4.0";

const PARENT_HEADER: f64 = 24.0;

fn swimlane_constraint_path(
    start: &str,
    target: &str,
    constraints: &[(String, String, usize)],
) -> bool {
    let mut pending = vec![start];
    let mut visited = Vec::<&str>::new();
    while let Some(node) = pending.pop() {
        if node == target {
            return true;
        }
        if visited.contains(&node) {
            continue;
        }
        visited.push(node);
        pending.extend(constraints.iter().filter_map(|(from, to, _)|
            (from == node).then_some(to.as_str())
        ));
    }
    false
}

fn swimlane_rank_segments_cross(
    first_start: (f64, f64),
    first_end: (f64, f64),
    second_start: (f64, f64),
    second_end: (f64, f64),
) -> bool {
    let first_dx = first_end.0 - first_start.0;
    let first_dy = first_end.1 - first_start.1;
    let second_dx = second_end.0 - second_start.0;
    let second_dy = second_end.1 - second_start.1;
    let denominator = first_dx * second_dy - first_dy * second_dx;
    if denominator.abs() < 1e-9 {
        return false;
    }
    let offset_x = second_start.0 - first_start.0;
    let offset_y = second_start.1 - first_start.1;
    let first_t = (offset_x * second_dy - offset_y * second_dx) / denominator;
    let second_t = (offset_x * first_dy - offset_y * first_dx) / denominator;
    first_t > 1e-9 && first_t < 1.0 - 1e-9
        && second_t > 1e-9 && second_t < 1.0 - 1e-9
}

fn swimlane_rank_crossing_count(
    diagram: &SwimlaneDiagram,
    ranks: &HashMap<String, usize>,
) -> usize {
    let lane_by_node = diagram.lanes.iter().enumerate().flat_map(|(lane_index, lane)|
        lane.node_ids.iter().map(move |node_id| (node_id.as_str(), lane_index as f64))
    ).collect::<HashMap<_, _>>();
    diagram.edges.iter().enumerate().map(|(index, first)| {
        diagram.edges[index + 1..].iter().filter(|second| {
            if first.from == second.from || first.from == second.to
                || first.to == second.from || first.to == second.to
            {
                return false;
            }
            let Some((&first_source_lane, &first_target_lane, &second_source_lane, &second_target_lane)) =
                lane_by_node.get(first.from.as_str()).zip(lane_by_node.get(first.to.as_str()))
                    .zip(lane_by_node.get(second.from.as_str()).zip(lane_by_node.get(second.to.as_str())))
                    .map(|((a, b), (c, d))| (a, b, c, d))
            else { return false };
            swimlane_rank_segments_cross(
                (ranks.get(&first.from).copied().unwrap_or(0) as f64, first_source_lane),
                (ranks.get(&first.to).copied().unwrap_or(0) as f64, first_target_lane),
                (ranks.get(&second.from).copied().unwrap_or(0) as f64, second_source_lane),
                (ranks.get(&second.to).copied().unwrap_or(0) as f64, second_target_lane),
            )
        }).count()
    }).sum()
}

fn optimize_swimlane_ranks_by_crossings(
    diagram: &SwimlaneDiagram,
    constraints: &[(String, String, usize)],
    ranks: &mut HashMap<String, usize>,
) {
    let mut best = swimlane_rank_crossing_count(diagram, ranks);
    for _ in 0..4 {
        let mut changed = false;
        for node in &diagram.nodes {
            let mut candidate = ranks.clone();
            *candidate.entry(node.id.clone()).or_default() += 1;
            for _ in 0..diagram.nodes.len() {
                let mut propagated = false;
                for (from, to, distance) in constraints {
                    let next = candidate.get(from).copied().unwrap_or(0) + distance;
                    if candidate.get(to).copied().unwrap_or(0) < next {
                        candidate.insert(to.clone(), next);
                        propagated = true;
                    }
                }
                if !propagated {
                    break;
                }
            }
            let score = swimlane_rank_crossing_count(diagram, &candidate);
            if score < best {
                *ranks = candidate;
                best = score;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn swimlane_cross_lane_ranks(diagram: &SwimlaneDiagram) -> Option<HashMap<String, usize>> {
    if diagram.ignore_cross_lane_edges {
        return None;
    }
    let mut constraints = Vec::<(String, String, usize)>::new();
    for lane in &diagram.lanes {
        for pair in lane.node_ids.windows(2) {
            constraints.push((
                pair[0].clone(), pair[1].clone(),
                swimlane_link_gap_units(diagram, &pair[0], &pair[1]),
            ));
        }
    }
    for edge in &diagram.edges {
        if edge.from == edge.to
            || swimlane_constraint_path(&edge.to, &edge.from, &constraints)
        {
            continue;
        }
        constraints.push((edge.from.clone(), edge.to.clone(), edge.min_length.max(1)));
    }
    let mut ranks = diagram.nodes.iter().map(|node| (node.id.clone(), 0usize))
        .collect::<HashMap<_, _>>();
    for _ in 0..diagram.nodes.len() {
        let mut changed = false;
        for (from, to, distance) in &constraints {
            let next = ranks.get(from).copied().unwrap_or(0) + distance;
            if ranks.get(to).copied().unwrap_or(0) < next {
                ranks.insert(to.clone(), next);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if diagram.optimize_ranks_by_crossings {
        optimize_swimlane_ranks_by_crossings(diagram, &constraints, &mut ranks);
    }
    Some(ranks)
}

/// Lay out explicit Railroad constructors into rule rows and branch paths.
pub fn layout_railroad(diagram: &RailroadDiagram) -> LayoutedRailroadDiagram {
    let padding = diagram.config.padding;
    let title_height = if diagram.title.is_some() { 36.0 + padding } else { 8.0 + padding };
    let mut rules = Vec::new(); let mut y = title_height; let mut width: f64 = 400.0 + padding * 2.0;
    for rule in &diagram.rules {
        let mut elements = Vec::new(); let mut paths = Vec::new();
        let content_y = y + padding + 8.0;
        let rule_label = format!("{} =", rule.name);
        let rule_label_width = rule_label.chars().count() as f64 * diagram.config.font_size * 0.61;
        let name_width = rule_label_width + 20.0;
        let start_marker_x = padding + name_width;
        let expression_x = start_marker_x + 20.0;
        let (expr_width, expr_height) = layout_railroad_expression(
            &rule.definition, expression_x, content_y, &diagram.config, &mut elements, &mut paths,
        );
        let center = content_y + expr_height / 2.0;
        let start_marker = Point { x: start_marker_x, y: center };
        let end_marker = Point { x: expression_x + expr_width + 10.0, y: center };
        paths.push(LayoutedRailroadPath { points: vec![Point { x: start_marker.x + diagram.config.marker_radius, y: center }, Point { x: expression_x, y: center }], loopback: false });
        paths.push(LayoutedRailroadPath { points: vec![Point { x: expression_x + expr_width, y: center }, Point { x: end_marker.x - diagram.config.marker_radius, y: center }], loopback: false });
        let height = expr_height + padding * 2.0 + 24.0;
        width = width.max(end_marker.x + diagram.config.marker_radius + padding);
        rules.push(LayoutedRailroadRule { name: rule.name.clone(), y, height, start_marker, end_marker, elements, paths });
        y += height + diagram.config.vertical_separation;
    }
    LayoutedRailroadDiagram { width, height: y + padding, title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(), accessibility_description: diagram.accessibility_description.clone(),
        config: diagram.config.clone(), rules }
}

fn layout_railroad_expression(expr: &RailroadExpression, x: f64, y: f64,
    config: &diagram_ir::RailroadConfig, elements: &mut Vec<LayoutedRailroadElement>,
    paths: &mut Vec<LayoutedRailroadPath>) -> (f64, f64) {
    match expr {
        RailroadExpression::Terminal(label) | RailroadExpression::NonTerminal(label) | RailroadExpression::Special(label) => {
            let kind = match expr { RailroadExpression::Terminal(_) => RailroadElementKind::Terminal,
                RailroadExpression::NonTerminal(_) => RailroadElementKind::NonTerminal, _ => RailroadElementKind::Special };
            let display_label = if kind == RailroadElementKind::Special { format!("? {label} ?") } else { label.clone() };
            let text_width = display_label.chars().count() as f64 * config.font_size * 0.61;
            let width = text_width.clamp(config.font_size * 3.15, config.font_size * 13.72) + config.padding * 2.0;
            let height = config.font_size * 1.15 + config.padding * 2.0;
            elements.push(LayoutedRailroadElement { kind, label: display_label, x, y, width, height }); (width, height)
        }
        RailroadExpression::Sequence(items) => {
            let sizes: Vec<_> = items.iter().map(|item| measure_railroad_expression(item, config)).collect();
            let max_height = sizes.iter().map(|size| size.1).fold(36.0, f64::max);
            let center = y + max_height / 2.0;
            let mut cursor = x; let mut previous_end = None;
            for (item, (_, item_height)) in items.iter().zip(sizes) {
                let (item_width, _) = layout_railroad_expression(item, cursor, y + (max_height - item_height) / 2.0, config, elements, paths);
                if let Some(end) = previous_end { paths.push(LayoutedRailroadPath { points: vec![Point { x: end, y: center }, Point { x: cursor, y: center }], loopback: false }); }
                previous_end = Some(cursor + item_width); cursor += item_width + config.horizontal_separation;
            }
            ((cursor - x - config.horizontal_separation).max(0.0), max_height)
        }
        RailroadExpression::Choice(alternatives) => {
            let radius = config.arc_radius;
            let gap = config.vertical_separation; let mut cursor_y = y; let mut max_width: f64 = 0.0; let mut sizes = Vec::new();
            for alternative in alternatives { let size = layout_railroad_expression(alternative, x + radius * 2.0, cursor_y, config, elements, paths); sizes.push((cursor_y, size)); cursor_y += size.1 + gap; max_width = max_width.max(size.0); }
            let height = (cursor_y - y - gap).max(36.0); let middle = y + height / 2.0;
            for (alt_y, (alt_width, alt_height)) in sizes { let alt_center = alt_y + alt_height / 2.0;
                let left = if (alt_center - middle).abs() <= f64::EPSILON {
                    vec![Point { x, y: middle }, Point { x: x + radius * 2.0, y: middle }]
                } else {
                    vec![Point { x, y: middle }, Point { x: x + radius, y: middle },
                        Point { x: x + radius, y: alt_center }, Point { x: x + radius * 2.0, y: alt_center }]
                };
                paths.push(LayoutedRailroadPath { points: left, loopback: false });
                let right_start = x + radius * 2.0 + alt_width;
                let right_lane = x + radius * 3.0 + max_width;
                let exit = x + radius * 4.0 + max_width;
                let right = if (alt_center - middle).abs() <= f64::EPSILON {
                    vec![Point { x: right_start, y: middle }, Point { x: exit, y: middle }]
                } else {
                    vec![Point { x: right_start, y: alt_center }, Point { x: right_lane, y: alt_center },
                        Point { x: right_lane, y: middle }, Point { x: exit, y: middle }]
                };
                paths.push(LayoutedRailroadPath { points: right, loopback: false }); }
            (max_width + radius * 4.0, height)
        }
        RailroadExpression::Optional(element) => {
            let radius = config.arc_radius;
            let inner_y = y + radius * 2.0;
            let (inner_width, inner_height) = layout_railroad_expression(element, x + radius * 2.0, inner_y, config, elements, paths);
            let height = inner_height + radius * 2.0; let middle = inner_y + inner_height / 2.0;
            let exit_x = x + inner_width + radius * 4.0;
            paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + radius * 2.0, y: middle }], loopback: false });
            paths.push(LayoutedRailroadPath { points: vec![Point { x: x + radius * 2.0 + inner_width, y: middle }, Point { x: exit_x, y: middle }], loopback: false });
            paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + radius, y: middle },
                Point { x: x + radius, y }, Point { x: exit_x - radius, y },
                Point { x: exit_x - radius, y: middle }, Point { x: exit_x, y: middle }], loopback: false });
            (inner_width + radius * 4.0, height)
        }
        RailroadExpression::Repetition { element, min, .. } => {
            let radius = config.arc_radius;
            let has_bypass = *min == 0;
            let inner_y = y + if has_bypass { radius * 2.0 } else { 0.0 };
            let inner_x = x + radius * 2.0;
            let (inner_width, inner_height) = layout_railroad_expression(element, inner_x, inner_y, config, elements, paths);
            let total_width = inner_width + radius * 4.0;
            let height = inner_height + radius * 2.0 + if has_bypass { radius * 2.0 } else { 0.0 };
            let middle = inner_y + inner_height / 2.0; let loop_y = inner_y + inner_height + radius * 2.0;
            paths.push(LayoutedRailroadPath { points: vec![Point { x: inner_x + inner_width, y: middle },
                Point { x: x + total_width - radius, y: middle }, Point { x: x + total_width - radius, y: loop_y },
                Point { x: x + radius, y: loop_y }, Point { x: x + radius, y: middle }, Point { x: inner_x, y: middle }], loopback: true });
            paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: inner_x, y: middle }], loopback: false });
            paths.push(LayoutedRailroadPath { points: vec![Point { x: inner_x + inner_width, y: middle }, Point { x: x + total_width, y: middle }], loopback: false });
            if has_bypass { paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + radius, y: middle },
                Point { x: x + radius, y }, Point { x: x + total_width - radius, y },
                Point { x: x + total_width - radius, y: middle }, Point { x: x + total_width, y: middle }], loopback: false }); }
            (total_width, height)
        }
    }
}

fn measure_railroad_expression(expr: &RailroadExpression, config: &diagram_ir::RailroadConfig) -> (f64, f64) {
    match expr {
        RailroadExpression::Terminal(label) | RailroadExpression::NonTerminal(label) | RailroadExpression::Special(label) => {
            let display_length = label.chars().count() + if matches!(expr, RailroadExpression::Special(_)) { 4 } else { 0 };
            let text_width = display_length as f64 * config.font_size * 0.61;
            (text_width.clamp(config.font_size * 3.15, config.font_size * 13.72) + config.padding * 2.0,
                config.font_size * 1.15 + config.padding * 2.0)
        }
        RailroadExpression::Sequence(items) => {
            let sizes: Vec<_> = items.iter().map(|item| measure_railroad_expression(item, config)).collect();
            let width = sizes.iter().map(|size| size.0).sum::<f64>()
                + config.horizontal_separation * items.len().saturating_sub(1) as f64;
            let height = sizes.iter().map(|size| size.1).fold(36.0, f64::max);
            (width, height)
        }
        RailroadExpression::Choice(alternatives) => {
            let sizes: Vec<_> = alternatives.iter().map(|item| measure_railroad_expression(item, config)).collect();
            let width = sizes.iter().map(|size| size.0).fold(0.0, f64::max) + config.arc_radius * 4.0;
            let height = sizes.iter().map(|size| size.1).sum::<f64>()
                + config.vertical_separation * alternatives.len().saturating_sub(1) as f64;
            (width, height.max(36.0))
        }
        RailroadExpression::Optional(element) => {
            let (width, height) = measure_railroad_expression(element, config);
            (width + config.arc_radius * 4.0, height + config.arc_radius * 2.0)
        }
        RailroadExpression::Repetition { element, min, .. } => {
            let (width, height) = measure_railroad_expression(element, config);
            (width + config.arc_radius * 4.0,
                height + config.arc_radius * 2.0 + if *min == 0 { config.arc_radius * 2.0 } else { 0.0 })
        }
    }
}

/// Lay out top-level Mermaid subgraphs as stable ownership lanes.
pub fn layout_swimlane(diagram: &SwimlaneDiagram) -> LayoutedSwimlaneDiagram {
    let title_height = if diagram.title.is_some() {
        44.0 + diagram.flowchart_config.title_top_margin.unwrap_or(0.0)
    } else {
        16.0
    };
    let horizontal_flow = matches!(
        diagram.direction,
        DiagramDirection::Lr | DiagramDirection::Rl
    );
    let subgraph_title_margin_top = diagram.flowchart_config.subgraph_title_margin_top.unwrap_or(0.0);
    let subgraph_title_margin_bottom = diagram.flowchart_config.subgraph_title_margin_bottom.unwrap_or(0.0);
    let lane_header = 38.0 + subgraph_title_margin_top + subgraph_title_margin_bottom;
    let outer_padding = diagram.flowchart_config.diagram_padding.unwrap_or(12.0);
    let lane_gap = diagram.flowchart_config.node_spacing.unwrap_or(12.0);
    let node_width = 132.0;
    let node_height = diagram.flowchart_config.padding
        .map(|padding| 26.0 + padding * 2.0).unwrap_or(54.0).max(54.0);
    let node_gap = diagram.flowchart_config.rank_spacing.unwrap_or(42.0);
    let cross_lane_ranks = swimlane_cross_lane_ranks(diagram);
    let max_rank = cross_lane_ranks.as_ref().and_then(|ranks| ranks.values().max().copied()).unwrap_or(0);
    let rank_slot_width = diagram.nodes.iter().map(|node|
        swimlane_node_width(&node.shape, &node.label, &node.style,
            diagram.theme.font_size.unwrap_or(13.0), diagram.flowchart_config.wrapping_width,
            diagram.flowchart_config.padding)
    ).fold(node_width, f64::max);
    let max_group_depth = diagram.groups.iter().map(|group| {
        let mut depth = 1usize;
        let mut parent = group.parent_id.as_deref();
        while let Some(parent_id) = parent {
            depth += 1;
            parent = diagram.groups.iter().find(|candidate| candidate.id == parent_id)
                .and_then(|candidate| candidate.parent_id.as_deref());
        }
        depth
    }).max().unwrap_or(0);
    let lane_cross = if horizontal_flow { 150.0 } else { (rank_slot_width + 18.0).max(150.0) }
        + 60.0 * max_group_depth as f64;
    let lane_along = if cross_lane_ranks.is_some() && horizontal_flow {
        lane_header + (max_rank + 1) as f64 * rank_slot_width + max_rank as f64 * node_gap + 40.0
    } else if cross_lane_ranks.is_some() {
        lane_header + (max_rank + 1) as f64 * node_height + max_rank as f64 * node_gap + 28.0
    } else if horizontal_flow {
        diagram.lanes.iter().map(|lane| {
            let widths = lane.node_ids.iter().filter_map(|node_id| {
                diagram.nodes.iter().find(|node| &node.id == node_id)
                    .map(|node| swimlane_node_width(
                        &node.shape, &node.label, &node.style, diagram.theme.font_size.unwrap_or(13.0),
                        diagram.flowchart_config.wrapping_width, diagram.flowchart_config.padding,
                    ))
            }).collect::<Vec<_>>();
            let gap_units = lane.node_ids.windows(2).map(|pair|
                swimlane_link_gap_units(diagram, &pair[0], &pair[1])
            ).sum::<usize>();
            lane_header + widths.iter().sum::<f64>() + node_gap * gap_units as f64 + 48.0
        }).fold(0.0, f64::max)
    } else {
        diagram.lanes.iter().map(|lane| {
            let gap_units = lane.node_ids.windows(2).map(|pair|
                swimlane_link_gap_units(diagram, &pair[0], &pair[1])
            ).sum::<usize>();
            lane_header + lane.node_ids.len().max(1) as f64 * node_height
                + (gap_units + 1) as f64 * node_gap + 28.0
        }).fold(0.0, f64::max)
    };
    let (width, height) = if horizontal_flow {
        (
            lane_along.max(420.0),
            title_height + diagram.lanes.len() as f64 * (lane_cross + lane_gap) + outer_padding,
        )
    } else {
        (
            diagram.lanes.len() as f64 * (lane_cross + lane_gap) + outer_padding,
            title_height + lane_along.max(420.0),
        )
    };
    let ordered_lanes = if diagram.automatic_lane_ordering {
        optimize_swimlane_lane_order(diagram)
    } else {
        diagram.lanes.iter().collect()
    };
    let mut lanes = Vec::new();
    let mut nodes = Vec::new();
    for (lane_index, lane) in ordered_lanes.into_iter().enumerate() {
        let (x, y, lane_width, lane_height) = if horizontal_flow {
            (
                outer_padding,
                title_height + lane_index as f64 * (lane_cross + lane_gap),
                width - outer_padding * 2.0,
                lane_cross,
            )
        } else {
            (
                outer_padding + lane_index as f64 * (lane_cross + lane_gap),
                title_height,
                lane_cross,
                height - title_height - outer_padding,
            )
        };
        lanes.push(LayoutedSwimlaneLane {
            id: lane.id.clone(),
            label: lane.label.clone(),
            classes: lane.classes.clone(),
            style: lane.style.clone(),
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
        let mut vertical_cursor = y + lane_header + 20.0;
        for (node_index, node_id) in member_ids.iter().enumerate() {
            let Some(node) = diagram.nodes.iter().find(|node| &node.id == node_id) else {
                continue;
            };
            let (resolved_node_width, resolved_node_height) = match node.shape {
                DiagramShape::Anchor => (2.0, 2.0),
                DiagramShape::SmallCircle | DiagramShape::FramedCircle | DiagramShape::FilledCircle => (14.0, 14.0),
                DiagramShape::Summary => (60.0, 60.0),
                DiagramShape::ForkJoin if horizontal_flow => (10.0, 70.0),
                DiagramShape::ForkJoin => (70.0, 10.0),
                DiagramShape::LightningBolt => (35.0, 70.0),
                _ => (swimlane_node_width(
                    &node.shape, &node.label, &node.style, diagram.theme.font_size.unwrap_or(13.0),
                    diagram.flowchart_config.wrapping_width, diagram.flowchart_config.padding,
                ), node_height),
            };
            let rank = cross_lane_ranks.as_ref().and_then(|ranks| ranks.get(node_id)).copied();
            let (node_x, node_y) = if let (true, Some(rank)) = (horizontal_flow, rank) {
                (
                    x + lane_header + 20.0 + rank as f64 * (rank_slot_width + node_gap),
                    y + (lane_cross - resolved_node_height) / 2.0,
                )
            } else if let Some(rank) = rank {
                (
                    x + (lane_cross - resolved_node_width) / 2.0,
                    y + lane_header + 20.0 + rank as f64 * (node_height + node_gap)
                        + (node_height - resolved_node_height) / 2.0,
                )
            } else if horizontal_flow {
                (
                    horizontal_cursor,
                    y + (lane_cross - resolved_node_height) / 2.0,
                )
            } else {
                (
                    x + (lane_cross - resolved_node_width) / 2.0,
                    vertical_cursor + (node_height - resolved_node_height) / 2.0,
                )
            };
            nodes.push(LayoutedSwimlaneNode {
                id: node.id.clone(),
                label: node.label.clone(),
                shape: node.shape.clone(),
                classes: node.classes.clone(),
                style: node.style.clone(),
                x: node_x,
                y: node_y,
                width: resolved_node_width,
                height: resolved_node_height,
            });
            let gap_units = member_ids.get(node_index + 1).map(|next_id|
                swimlane_link_gap_units(diagram, node_id, next_id)
            ).unwrap_or(1);
            horizontal_cursor += resolved_node_width + node_gap * gap_units as f64;
            vertical_cursor += node_height + node_gap * gap_units as f64;
        }
    }
    let mut groups = Vec::<LayoutedSwimlaneGroup>::new();
    for group in diagram.groups.iter().rev() {
        let lane = lanes.iter().find(|lane| lane.id == group.lane_id)
            .expect("nested Swimlane group must belong to a lane");
        let mut content = group.node_ids.iter().filter_map(|node_id|
            nodes.iter().find(|node| &node.id == node_id)
                .map(|node| (node.x, node.y, node.x + node.width, node.y + node.height))
        ).collect::<Vec<_>>();
        content.extend(groups.iter().filter(|child| child.parent_id.as_deref() == Some(group.id.as_str()))
            .map(|child| (child.x, child.y, child.x + child.width, child.y + child.height)));
        let (mut left, mut top, mut right, mut bottom) = content.first().copied().unwrap_or((
            lane.x + 20.0, lane.y + 42.0, lane.x + 120.0, lane.y + 92.0,
        ));
        for (x1, y1, x2, y2) in content.into_iter().skip(1) {
            left = left.min(x1); top = top.min(y1); right = right.max(x2); bottom = bottom.max(y2);
        }
        left = (left - 12.0).max(lane.x + 6.0);
        top = (top - 30.0 - subgraph_title_margin_top - subgraph_title_margin_bottom)
            .max(lane.y + lane_header - 6.0);
        right = (right + 12.0).min(lane.x + lane.width - 6.0);
        bottom = (bottom + 12.0).min(lane.y + lane.height - 6.0);
        groups.push(LayoutedSwimlaneGroup {
            id: group.id.clone(), label: group.label.clone(), lane_id: group.lane_id.clone(),
            parent_id: group.parent_id.clone(), classes: group.classes.clone(), style: group.style.clone(), x: left, y: top,
            width: (right - left).max(40.0), height: (bottom - top).max(40.0),
        });
    }
    groups.reverse();
    let bounds = nodes
        .iter()
        .map(|node| {
            (
                node.id.as_str(),
                (Point { x: node.x + node.width / 2.0, y: node.y + node.height / 2.0 }, node.width, node.height),
            )
        })
        .collect::<HashMap<_, _>>();
    let edges = diagram
        .edges
        .iter()
        .filter_map(|edge| {
            let (from_center, from_width, from_height) = bounds.get(edge.from.as_str())?;
            let (to_center, to_width, to_height) = bounds.get(edge.to.as_str())?;
            let (from, to) = swimlane_edge_endpoints(
                from_center, *from_width, *from_height, to_center, *to_width, *to_height,
            );
            Some(LayoutedSwimlaneEdge {
                id: edge.id.clone(),
                from,
                to,
                label: edge.label.clone(),
                kind: edge.kind,
                start_marker: edge.start_marker,
                end_marker: edge.end_marker,
                min_length: edge.min_length,
                classes: edge.classes.clone(),
                style: edge.style.clone(),
                animate: edge.animate,
                animation: edge.animation,
                curve: edge.curve.or(diagram.flowchart_config.curve),
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
        groups,
        nodes,
        edges,
        links: diagram.links.clone(),
        callbacks: diagram.callbacks.clone(),
        line_hops: diagram.line_hops,
        automatic_lane_ordering: diagram.automatic_lane_ordering,
        ignore_cross_lane_edges: diagram.ignore_cross_lane_edges,
        optimize_ranks_by_crossings: diagram.optimize_ranks_by_crossings,
        flowchart_config: diagram.flowchart_config.clone(),
        theme: diagram.theme.clone(),
    }
}

#[derive(Clone)]
struct WeightedSwimlaneEdge { a: String, b: String, weight: usize }

fn optimize_swimlane_lane_order(diagram: &SwimlaneDiagram) -> Vec<&diagram_ir::SwimlaneLane> {
    let source_order = diagram.lanes.iter().map(|lane| lane.id.as_str()).collect::<Vec<_>>();
    if source_order.len() < 2 {
        return diagram.lanes.iter().collect();
    }
    let source_index = source_order.iter().enumerate()
        .map(|(index, lane)| (*lane, index)).collect::<HashMap<_, _>>();
    let node_lanes = diagram.lanes.iter().flat_map(|lane|
        lane.node_ids.iter().map(move |node| (node.as_str(), lane.id.as_str()))
    ).collect::<HashMap<_, _>>();
    let mut counts = BTreeMap::<(usize, usize), usize>::new();
    for edge in &diagram.edges {
        if edge.kind == SwimlaneEdgeKind::Invisible {
            continue;
        }
        let (Some(from_lane), Some(to_lane)) = (
            node_lanes.get(edge.from.as_str()), node_lanes.get(edge.to.as_str()),
        ) else { continue };
        if from_lane == to_lane {
            continue;
        }
        let (Some(&from_index), Some(&to_index)) = (
            source_index.get(from_lane), source_index.get(to_lane),
        ) else { continue };
        let pair = if from_index <= to_index {
            (from_index, to_index)
        } else {
            (to_index, from_index)
        };
        *counts.entry(pair).or_default() += 1;
    }
    let weights = counts.into_iter().map(|((a, b), weight)| WeightedSwimlaneEdge {
        a: source_order[a].to_string(), b: source_order[b].to_string(), weight,
    }).collect::<Vec<_>>();
    if weights.is_empty() {
        return diagram.lanes.iter().collect();
    }
    let source = source_order.iter().map(|lane| (*lane).to_string()).collect::<Vec<_>>();
    let source_positions = source.iter().enumerate()
        .map(|(index, lane)| (lane.as_str(), index)).collect::<HashMap<_, _>>();
    let mut best = greedy_swimlane_order(source.clone(), &weights, &source_positions);
    for restart in 0..8 {
        let mut candidate = source.clone();
        deterministic_swimlane_shuffle(&mut candidate, swimlane_restart_seed(&source, &weights, restart));
        let candidate = greedy_swimlane_order(candidate, &weights, &source_positions);
        if candidate.1 < best.1 || (candidate.1 == best.1 && candidate.2 < best.2) {
            best = candidate;
        }
    }
    best.0.iter().filter_map(|lane_id|
        diagram.lanes.iter().find(|lane| lane.id == *lane_id)
    ).collect()
}

fn swimlane_arrangement_cost(order: &[String], weights: &[WeightedSwimlaneEdge]) -> usize {
    let positions = order.iter().enumerate().map(|(index, lane)| (lane.as_str(), index))
        .collect::<HashMap<_, _>>();
    weights.iter().map(|edge| edge.weight * positions[edge.a.as_str()].abs_diff(positions[edge.b.as_str()])).sum()
}

fn greedy_swimlane_order(
    mut order: Vec<String>, weights: &[WeightedSwimlaneEdge], source: &HashMap<&str, usize>,
) -> (Vec<String>, usize, usize) {
    let mut cost = swimlane_arrangement_cost(&order, weights);
    for _ in 0..order.len().max(1) {
        let mut changed = false;
        for index in 0..order.len().saturating_sub(1) {
            order.swap(index, index + 1);
            let next_cost = swimlane_arrangement_cost(&order, weights);
            if next_cost < cost {
                cost = next_cost;
                changed = true;
            } else {
                order.swap(index, index + 1);
            }
        }
        if !changed { break; }
    }
    let distance = order.iter().enumerate().map(|(index, lane)| index.abs_diff(source[lane.as_str()])).sum();
    (order, cost, distance)
}

fn swimlane_restart_seed(source: &[String], weights: &[WeightedSwimlaneEdge], restart: usize) -> u32 {
    let mut sorted_weights = weights.to_vec();
    sorted_weights.sort_by(|left, right| left.a.cmp(&right.a).then_with(|| left.b.cmp(&right.b)));
    let signature = sorted_weights.iter().map(|edge| format!("{}:{}:{}", edge.a, edge.b, edge.weight))
        .collect::<Vec<_>>().join("|");
    let input = format!("{}#{}#{restart}", source.join("|"), signature);
    input.encode_utf16().fold(2_166_136_261_u32, |hash, unit|
        (hash ^ u32::from(unit)).wrapping_mul(16_777_619)
    )
}

fn deterministic_swimlane_shuffle(order: &mut [String], seed: u32) {
    let mut state = seed;
    for index in (1..order.len()).rev() {
        state = state.wrapping_add(0x6d2b79f5);
        let mut value = state;
        value = (value ^ (value >> 15)).wrapping_mul(value | 1);
        value ^= value.wrapping_add((value ^ (value >> 7)).wrapping_mul(value | 61));
        let random = (value ^ (value >> 14)) as f64 / 4_294_967_296.0;
        order.swap(index, (random * (index + 1) as f64).floor() as usize);
    }
}

fn swimlane_link_gap_units(diagram: &SwimlaneDiagram, first: &str, second: &str) -> usize {
    diagram.edges.iter().filter(|edge|
        (edge.from == first && edge.to == second) || (edge.from == second && edge.to == first)
    ).map(|edge| edge.min_length).max().unwrap_or(1).max(1)
}

fn swimlane_edge_endpoints(
    from: &Point, from_width: f64, from_height: f64,
    to: &Point, to_width: f64, to_height: f64,
) -> (Point, Point) {
    fn boundary(center: &Point, toward: &Point, width: f64, height: f64) -> Point {
        let dx = toward.x - center.x;
        let dy = toward.y - center.y;
        let scale_x = if dx.abs() < 1e-9 { f64::INFINITY } else { width / 2.0 / dx.abs() };
        let scale_y = if dy.abs() < 1e-9 { f64::INFINITY } else { height / 2.0 / dy.abs() };
        let scale = scale_x.min(scale_y);
        Point { x: center.x + dx * scale, y: center.y + dy * scale }
    }
    (
        boundary(from, to, from_width, from_height),
        boundary(to, from, to_width, to_height),
    )
}

fn swimlane_node_width(
    shape: &DiagramShape,
    label: &str,
    style: &diagram_ir::DiagramStyle,
    theme_font_size: f64,
    wrapping_width: Option<f64>,
    padding: Option<f64>,
) -> f64 {
    match shape {
        DiagramShape::Anchor => 2.0,
        DiagramShape::SmallCircle | DiagramShape::FramedCircle | DiagramShape::FilledCircle => 14.0,
        DiagramShape::Summary => 60.0,
        DiagramShape::ForkJoin => 10.0,
        DiagramShape::LightningBolt => 35.0,
        _ => {
            let font_scale = style.font_size.unwrap_or(theme_font_size) / 13.0;
            let weight_scale = if style.font_weight.unwrap_or(400) >= 600 { 1.06 } else { 1.0 };
            let horizontal_padding = padding.map(|value| value * 2.0).unwrap_or(28.0);
            let minimum_width = 104.0 + horizontal_padding;
            (label.chars().count() as f64 * 7.2 * font_scale * weight_scale + horizontal_padding)
                .clamp(minimum_width, wrapping_width.unwrap_or(360.0).max(minimum_width))
        }
    }
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
        use diagram_ir::{DiagramShape, EdgeMarker, SwimlaneDiagram, SwimlaneEdge, SwimlaneEdgeKind, SwimlaneLane, SwimlaneNode};
        let diagram = SwimlaneDiagram {
            direction: DiagramDirection::Lr, title: Some("Handoff".into()), accessibility_title: None,
            accessibility_description: None,
            lanes: vec![
                SwimlaneLane { id: "buyer".into(), label: "Buyer".into(), node_ids: vec!["choose".into()], classes: Vec::new(), style: Default::default() },
                SwimlaneLane { id: "store".into(), label: "Store".into(), node_ids: vec!["ship".into()], classes: Vec::new(), style: Default::default() },
            ],
            groups: vec![], nodes: vec![
                SwimlaneNode { id: "choose".into(), label: "Choose".into(), lane_id: Some("buyer".into()), shape: DiagramShape::Rect, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "ship".into(), label: "Ship".into(), lane_id: Some("store".into()), shape: DiagramShape::RoundedRect, classes: Vec::new(), style: Default::default() },
            ],
            edges: vec![SwimlaneEdge {
                id: None, from: "choose".into(), to: "ship".into(), label: None,
                kind: SwimlaneEdgeKind::Directed,
                start_marker: EdgeMarker::Circle, end_marker: EdgeMarker::Point,
                min_length: 1,
                classes: Vec::new(), style: Default::default(),
                animate: false, animation: None,
                curve: None,
            }],
            links: Vec::new(), callbacks: Vec::new(), line_hops: None, automatic_lane_ordering: false,
            ignore_cross_lane_edges: true,
            optimize_ranks_by_crossings: true,
            flowchart_config: Default::default(),
            theme: Default::default(),
        };
        let layout = layout_swimlane(&diagram);
        assert!(layout.lanes[1].y > layout.lanes[0].y);
        for (node, lane) in layout.nodes.iter().zip(layout.lanes.iter()) {
            assert!(node.x >= lane.x && node.x + node.width <= lane.x + lane.width);
            assert!(node.y >= lane.y && node.y + node.height <= lane.y + lane.height);
        }
        assert_eq!(layout.edges[0].start_marker, EdgeMarker::Circle);
        assert_eq!(layout.edges[0].end_marker, EdgeMarker::Point);
        assert_ne!(layout.edges[0].from.y, layout.nodes[0].y + layout.nodes[0].height / 2.0);
    }

    #[test]
    fn swimlane_layout_expands_horizontal_nodes_for_long_labels() {
        use diagram_ir::{DiagramShape, SwimlaneDiagram, SwimlaneLane, SwimlaneNode};
        let diagram = SwimlaneDiagram {
            direction: DiagramDirection::Lr, title: None, accessibility_title: None,
            accessibility_description: None,
            lanes: vec![SwimlaneLane { id: "lane".into(), label: "Lane".into(), node_ids: vec!["long".into()], classes: Vec::new(), style: Default::default() }],
            groups: vec![], nodes: vec![SwimlaneNode { id: "long".into(),
                label: "A substantially longer process description".into(),
                lane_id: Some("lane".into()), shape: DiagramShape::Rect, classes: Vec::new(), style: Default::default() }],
            edges: vec![], links: Vec::new(), callbacks: Vec::new(), line_hops: None, automatic_lane_ordering: false,
            ignore_cross_lane_edges: true,
            optimize_ranks_by_crossings: true,
            flowchart_config: Default::default(),
            theme: Default::default(),
        };
        let layout = layout_swimlane(&diagram);
        assert!(layout.nodes[0].width > 132.0);
        assert!(layout.nodes[0].x + layout.nodes[0].width < layout.lanes[0].x + layout.lanes[0].width);
    }

    #[test]
    fn swimlane_layout_preserves_expanded_node_shapes() {
        use diagram_ir::{DiagramShape, SwimlaneDiagram, SwimlaneLane, SwimlaneNode};
        let diagram = SwimlaneDiagram {
            direction: DiagramDirection::Lr, title: None, accessibility_title: None,
            accessibility_description: None,
            lanes: vec![SwimlaneLane { id: "runtime".into(), label: "Runtime".into(),
                node_ids: vec!["hosted".into(), "alert".into(), "collect".into(), "extract".into(), "manual".into(), "card".into(), "lined".into(), "text".into(), "start".into(), "stop".into(), "fork".into(), "bolt".into(), "brace-left".into(), "brace-right".into(), "braces".into(), "document".into(), "delay".into(), "horizontal-cylinder".into(), "lined-cylinder".into(), "display".into(), "divided".into(), "window-pane".into(), "junction".into(), "loop-limit".into(), "manual-input".into(), "multi-document".into(), "multi-process".into(), "stored-data".into(), "summary".into(), "tagged-document".into(), "tagged-process".into(), "paper-tape".into(), "lined-document".into(), "data-store".into(), "titled".into()], classes: Vec::new(), style: Default::default() }],
            groups: vec![], nodes: vec![
                SwimlaneNode { id: "hosted".into(), label: "Hosted".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::Cloud, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "alert".into(), label: "Alert".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::Bang, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "collect".into(), label: "Collect".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::Hourglass, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "extract".into(), label: "Extract".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::Triangle, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "manual".into(), label: "Manual".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::InvertedTriangle, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "card".into(), label: "Card".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::NotchedRect, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "lined".into(), label: "Lined".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::LinedRect, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "text".into(), label: "Text".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::TextBlock, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "start".into(), label: "Start".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::SmallCircle, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "stop".into(), label: "Stop".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::FramedCircle, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "fork".into(), label: "Fork".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::ForkJoin, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "bolt".into(), label: "Bolt".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::LightningBolt, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "brace-left".into(), label: "Left comment".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::CurlyBraceLeft, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "brace-right".into(), label: "Right comment".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::CurlyBraceRight, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "braces".into(), label: "Both comments".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::CurlyBraces, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "document".into(), label: "Document".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::Document, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "delay".into(), label: "Delay".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::Delay, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "horizontal-cylinder".into(), label: "Storage".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::HorizontalCylinder, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "lined-cylinder".into(), label: "Disk".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::LinedCylinder, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "display".into(), label: "Display".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::Display, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "divided".into(), label: "Divided".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::DividedRect, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "window-pane".into(), label: "Internal".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::WindowPane, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "junction".into(), label: "Junction".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::FilledCircle, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "loop-limit".into(), label: "Loop limit".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::LoopLimit, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "manual-input".into(), label: "Manual input".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::ManualInput, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "multi-document".into(), label: "Documents".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::MultiDocument, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "multi-process".into(), label: "Processes".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::MultiProcess, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "stored-data".into(), label: "Stored data".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::StoredData, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "summary".into(), label: "Summary".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::Summary, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "tagged-document".into(), label: "Tagged document".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::TaggedDocument, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "tagged-process".into(), label: "Tagged process".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::TaggedProcess, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "paper-tape".into(), label: "Paper tape".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::PaperTape, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "lined-document".into(), label: "Lined document".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::LinedDocument, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "data-store".into(), label: "Data store".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::DataStore, classes: Vec::new(), style: Default::default() },
                SwimlaneNode { id: "titled".into(), label: "Details".into(), lane_id: Some("runtime".into()),
                    shape: DiagramShape::TitledRect, classes: Vec::new(), style: Default::default() },
            ],
            edges: vec![], links: Vec::new(), callbacks: Vec::new(), line_hops: None, automatic_lane_ordering: false,
            ignore_cross_lane_edges: true,
            optimize_ranks_by_crossings: true,
            flowchart_config: Default::default(),
            theme: Default::default(),
        };
        let layout = layout_swimlane(&diagram);
        assert_eq!(layout.nodes[0].shape, DiagramShape::Cloud);
        assert_eq!(layout.nodes[1].shape, DiagramShape::Bang);
        assert_eq!(layout.nodes[2].shape, DiagramShape::Hourglass);
        assert_eq!(layout.nodes[3].shape, DiagramShape::Triangle);
        assert_eq!(layout.nodes[4].shape, DiagramShape::InvertedTriangle);
        assert_eq!(layout.nodes[5].shape, DiagramShape::NotchedRect);
        assert_eq!(layout.nodes[6].shape, DiagramShape::LinedRect);
        assert_eq!(layout.nodes[7].shape, DiagramShape::TextBlock);
        assert_eq!(layout.nodes[8].shape, DiagramShape::SmallCircle);
        assert_eq!((layout.nodes[8].width, layout.nodes[8].height), (14.0, 14.0));
        assert_eq!(layout.nodes[9].shape, DiagramShape::FramedCircle);
        assert_eq!((layout.nodes[9].width, layout.nodes[9].height), (14.0, 14.0));
        assert_eq!(layout.nodes[10].shape, DiagramShape::ForkJoin);
        assert_eq!((layout.nodes[10].width, layout.nodes[10].height), (10.0, 70.0));
        assert_eq!(layout.nodes[11].shape, DiagramShape::LightningBolt);
        assert_eq!((layout.nodes[11].width, layout.nodes[11].height), (35.0, 70.0));
        assert_eq!(layout.nodes[12].shape, DiagramShape::CurlyBraceLeft);
        assert_eq!(layout.nodes[13].shape, DiagramShape::CurlyBraceRight);
        assert_eq!(layout.nodes[14].shape, DiagramShape::CurlyBraces);
        assert!(layout.nodes[12..=14].iter().all(|node| node.width == 132.0 && node.height == 54.0));
        assert_eq!(layout.nodes[15].shape, DiagramShape::Document);
        assert_eq!((layout.nodes[15].width, layout.nodes[15].height), (132.0, 54.0));
        assert_eq!(layout.nodes[16].shape, DiagramShape::Delay);
        assert_eq!((layout.nodes[16].width, layout.nodes[16].height), (132.0, 54.0));
        assert_eq!(layout.nodes[17].shape, DiagramShape::HorizontalCylinder);
        assert_eq!((layout.nodes[17].width, layout.nodes[17].height), (132.0, 54.0));
        assert_eq!(layout.nodes[18].shape, DiagramShape::LinedCylinder);
        assert_eq!((layout.nodes[18].width, layout.nodes[18].height), (132.0, 54.0));
        assert_eq!(layout.nodes[19].shape, DiagramShape::Display);
        assert_eq!((layout.nodes[19].width, layout.nodes[19].height), (132.0, 54.0));
        assert_eq!(layout.nodes[20].shape, DiagramShape::DividedRect);
        assert_eq!((layout.nodes[20].width, layout.nodes[20].height), (132.0, 54.0));
        assert_eq!(layout.nodes[21].shape, DiagramShape::WindowPane);
        assert_eq!((layout.nodes[21].width, layout.nodes[21].height), (132.0, 54.0));
        assert_eq!(layout.nodes[22].shape, DiagramShape::FilledCircle);
        assert_eq!((layout.nodes[22].width, layout.nodes[22].height), (14.0, 14.0));
        assert_eq!(layout.nodes[23].shape, DiagramShape::LoopLimit);
        assert_eq!((layout.nodes[23].width, layout.nodes[23].height), (132.0, 54.0));
        assert_eq!(layout.nodes[24].shape, DiagramShape::ManualInput);
        assert_eq!((layout.nodes[24].width, layout.nodes[24].height), (132.0, 54.0));
        assert_eq!(layout.nodes[25].shape, DiagramShape::MultiDocument);
        assert_eq!((layout.nodes[25].width, layout.nodes[25].height), (132.0, 54.0));
        assert_eq!(layout.nodes[26].shape, DiagramShape::MultiProcess);
        assert_eq!((layout.nodes[26].width, layout.nodes[26].height), (132.0, 54.0));
        assert_eq!(layout.nodes[27].shape, DiagramShape::StoredData);
        assert_eq!((layout.nodes[27].width, layout.nodes[27].height), (132.0, 54.0));
        assert_eq!(layout.nodes[28].shape, DiagramShape::Summary);
        assert_eq!((layout.nodes[28].width, layout.nodes[28].height), (60.0, 60.0));
        assert_eq!(layout.nodes[29].shape, DiagramShape::TaggedDocument);
        assert_eq!((layout.nodes[29].width, layout.nodes[29].height), (136.0, 54.0));
        assert_eq!(layout.nodes[30].shape, DiagramShape::TaggedProcess);
        assert_eq!((layout.nodes[30].width, layout.nodes[30].height), (132.0, 54.0));
        assert_eq!(layout.nodes[31].shape, DiagramShape::PaperTape);
        assert_eq!((layout.nodes[31].width, layout.nodes[31].height), (132.0, 54.0));
        assert_eq!(layout.nodes[32].shape, DiagramShape::LinedDocument);
        assert_eq!((layout.nodes[32].width, layout.nodes[32].height), (132.0, 54.0));
        assert_eq!(layout.nodes[33].shape, DiagramShape::DataStore);
        assert_eq!((layout.nodes[33].width, layout.nodes[33].height), (132.0, 54.0));
        assert_eq!(layout.nodes[34].shape, DiagramShape::TitledRect);
        assert_eq!((layout.nodes[34].width, layout.nodes[34].height), (132.0, 54.0));
    }

    #[test]
    fn railroad_layout_preserves_choice_branches_and_repetition_loopbacks() {
        use diagram_ir::{RailroadDiagram, RailroadExpression, RailroadRule};
        let diagram = RailroadDiagram {
            title: Some("Number".into()), accessibility_title: None, accessibility_description: None,
            config: Default::default(),
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
    fn railroad_layout_uses_upstream_rule_and_special_display_labels() {
        use diagram_ir::{RailroadDiagram, RailroadExpression, RailroadRule};
        let diagram = RailroadDiagram {
            title: None, accessibility_title: None, accessibility_description: None,
            config: Default::default(),
            rules: vec![
                RailroadRule { name: "x".into(), definition: RailroadExpression::Special("integer".into()) },
                RailroadRule { name: "long-rule-name".into(), definition: RailroadExpression::Terminal("value".into()) },
            ],
        };

        let layout = layout_railroad(&diagram);
        assert_eq!(layout.rules[0].elements[0].label, "? integer ?");
        assert!(layout.rules[1].start_marker.x > layout.rules[0].start_marker.x);
        for rule in &layout.rules {
            assert_eq!(rule.start_marker.y, rule.end_marker.y);
            assert_eq!(rule.paths[rule.paths.len() - 2].points[0].x,
                rule.start_marker.x + layout.config.marker_radius);
            assert_eq!(rule.paths[rule.paths.len() - 1].points[1].x,
                rule.end_marker.x - layout.config.marker_radius);
        }
    }

    #[test]
    fn railroad_layout_sizes_nested_repetition_without_clipping() {
        use diagram_ir::{RailroadDiagram, RailroadExpression, RailroadRule};
        let diagram = RailroadDiagram {
            title: None, accessibility_title: None, accessibility_description: None,
            config: Default::default(),
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

    #[test]
    fn railroad_layout_applies_configured_spacing() {
        use diagram_ir::{RailroadConfig, RailroadDiagram, RailroadExpression, RailroadRule};
        let expression = RailroadExpression::Sequence(vec![
            RailroadExpression::Terminal("a".into()),
            RailroadExpression::Choice(vec![
                RailroadExpression::Terminal("b".into()),
                RailroadExpression::Terminal("c".into()),
            ]),
        ]);
        let compact = layout_railroad(&RailroadDiagram {
            title: None, accessibility_title: None, accessibility_description: None,
            config: RailroadConfig { padding: 4.0, vertical_separation: 2.0, horizontal_separation: 3.0, ..Default::default() },
            rules: vec![RailroadRule { name: "value".into(), definition: expression.clone() }],
        });
        let spacious = layout_railroad(&RailroadDiagram {
            title: None, accessibility_title: None, accessibility_description: None,
            config: RailroadConfig { padding: 30.0, vertical_separation: 40.0, horizontal_separation: 50.0, ..Default::default() },
            rules: vec![RailroadRule { name: "value".into(), definition: expression }],
        });
        assert!(spacious.width > compact.width);
        assert!(spacious.height > compact.height);
        assert!(spacious.rules[0].elements[1].x > compact.rules[0].elements[1].x);
    }

    #[test]
    fn railroad_layout_applies_configured_arc_radius_to_branch_geometry() {
        use diagram_ir::{RailroadConfig, RailroadDiagram, RailroadExpression, RailroadRule};
        let expression = RailroadExpression::Optional(Box::new(RailroadExpression::Choice(vec![
            RailroadExpression::Terminal("a".into()),
            RailroadExpression::Terminal("b".into()),
        ])));
        let layout_with_radius = |arc_radius| layout_railroad(&RailroadDiagram {
            title: None, accessibility_title: None, accessibility_description: None,
            config: RailroadConfig { arc_radius, ..Default::default() },
            rules: vec![RailroadRule { name: "value".into(), definition: expression.clone() }],
        });
        let tight = layout_with_radius(4.0);
        let wide = layout_with_radius(24.0);
        assert!(wide.rules[0].end_marker.x > tight.rules[0].end_marker.x);
        assert!(wide.width >= wide.rules[0].end_marker.x + wide.config.marker_radius + wide.config.padding);
        assert!(wide.rules[0].height > tight.rules[0].height);
        assert_ne!(wide.rules[0].paths, tight.rules[0].paths);
        for layout in [&tight, &wide] {
            let rule = &layout.rules[0];
            assert!(rule.paths.iter().flat_map(|path| &path.points).all(|point|
                point.x >= 0.0 && point.x <= layout.width
                    && point.y >= rule.y && point.y <= rule.y + rule.height));
        }
    }
}
