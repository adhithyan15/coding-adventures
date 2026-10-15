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
) -> Result<(), SwimlaneLayoutError> {
    let mut best = swimlane_rank_crossing_count(diagram, ranks);
    for _ in 0..4 {
        let mut changed = false;
        for node in &diagram.nodes {
            let mut candidate = ranks.clone();
            let rank = candidate.entry(node.id.clone()).or_default();
            *rank = rank.checked_add(1).ok_or(SwimlaneLayoutError::RankArithmeticOverflow)?;
            for _ in 0..diagram.nodes.len() {
                let mut propagated = false;
                for (from, to, distance) in constraints {
                    let next = candidate.get(from).copied().unwrap_or(0).checked_add(*distance)
                        .ok_or(SwimlaneLayoutError::RankArithmeticOverflow)?;
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
    Ok(())
}

fn swimlane_cross_lane_ranks(diagram: &SwimlaneDiagram, link_gaps: &SwimlaneLinkGaps<'_>) -> Result<Option<HashMap<String, usize>>, SwimlaneLayoutError> {
    if diagram.ignore_cross_lane_edges {
        return Ok(None);
    }
    let mut constraints = Vec::<(String, String, usize)>::new();
    for lane in &diagram.lanes {
        for pair in lane.node_ids.windows(2) {
            constraints.push((
                pair[0].clone(), pair[1].clone(),
                link_gaps.units(&pair[0], &pair[1]),
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
            let next = ranks.get(from).copied().unwrap_or(0).checked_add(*distance)
                .ok_or(SwimlaneLayoutError::RankArithmeticOverflow)?;
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
        optimize_swimlane_ranks_by_crossings(diagram, &constraints, &mut ranks)?;
    }
    Ok(Some(ranks))
}

/// Typed refusal for caller-constructed Railroad spacing or derived geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RailroadLayoutError {
    UnsupportedSpacing { field: &'static str },
    UnsupportedStyle { field: &'static str },
    InvalidDerivedGeometry,
}

impl std::fmt::Display for RailroadLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSpacing { field } => write!(f,"Railroad {field} must be finite and in 0..=65536"),
            Self::UnsupportedStyle { field: "fontSize" } => f.write_str("Railroad fontSize must be finite and in (0, 65536]"),
            Self::UnsupportedStyle { field } => write!(f,"Railroad {field} must be finite and in 0..=65536"),
            Self::InvalidDerivedGeometry => f.write_str("Railroad layout requires finite coordinates and nonnegative dimensions"),
        }
    }
}
impl std::error::Error for RailroadLayoutError {}

/// Compatibility entry for supported IR. Panics on unsupported spacing or
/// derived geometry; callers handling untrusted typed IR use the checked API.
pub fn layout_railroad(diagram: &RailroadDiagram) -> LayoutedRailroadDiagram {
    try_layout_railroad(diagram).expect("Railroad layout requires supported spacing and finite geometry")
}

/// Check all spacing before arithmetic, then validate complete derived boxes
/// and path coordinates. This is a scalar/geometry boundary, not a promise of
/// arbitrary recursive AST depth or remeasurement resource bounds.
pub fn try_layout_railroad(diagram: &RailroadDiagram) -> Result<LayoutedRailroadDiagram, RailroadLayoutError> {
    for (field,value) in [("padding",diagram.config.padding),
        ("verticalSeparation",diagram.config.vertical_separation),
        ("horizontalSeparation",diagram.config.horizontal_separation)] {
        if !value.is_finite() || !(0.0..=diagram_ir::MAX_RAILROAD_SPACING).contains(&value) {
            return Err(RailroadLayoutError::UnsupportedSpacing{field});
        }
    }
    // New style scalars must cross the same admission boundary before font
    // sizes reach clamp bounds or a downstream f32 shaper argument. Empty
    // diagrams are checked too; lack of leaves does not validate bad config.
    for (field,value) in [("fontSize",diagram.config.font_size),
        ("strokeWidth",diagram.config.stroke_width),("markerRadius",diagram.config.marker_radius)] {
        if !value.is_finite() || !(0.0..=diagram_ir::MAX_RAILROAD_SPACING).contains(&value)
            || (field=="fontSize" && value==0.0) {
            return Err(RailroadLayoutError::UnsupportedStyle{field});
        }
    }
    let layout=layout_railroad_supported(diagram);
    let rectangle=|x:f64,y:f64,w:f64,h:f64| [x,y,w,h].iter().all(|v|v.is_finite())&&w>=0.0&&h>=0.0;
    if !rectangle(0.0,0.0,layout.width,layout.height) || layout.rules.iter().any(|rule|
        !rectangle(0.0,rule.y,layout.width,rule.height)
        || rule.elements.iter().any(|e|!rectangle(e.x,e.y,e.width,e.height))
        || rule.paths.iter().flat_map(|p|&p.points).any(|p|!p.x.is_finite()||!p.y.is_finite())) {
        return Err(RailroadLayoutError::InvalidDerivedGeometry);
    }
    Ok(layout)
}

fn layout_railroad_supported(diagram: &RailroadDiagram) -> LayoutedRailroadDiagram {
    let padding = diagram.config.padding;
    let title_height = if diagram.title.is_some() { 36.0 + padding } else { 8.0 + padding };
    let expression_x = padding + 130.0;
    let marker_x = padding + 100.0;
    let mut rules = Vec::new(); let mut y = title_height; let mut width: f64 = 400.0 + padding * 2.0;
    for rule in &diagram.rules {
        let mut elements = Vec::new(); let mut paths = Vec::new();
        let content_y = y + padding + 8.0;
        let (expr_width, expr_height) = layout_railroad_expression(
            &rule.definition, expression_x, content_y, &diagram.config, &mut elements, &mut paths,
        );
        let center = content_y + expr_height / 2.0;
        paths.push(LayoutedRailroadPath { points: vec![Point { x: marker_x + 6.0, y: center }, Point { x: expression_x, y: center }], loopback: false });
        paths.push(LayoutedRailroadPath { points: vec![Point { x: expression_x + expr_width, y: center }, Point { x: expression_x + expr_width + 24.0, y: center }], loopback: false });
        let height = expr_height + padding * 2.0 + 24.0; width = width.max(expr_width + 168.0 + padding * 2.0);
        rules.push(LayoutedRailroadRule { name: rule.name.clone(), y, height, elements, paths });
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
            let text_width = label.chars().count() as f64 * config.font_size * 0.61;
            let width = text_width.clamp(config.font_size * 3.15, config.font_size * 13.72) + config.padding * 2.0;
            let height = config.font_size * 1.15 + config.padding * 2.0;
            elements.push(LayoutedRailroadElement { kind, label: label.clone(), x, y, width, height }); (width, height)
        }
        RailroadExpression::Sequence(items) => {
            let sizes: Vec<_> = items.iter().map(|item| measure_railroad_expression(item, config)).collect();
            // A sequence has exactly n-1 gaps. Keep its measured extent rather
            // than recover it by subtracting a trailing gap from a large cursor.
            let width=sizes.iter().map(|size|size.0).sum::<f64>()
                +config.horizontal_separation*items.len().saturating_sub(1) as f64;
            let max_height = sizes.iter().map(|size| size.1).fold(36.0, f64::max);
            let center = y + max_height / 2.0;
            let mut cursor = x; let mut previous_end = None;
            for (index,(item, (_, item_height))) in items.iter().zip(sizes).enumerate() {
                let (item_width, _) = layout_railroad_expression(item, cursor, y + (max_height - item_height) / 2.0, config, elements, paths);
                if let Some(end) = previous_end { paths.push(LayoutedRailroadPath { points: vec![Point { x: end, y: center }, Point { x: cursor, y: center }], loopback: false }); }
                previous_end = Some(cursor + item_width); cursor += item_width;
                if index+1<items.len() {cursor+=config.horizontal_separation;}
            }
            (width, max_height)
        }
        RailroadExpression::Choice(alternatives) => {
            let gap = config.vertical_separation; let mut cursor_y = y; let mut max_width: f64 = 0.0; let mut sizes = Vec::new();
            for alternative in alternatives { let size = layout_railroad_expression(alternative, x + 28.0, cursor_y, config, elements, paths); sizes.push((cursor_y, size)); cursor_y += size.1 + gap; max_width = max_width.max(size.0); }
            let height = (cursor_y - y - gap).max(36.0); let middle = y + height / 2.0;
            for (alt_y, (alt_width, alt_height)) in sizes { let alt_center = alt_y + alt_height / 2.0;
                paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + 14.0, y: alt_center }, Point { x: x + 28.0, y: alt_center }], loopback: false });
                paths.push(LayoutedRailroadPath { points: vec![Point { x: x + 28.0 + alt_width, y: alt_center }, Point { x: x + 42.0 + max_width, y: alt_center }, Point { x: x + 56.0 + max_width, y: middle }], loopback: false }); }
            (max_width + 56.0, height)
        }
        RailroadExpression::Optional(element) => {
            let inner_y = y + 14.0;
            let (inner_width, inner_height) = layout_railroad_expression(element, x + 24.0, inner_y, config, elements, paths);
            let height = inner_height + 28.0; let middle = inner_y + inner_height / 2.0; let bypass_y = y + 4.0;
            let exit_x = x + inner_width + 48.0;
            paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + 24.0, y: middle }], loopback: false });
            paths.push(LayoutedRailroadPath { points: vec![Point { x: x + 24.0 + inner_width, y: middle }, Point { x: exit_x, y: middle }], loopback: false });
            paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + 12.0, y: bypass_y }, Point { x: exit_x - 12.0, y: bypass_y }, Point { x: exit_x, y: middle }], loopback: false });
            (inner_width + 48.0, height)
        }
        RailroadExpression::Repetition { element, min, .. } => {
            let inner_y = y + 15.0;
            let (inner_width, inner_height) = layout_railroad_expression(element, x, inner_y, config, elements, paths);
            let height = inner_height + 30.0; let middle = inner_y + inner_height / 2.0; let loop_y = inner_y + inner_height + 9.0;
            paths.push(LayoutedRailroadPath { points: vec![Point { x: x + inner_width, y: middle }, Point { x: x + inner_width, y: loop_y }, Point { x, y: loop_y }, Point { x, y: middle }], loopback: true });
            if *min == 0 { paths.push(LayoutedRailroadPath { points: vec![Point { x, y: middle }, Point { x: x + 12.0, y: y + 4.0 }, Point { x: x + inner_width - 12.0, y: y + 4.0 }, Point { x: x + inner_width, y: middle }], loopback: false }); }
            (inner_width, height)
        }
    }
}

fn measure_railroad_expression(expr: &RailroadExpression, config: &diagram_ir::RailroadConfig) -> (f64, f64) {
    match expr {
        RailroadExpression::Terminal(label) | RailroadExpression::NonTerminal(label) | RailroadExpression::Special(label) => {
            let text_width = label.chars().count() as f64 * config.font_size * 0.61;
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
            let width = sizes.iter().map(|size| size.0).fold(0.0, f64::max) + 56.0;
            let height = sizes.iter().map(|size| size.1).sum::<f64>()
                + config.vertical_separation * alternatives.len().saturating_sub(1) as f64;
            (width, height.max(36.0))
        }
        RailroadExpression::Optional(element) => {
            let (width, height) = measure_railroad_expression(element, config);
            (width + 48.0, height + 28.0)
        }
        RailroadExpression::Repetition { element, .. } => {
            let (width, height) = measure_railroad_expression(element, config);
            (width, height + 30.0)
        }
    }
}

/// Lay out top-level Mermaid subgraphs as stable ownership lanes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwimlaneLayoutError {
    UnsupportedSpacing { field: &'static str },
    InvalidDerivedGeometry,
    RankArithmeticOverflow,
}

impl std::fmt::Display for SwimlaneLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSpacing { field } => write!(f, "Swimlane flowchart {field} must be finite and in 0..=65536"),
            Self::RankArithmeticOverflow => f.write_str("Swimlane rank arithmetic overflow"),
            Self::InvalidDerivedGeometry => f.write_str("Swimlane layout requires finite coordinates and nonnegative dimensions"),
        }
    }
}

impl std::error::Error for SwimlaneLayoutError {}

/// Checked boundary for caller-constructed IR. No invalid spacing reaches the
/// arithmetic, and no invalid derived geometry leaves this boundary.
pub fn try_layout_swimlane(diagram: &SwimlaneDiagram) -> Result<LayoutedSwimlaneDiagram, SwimlaneLayoutError> {
    for (field, value) in [
        ("nodeSpacing", diagram.flowchart_config.node_spacing),
        ("rankSpacing", diagram.flowchart_config.rank_spacing),
        ("diagramPadding", diagram.flowchart_config.diagram_padding),
        ("padding", diagram.flowchart_config.padding),
        ("titleTopMargin", diagram.flowchart_config.title_top_margin),
        ("subGraphTitleMargin.top", diagram.flowchart_config.subgraph_title_margin_top),
        ("subGraphTitleMargin.bottom", diagram.flowchart_config.subgraph_title_margin_bottom),
    ] {
        if value.is_some_and(|v| !v.is_finite() || !(0.0..=diagram_ir::MAX_SWIMLANE_FLOWCHART_SPACING).contains(&v)) {
            return Err(SwimlaneLayoutError::UnsupportedSpacing { field });
        }
    }
    let layout = layout_swimlane_supported(diagram)?;
    let rectangle = |x: f64, y: f64, width: f64, height: f64| {
        [x,y,width,height].iter().all(|v| v.is_finite()) && width >= 0.0 && height >= 0.0
    };
    if !rectangle(0.0, 0.0, layout.width, layout.height)
        || layout.lanes.iter().any(|v| !rectangle(v.x,v.y,v.width,v.height))
        || layout.groups.iter().any(|v| !rectangle(v.x,v.y,v.width,v.height))
        || layout.nodes.iter().any(|v| !rectangle(v.x,v.y,v.width,v.height))
        || layout.edges.iter().any(|v| ![v.from.x,v.from.y,v.to.x,v.to.y].iter().all(|v|v.is_finite()))
    {
        return Err(SwimlaneLayoutError::InvalidDerivedGeometry);
    }
    Ok(layout)
}

/// Layout supported Swimlane input. Panics explicitly for unsupported spacing
/// or invalid derived geometry; use try_layout_swimlane for fallible input.
pub fn layout_swimlane(diagram: &SwimlaneDiagram) -> LayoutedSwimlaneDiagram {
    try_layout_swimlane(diagram).expect("supported Swimlane layout input")
}

fn layout_swimlane_supported(diagram: &SwimlaneDiagram) -> Result<LayoutedSwimlaneDiagram, SwimlaneLayoutError> {
    // Parallel edges remain separate drawing instructions, but spacing needs
    // only their maximum constraint. Index once before either layout phase;
    // otherwise a small flat document can force two full scans per node gap.
    let link_gaps = SwimlaneLinkGaps::new(diagram);
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
    let cross_lane_ranks = swimlane_cross_lane_ranks(diagram, &link_gaps)?;
    let max_rank = cross_lane_ranks.as_ref().and_then(|ranks| ranks.values().max().copied()).unwrap_or(0);
    let rank_count = max_rank.checked_add(1).ok_or(SwimlaneLayoutError::RankArithmeticOverflow)?;
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
    // Accepted node padding may exceed the ordinary lane extent. Reserve the
    // same cross-axis inset for height as for width before centering nodes.
    // Each nested title gets its own margin band. Horizontal centering already
    // contributes30 per depth; vertical cursors and empty-group bounds do not.
    // The explicit zero branch preserves every pre-margin layout exactly.
    let title_margin = subgraph_title_margin_top + subgraph_title_margin_bottom;
    // Fixed-height shapes reach70; ordinary rectangles may be taller through
    // accepted padding. Leave a full lane-header prefix before either kind.
    let unexpanded_cross = (node_height + 18.0).max(150.0);
    let header_deficit = (58.0 - (unexpanded_cross - node_height.max(70.0)) / 2.0).max(0.0);
    let cross_title_reserve = if title_margin > 0.0 {
        title_margin * (max_group_depth + 1) as f64 + header_deficit
    } else { 0.0 };
    let nested_title_reserve = if title_margin > 0.0 {
        (title_margin + 30.0) * max_group_depth as f64
    } else { 0.0 };
    let empty_title_reserve = if title_margin > 0.0 {
        title_margin + nested_title_reserve
    } else { 0.0 };
    let group_boundary_inset = if diagram.groups.is_empty() { 0.0 } else { 6.0 };
    let lane_cross = if horizontal_flow { (node_height + 18.0).max(150.0) + cross_title_reserve } else { (rank_slot_width + 18.0).max(150.0) }
        + 60.0 * max_group_depth as f64;
    let lane_along = if cross_lane_ranks.is_some() && horizontal_flow {
        let tail = if title_margin > 0.0 { (20.0 + 2.0 * outer_padding + group_boundary_inset).max(40.0) } else { 40.0 };
        lane_header + rank_count as f64 * rank_slot_width + max_rank as f64 * node_gap + tail
    } else if cross_lane_ranks.is_some() {
        let tail = if title_margin > 0.0 { (20.0 + outer_padding + group_boundary_inset).max(28.0) } else { 28.0 };
        lane_header + rank_count as f64 * node_height + max_rank as f64 * node_gap + tail + nested_title_reserve
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
                link_gaps.units(&pair[0], &pair[1]) as f64
            ).sum::<f64>();
            let tail = if title_margin > 0.0 { (20.0 + 2.0 * outer_padding + group_boundary_inset).max(48.0) } else { 48.0 };
            lane_header + widths.iter().sum::<f64>() + node_gap * gap_units + tail
        }).fold(0.0, f64::max)
    } else {
        diagram.lanes.iter().map(|lane| {
            let gap_units = lane.node_ids.windows(2).map(|pair|
                link_gaps.units(&pair[0], &pair[1]) as f64
            ).sum::<f64>();
            let tail = if title_margin > 0.0 { (20.0 + outer_padding + group_boundary_inset).max(28.0) } else { 28.0 };
            lane_header + lane.node_ids.len().max(1) as f64 * node_height
                + (gap_units + 1.0) * node_gap + tail + nested_title_reserve
        }).fold(0.0, f64::max)
    };
    let (width, height) = if horizontal_flow {
        (
            lane_along.max(420.0).max(outer_padding * 2.0),
            title_height + diagram.lanes.len() as f64 * (lane_cross + lane_gap) + outer_padding,
        )
    } else {
        (
            diagram.lanes.len() as f64 * (lane_cross + lane_gap) + outer_padding,
            title_height + lane_along.max(420.0).max(outer_padding),
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
        let mut vertical_cursor = y + lane_header + 20.0 + nested_title_reserve;
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
                    y + (lane_cross - resolved_node_height + cross_title_reserve) / 2.0,
                )
            } else if let Some(rank) = rank {
                (
                    x + (lane_cross - resolved_node_width) / 2.0,
                    y + lane_header + 20.0 + rank as f64 * (node_height + node_gap)
                        + (node_height - resolved_node_height) / 2.0 + nested_title_reserve,
                )
            } else if horizontal_flow {
                (
                    horizontal_cursor,
                    y + (lane_cross - resolved_node_height + cross_title_reserve) / 2.0,
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
                link_gaps.units(node_id, next_id)
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
            lane.x + 20.0, lane.y + 42.0 + empty_title_reserve,
            lane.x + 120.0, lane.y + 92.0 + empty_title_reserve,
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
    Ok(LayoutedSwimlaneDiagram {
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
    })
}

#[cfg(test)]
std::thread_local! {
    static SWIMLANE_ORDER_WEIGHT_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
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
    weights.iter().fold(0_usize, |cost, edge| {
        #[cfg(test)]
        SWIMLANE_ORDER_WEIGHT_VISITS.with(|count| count.set(count.get() + 1));
        let term = edge.weight.checked_mul(positions[edge.a.as_str()].abs_diff(positions[edge.b.as_str()]))
            .expect("lane arrangement term overflow");
        cost.checked_add(term).expect("lane arrangement cost overflow")
    })
}

fn greedy_swimlane_order(
    mut order: Vec<String>, weights: &[WeightedSwimlaneEdge], source: &HashMap<&str, usize>,
) -> (Vec<String>, usize, usize) {
    let positions = order.iter().enumerate().map(|(index, lane)| (lane.as_str(), index))
        .collect::<HashMap<_, _>>();
    // Authored subgraphs have unique IDs. Keep the old last-occurrence semantics
    // for malformed direct IR instead of applying a permutation identity to it.
    if positions.len() != order.len() {
        return greedy_swimlane_order_full_cost(order, weights, source);
    }
    let mut cost = swimlane_arrangement_cost(&order, weights);
    let mut balances = source.keys().map(|lane| (*lane, 0_i128)).collect::<HashMap<_, _>>();
    for edge in weights {
        #[cfg(test)]
        SWIMLANE_ORDER_WEIGHT_VISITS.with(|count| count.set(count.get() + 1));
        let weight = i128::try_from(edge.weight).expect("lane weight must fit signed arithmetic");
        let (left, right) = if positions[edge.a.as_str()] < positions[edge.b.as_str()] {
            (edge.a.as_str(), edge.b.as_str())
        } else { (edge.b.as_str(), edge.a.as_str()) };
        let balance = balances.get_mut(left).expect("weighted lane is in the source order");
        *balance = balance.checked_sub(weight).expect("lane balance overflow");
        let balance = balances.get_mut(right).expect("weighted lane is in the source order");
        *balance = balance.checked_add(weight).expect("lane balance overflow");
    }
    let mut pair_weights = BTreeMap::<(&str, &str), i128>::new();
    for edge in weights {
        #[cfg(test)]
        SWIMLANE_ORDER_WEIGHT_VISITS.with(|count| count.set(count.get() + 1));
        let pair = if edge.a <= edge.b { (edge.a.as_str(), edge.b.as_str()) }
            else { (edge.b.as_str(), edge.a.as_str()) };
        let weight = pair_weights.entry(pair).or_default();
        *weight = weight.checked_add(i128::try_from(edge.weight).expect("lane weight must fit signed arithmetic"))
            .expect("lane pair weight overflow");
    }
    for _ in 0..order.len().max(1) {
        let mut changed = false;
        for index in 0..order.len().saturating_sub(1) {
            let (left, right) = (order[index].as_str(), order[index + 1].as_str());
            let pair = if left <= right { (left, right) } else { (right, left) };
            let twice = pair_weights.get(&pair).copied().unwrap_or(0).checked_mul(2)
                .expect("lane pair weight overflow");
            // Moving left rightwards crosses only right; every other lane keeps
            // its side. The pair contributes zero net cost and needs +2*w to
            // cancel the -w contributions already present in the balances.
            let delta = balances[left].checked_sub(balances[right])
                .and_then(|value| value.checked_add(twice)).expect("lane swap delta overflow");
            let next_cost = i128::try_from(cost).ok().and_then(|value| value.checked_add(delta))
                .and_then(|value| usize::try_from(value).ok()).expect("lane arrangement cost overflow");
            if next_cost < cost {
                cost = next_cost;
                let balance = balances.get_mut(left).expect("left lane balance");
                *balance = balance.checked_add(twice).expect("lane balance overflow");
                let balance = balances.get_mut(right).expect("right lane balance");
                *balance = balance.checked_sub(twice).expect("lane balance overflow");
                order.swap(index, index + 1);
                changed = true;
            }
        }
        if !changed { break; }
    }
    let distance = order.iter().enumerate().map(|(index, lane)| index.abs_diff(source[lane.as_str()])).sum();
    (order, cost, distance)
}

fn greedy_swimlane_order_full_cost(
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

#[cfg(test)]
#[derive(Clone)]
struct ReferenceWeightedSwimlaneEdge { a: String, b: String, weight: usize }

#[cfg(test)]
fn reference_optimize_swimlane_lane_order(diagram: &SwimlaneDiagram) -> Vec<&diagram_ir::SwimlaneLane> {
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
    let weights = counts.into_iter().map(|((a, b), weight)| ReferenceWeightedSwimlaneEdge {
        a: source_order[a].to_string(), b: source_order[b].to_string(), weight,
    }).collect::<Vec<_>>();
    if weights.is_empty() {
        return diagram.lanes.iter().collect();
    }
    let source = source_order.iter().map(|lane| (*lane).to_string()).collect::<Vec<_>>();
    let source_positions = source.iter().enumerate()
        .map(|(index, lane)| (lane.as_str(), index)).collect::<HashMap<_, _>>();
    let mut best = reference_greedy_swimlane_order(source.clone(), &weights, &source_positions);
    for restart in 0..8 {
        let mut candidate = source.clone();
        reference_deterministic_swimlane_shuffle(&mut candidate, reference_swimlane_restart_seed(&source, &weights, restart));
        let candidate = reference_greedy_swimlane_order(candidate, &weights, &source_positions);
        if candidate.1 < best.1 || (candidate.1 == best.1 && candidate.2 < best.2) {
            best = candidate;
        }
    }
    best.0.iter().filter_map(|lane_id|
        diagram.lanes.iter().find(|lane| lane.id == *lane_id)
    ).collect()
}

#[cfg(test)]
fn reference_swimlane_arrangement_cost(order: &[String], weights: &[ReferenceWeightedSwimlaneEdge]) -> usize {
    let positions = order.iter().enumerate().map(|(index, lane)| (lane.as_str(), index))
        .collect::<HashMap<_, _>>();
    weights.iter().map(|edge| edge.weight * positions[edge.a.as_str()].abs_diff(positions[edge.b.as_str()])).sum()
}

#[cfg(test)]
fn reference_greedy_swimlane_order(
    mut order: Vec<String>, weights: &[ReferenceWeightedSwimlaneEdge], source: &HashMap<&str, usize>,
) -> (Vec<String>, usize, usize) {
    let mut cost = reference_swimlane_arrangement_cost(&order, weights);
    for _ in 0..order.len().max(1) {
        let mut changed = false;
        for index in 0..order.len().saturating_sub(1) {
            order.swap(index, index + 1);
            let next_cost = reference_swimlane_arrangement_cost(&order, weights);
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

#[cfg(test)]
fn reference_swimlane_restart_seed(source: &[String], weights: &[ReferenceWeightedSwimlaneEdge], restart: usize) -> u32 {
    let mut sorted_weights = weights.to_vec();
    sorted_weights.sort_by(|left, right| left.a.cmp(&right.a).then_with(|| left.b.cmp(&right.b)));
    let signature = sorted_weights.iter().map(|edge| format!("{}:{}:{}", edge.a, edge.b, edge.weight))
        .collect::<Vec<_>>().join("|");
    let input = format!("{}#{}#{restart}", source.join("|"), signature);
    input.encode_utf16().fold(2_166_136_261_u32, |hash, unit|
        (hash ^ u32::from(unit)).wrapping_mul(16_777_619)
    )
}

#[cfg(test)]
fn reference_deterministic_swimlane_shuffle(order: &mut [String], seed: u32) {
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

/// A spacing constraint is undirected even when its drawing edge has arrows.
/// Borrow keys rather than cloning IDs for every duplicate. A deterministic
/// tree gives logarithmic comparisons without assuming a hash distribution.
struct SwimlaneLinkGaps<'a> {
    by_pair: BTreeMap<(&'a str, &'a str), usize>,
}

fn swimlane_gap_pair<'a>(first: &'a str, second: &'a str) -> (&'a str, &'a str) {
    if first <= second { (first, second) } else { (second, first) }
}

impl<'a> SwimlaneLinkGaps<'a> {
    fn new(diagram: &'a SwimlaneDiagram) -> Self {
        let mut by_pair = BTreeMap::<_, usize>::new();
        for edge in swimlane_gap_edges(diagram) {
            let pair = swimlane_gap_pair(&edge.from, &edge.to);
            let units = edge.min_length.max(1);
            by_pair.entry(pair).and_modify(|maximum| *maximum = (*maximum).max(units)).or_insert(units);
        }
        Self { by_pair }
    }

    fn units(&self, first: &str, second: &str) -> usize {
        #[cfg(test)]
        SWIMLANE_GAP_QUERIES.with(|count| count.set(count.get() + 1));
        self.by_pair.get(&swimlane_gap_pair(first, second)).copied().unwrap_or(1)
    }
}

// Count actual constraint-edge visits, separately from ordinary output lowering.
// Thread-local counters keep parallel tests independent; production has no counters.
#[cfg(test)]
std::thread_local! {
    static SWIMLANE_GAP_EDGE_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static SWIMLANE_GAP_QUERIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn swimlane_gap_edges(diagram: &SwimlaneDiagram) -> impl Iterator<Item = &diagram_ir::SwimlaneEdge> {
    #[cfg(not(test))]
    { diagram.edges.iter() }
    #[cfg(test)]
    { diagram.edges.iter().inspect(|_| SWIMLANE_GAP_EDGE_VISITS.with(|count| count.set(count.get() + 1))) }
}

fn swimlane_edge_endpoints(
    from: &Point, from_width: f64, from_height: f64,
    to: &Point, to_width: f64, to_height: f64,
) -> (Point, Point) {
    // Coincident centers have no direction to normalize. Multiplying the
    // old infinite scale by zero produced NaN; choose two finite boundary
    // anchors explicitly. All noncoincident endpoints keep their old formula.
    if from.x == to.x && from.y == to.y {
        return (
            Point { x: from.x + from_width / 2.0, y: from.y },
            Point { x: to.x, y: to.y - to_height / 2.0 },
        );
    }
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
    mod railroad_spacing;
    mod railroad_style;
    use super::*;
    use diagram_ir::TreemapNode;
    mod subgraph_margin;

    fn gap_work_diagram(direction: DiagramDirection, node_count: usize, lengths: &[usize]) -> SwimlaneDiagram {
        use diagram_ir::{EdgeMarker, SwimlaneEdge, SwimlaneEdgeKind, SwimlaneLane, SwimlaneNode};
        let ids = (0..node_count).map(|index| format!("n{index}")).collect::<Vec<_>>();
        let nodes = ids.iter().map(|id| SwimlaneNode {
            id: id.clone(), label: id.clone(), lane_id: Some("lane".into()),
            shape: DiagramShape::Rect, classes: Vec::new(), style: Default::default(),
        }).collect();
        let edges = lengths.iter().enumerate().map(|(index, &min_length)| {
            let (from, to) = if index % 2 == 0 { ("n0", "n1") } else { ("n1", "n0") };
            SwimlaneEdge {
                id: Some(format!("constraint-{index}")),
                classes: vec![format!("class-{index}")],
                style: diagram_ir::DiagramStyle { stroke: Some(format!("stroke-{index}")), ..Default::default() },
                animate: index % 2 == 0,
                animation: Some(if index % 2 == 0 { diagram_ir::SwimlaneEdgeAnimation::Slow } else { diagram_ir::SwimlaneEdgeAnimation::Fast }),
                curve: Some(if index % 2 == 0 { diagram_ir::SwimlaneEdgeCurve::Linear } else { diagram_ir::SwimlaneEdgeCurve::Basis }),
                from: from.into(), to: to.into(), label: Some(format!("edge{index}")),
                kind: if index % 2 == 0 { SwimlaneEdgeKind::Directed } else { SwimlaneEdgeKind::Dotted },
                start_marker: EdgeMarker::Circle, end_marker: EdgeMarker::Cross, min_length,
            }
        }).collect();
        SwimlaneDiagram {
            direction, title: None, accessibility_title: None, accessibility_description: None,
            lanes: vec![SwimlaneLane { id: "lane".into(), label: "Lane".into(), node_ids: ids, classes: Vec::new(), style: Default::default() }],
            nodes, edges, links: Vec::new(), callbacks: Vec::new(), line_hops: None,
            automatic_lane_ordering: false,
            groups: Vec::new(),
            ignore_cross_lane_edges: true,
            optimize_ranks_by_crossings: false,
            flowchart_config: Default::default(),
            theme: Default::default(),
        }
    }

    fn ordering_diagram(lane_count: usize, pairs: &[(usize, usize, usize)]) -> SwimlaneDiagram {
        use diagram_ir::*;
        let ids = (0..lane_count).map(|i| format!("lane{i}")).collect::<Vec<_>>();
        let lanes = ids.iter().enumerate().map(|(i, id)| SwimlaneLane {
            id: id.clone(), label: id.clone(), node_ids: vec![format!("node{i}")],
            classes: Vec::new(), style: Default::default(),
        }).collect();
        let nodes = ids.iter().enumerate().map(|(i, id)| SwimlaneNode {
            id: format!("node{i}"), label: format!("Node{i}"), lane_id: Some(id.clone()),
            shape: DiagramShape::Rect, classes: Vec::new(), style: Default::default(),
        }).collect();
        let mut edges = Vec::new();
        for &(from, to, weight) in pairs {
            for _ in 0..weight {
                edges.push(SwimlaneEdge {
                    id: None, from: format!("node{from}"), to: format!("node{to}"), label: None,
                    kind: SwimlaneEdgeKind::Directed, start_marker: EdgeMarker::None,
                    end_marker: EdgeMarker::Point, min_length: 1, classes: Vec::new(),
                    style: Default::default(), animate: false, animation: None, curve: None,
                });
            }
        }
        SwimlaneDiagram {
            direction: DiagramDirection::Lr, title: None, accessibility_title: None,
            accessibility_description: None, lanes, groups: Vec::new(), nodes, edges,
            links: Vec::new(), callbacks: Vec::new(), line_hops: None,
            automatic_lane_ordering: true, ignore_cross_lane_edges: true,
            optimize_ranks_by_crossings: false,
            flowchart_config: Default::default(),
            theme: Default::default(),
        }
    }

    #[test]
    fn imported_title_margin_has_exact_checked_admission() {
        let mut failures = Vec::new();
        for direction in [DiagramDirection::Lr,DiagramDirection::Rl,DiagramDirection::Tb,DiagramDirection::Bt] {
            for titled in [false,true] {
                for margin in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,-1.0,65_536.000_001] {
                    let mut diagram = ordering_diagram(2,&[(0,1,1)]);
                    diagram.direction = direction.clone();
                    diagram.title = titled.then(|| "Owned title".to_owned());
                    diagram.flowchart_config.title_top_margin = Some(margin);
                    let before = diagram.clone();
                    match std::panic::catch_unwind(|| try_layout_swimlane(&diagram)) {
                        Ok(Err(SwimlaneLayoutError::UnsupportedSpacing { field: "titleTopMargin" })) => {}
                        Ok(Err(other)) => failures.push((direction.clone(),titled,margin.to_bits(),format!("wrong typed error: {other}"))),
                        Ok(Ok(_)) => failures.push((direction.clone(),titled,margin.to_bits(),"unexpectedly accepted".to_owned())),
                        Err(_) => failures.push((direction.clone(),titled,margin.to_bits(),"panicked".to_owned())),
                    }
                    assert_eq!(diagram.flowchart_config.title_top_margin.map(f64::to_bits),before.flowchart_config.title_top_margin.map(f64::to_bits));
                    let mut normalized_before=before;
                    let mut normalized_after=diagram;
                    normalized_before.flowchart_config.title_top_margin=None;
                    normalized_after.flowchart_config.title_top_margin=None;
                    assert_eq!(normalized_after,normalized_before);
                }
            }
        }
        assert!(failures.is_empty(),"title-margin admission failures {}/40: {failures:?}",failures.len());
    }

    #[test]
    fn imported_title_margin_preserves_complete_translated_layouts() {
        for direction in [DiagramDirection::Lr,DiagramDirection::Rl,DiagramDirection::Tb,DiagramDirection::Bt] {
            for titled in [false,true] {
                for grouped in [false,true] {
                    let mut baseline=ordering_diagram(2,&[(0,1,1)]);
                    baseline.direction=direction.clone();
                    baseline.title=titled.then(|| "Owned title".to_owned());
                    if grouped {
                        baseline.groups.push(diagram_ir::SwimlaneGroup {
                            id:"owned-group".into(),label:"Owned group".into(),lane_id:"lane0".into(),
                            parent_id:None,node_ids:vec!["node0".into()],classes:vec!["owned".into()],style:Default::default(),
                        });
                    }
                    let golden=try_layout_swimlane(&baseline).unwrap();
                    for margin in [None,Some(-0.0),Some(0.0),Some(14.0),Some(65_536.0)] {
                        let mut input=baseline.clone();
                        input.flowchart_config.title_top_margin=margin;
                        let before=input.clone();
                        let actual=try_layout_swimlane(&input).unwrap();
                        let offset=if titled {margin.unwrap_or(0.0)} else {0.0};
                        let mut expected=golden.clone();
                        expected.flowchart_config.title_top_margin=margin;
                        expected.height+=offset;
                        for lane in &mut expected.lanes {lane.y+=offset;}
                        for node in &mut expected.nodes {node.y+=offset;}
                        for group in &mut expected.groups {group.y+=offset;}
                        for edge in &mut expected.edges {edge.from.y+=offset;edge.to.y+=offset;}
                        assert_eq!(actual,expected,"{direction:?} title={titled} group={grouped} margin={margin:?}");
                        assert_eq!(input,before);
                        assert_eq!(input.flowchart_config.title_top_margin.map(f64::to_bits),margin.map(f64::to_bits));
                    }
                }
            }
        }
    }

    #[test]
    fn imported_padding_checked_admission_never_panics() {
        let mut failures = Vec::new();
        for direction in [DiagramDirection::Lr, DiagramDirection::Rl, DiagramDirection::Tb, DiagramDirection::Bt] {
            for padding in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 65_536.000_001] {
                let mut diagram = ordering_diagram(2, &[(0, 1, 1)]);
                diagram.direction = direction.clone();
                diagram.flowchart_config.padding = Some(padding);
                let before = diagram.flowchart_config.padding.unwrap().to_bits();
                let result = std::panic::catch_unwind(|| try_layout_swimlane(&diagram));
                if !matches!(result, Ok(Err(SwimlaneLayoutError::UnsupportedSpacing { field: "padding" }))) {
                    failures.push((direction.clone(), before, format!("{result:?}")));
                }
                assert_eq!(diagram.flowchart_config.padding.unwrap().to_bits(), before);
            }
        }
        assert!(failures.is_empty(), "checked padding failures {}/20: {failures:?}", failures.len());
    }

    #[test]
    fn imported_padding_accepted_boundaries_keep_finite_layout() {
        for direction in [DiagramDirection::Lr, DiagramDirection::Rl, DiagramDirection::Tb, DiagramDirection::Bt] {
            for padding in [None, Some(-0.0), Some(0.0), Some(14.0), Some(65_536.0)] {
                let mut diagram = ordering_diagram(2, &[(0, 1, 1)]);
                diagram.direction = direction.clone();
                diagram.flowchart_config.padding = padding;
                let before = diagram.clone();
                let layout = try_layout_swimlane(&diagram).unwrap();
                assert_eq!(layout, layout_swimlane(&diagram));
                assert_eq!(diagram, before);
                assert!([layout.width, layout.height].iter().all(|v| v.is_finite() && *v >= 0.0));
                for node in &layout.nodes {
                    assert!(node.width.is_finite() && node.height.is_finite());
                    assert!(node.width >= 0.0 && node.height >= 0.0);
                }
            }
        }
    }

    #[test]
    fn imported_padding_complete_golden_layouts_enclose_nodes() {
        let mut failures = Vec::new();
        for direction in [DiagramDirection::Lr, DiagramDirection::Rl, DiagramDirection::Tb, DiagramDirection::Bt] {
            let mut baseline = ordering_diagram(2, &[]);
            baseline.direction = direction.clone();
            baseline.automatic_lane_ordering = false;
            let baseline_layout = layout_swimlane(&baseline);
            for padding in [None, Some(0.0), Some(14.0), Some(100.0), Some(65_536.0)] {
                let mut diagram = baseline.clone();
                diagram.flowchart_config.padding = padding;
                let before = diagram.clone();
                let actual = try_layout_swimlane(&diagram).unwrap();
                // Independent fixed two-lane arithmetic. Inherit only untouched
                // identity/style/metadata fields from the default golden scene.
                let node_width = padding.map_or(132.0, |p| 104.0 + 2.0*p);
                let node_height = padding.map_or(54.0, |p| (26.0 + 2.0*p).max(54.0));
                let horizontal = matches!(direction, DiagramDirection::Lr | DiagramDirection::Rl);
                let cross = if horizontal { (node_height+18.0).max(150.0) }
                    else { (node_width.max(132.0)+18.0).max(150.0) };
                let mut expected = baseline_layout.clone();
                expected.flowchart_config.padding = padding;
                expected.width = if horizontal { (38.0+node_width+48.0).max(420.0) } else { 2.0*(cross+12.0)+12.0 };
                expected.height = if horizontal { 16.0+2.0*(cross+12.0)+12.0 } else { 16.0+(node_height+108.0).max(420.0) };
                for (i,lane) in expected.lanes.iter_mut().enumerate() {
                    lane.x = if horizontal { 12.0 } else { 12.0+i as f64*(cross+12.0) };
                    lane.y = if horizontal { 16.0+i as f64*(cross+12.0) } else { 16.0 };
                    lane.width = if horizontal { expected.width-24.0 } else { cross };
                    lane.height = if horizontal { cross } else { expected.height-28.0 };
                    let node = &mut expected.nodes[i];
                    node.x = lane.x + if horizontal { 58.0 } else { (cross-node_width)/2.0 };
                    node.y = lane.y + if horizontal { (cross-node_height)/2.0 } else { 58.0 };
                    node.width = node_width; node.height = node_height;
                }
                let enclosure = actual.nodes.iter().all(|node| {
                    let source = diagram.nodes.iter().find(|source| source.id==node.id).unwrap();
                    let lane = actual.lanes.iter().find(|lane| Some(&lane.id)==source.lane_id.as_ref()).unwrap();
                    node.x>=lane.x && node.y>=lane.y && node.x+node.width<=lane.x+lane.width
                        && node.y+node.height<=lane.y+lane.height && node.x>=0.0 && node.y>=0.0
                        && node.x+node.width<=actual.width && node.y+node.height<=actual.height
                });
                if actual != expected || !enclosure { failures.push((direction.clone(),padding,enclosure)); }
                assert_eq!(diagram,before);
            }
        }
        assert!(failures.is_empty(), "padding golden/enclosure failures {}/20: {failures:?}",failures.len());
    }

    #[test]
    fn swimlane_ordering_dense_public_layout_bounds_weight_visits() {
        let pairs = (0..16).flat_map(|a| ((a + 1)..16).map(move |b| (a, b, 1))).collect::<Vec<_>>();
        assert_eq!(pairs.len(), 120);
        let diagram = ordering_diagram(16, &pairs);
        let reference_order = reference_optimize_swimlane_lane_order(&diagram);
        let mut expected_input = diagram.clone();
        expected_input.lanes = reference_order.into_iter().cloned().collect();
        expected_input.automatic_lane_ordering = false;
        let mut expected = layout_swimlane(&expected_input);
        expected.automatic_lane_ordering = true;
        SWIMLANE_ORDER_WEIGHT_VISITS.with(|count| count.set(0));
        let actual = layout_swimlane(&diagram);
        let visits = SWIMLANE_ORDER_WEIGHT_VISITS.with(|count| count.get());
        assert_eq!(actual, expected, "complete layout and metadata retain reference output");
        eprintln!("dense ordering: {visits} actual weighted-edge visits");
        assert!(visits <= 27 * pairs.len(), "{visits} actual weighted-edge visits exceed nine-start initialization bound {}", 27 * pairs.len());
    }

    #[test]
    fn swimlane_ordering_all_four_lane_costs_orders_and_distances_match_reference() {
        fn permutations(rest: Vec<String>, prefix: Vec<String>, output: &mut Vec<Vec<String>>) {
            if rest.is_empty() { output.push(prefix); return; }
            for i in 0..rest.len() {
                let mut remaining = rest.clone();
                let mut next = prefix.clone();
                next.push(remaining.remove(i));
                permutations(remaining, next, output);
            }
        }
        let ids = ["Lé", "🛰", "Lane", "中"].map(str::to_owned).to_vec();
        let positions = ids.iter().enumerate().map(|(i, id)| (id.as_str(), i)).collect::<HashMap<_, _>>();
        let mut orders = Vec::new();
        permutations(ids.clone(), Vec::new(), &mut orders);
        assert_eq!(orders.len(), 24);
        for graph in 0..729 {
            let mut digits = graph;
            let mut weights = Vec::new();
            let mut reference_weights = Vec::new();
            let mut pairs = Vec::new();
            for a in 0..4 {
                for b in (a + 1)..4 {
                    let weight = digits % 3; digits /= 3;
                    pairs.push((a, b, weight));
                    weights.push(WeightedSwimlaneEdge { a: ids[a].clone(), b: ids[b].clone(), weight });
                    reference_weights.push(ReferenceWeightedSwimlaneEdge { a: ids[a].clone(), b: ids[b].clone(), weight });
                }
            }
            let mut diagram = ordering_diagram(4, &pairs);
            for (i, lane) in diagram.lanes.iter_mut().enumerate() {
                lane.id = ids[i].clone(); diagram.nodes[i].lane_id = Some(ids[i].clone());
            }
            assert_eq!(optimize_swimlane_lane_order(&diagram).iter().map(|lane| &lane.id).collect::<Vec<_>>(),
                reference_optimize_swimlane_lane_order(&diagram).iter().map(|lane| &lane.id).collect::<Vec<_>>(),
                "all nine starts and UTF16 seeds, graph={graph}");
            for order in &orders {
                assert_eq!(greedy_swimlane_order(order.clone(), &weights, &positions),
                    reference_greedy_swimlane_order(order.clone(), &reference_weights, &positions),
                    "graph={graph}, initial={order:?}");
            }
        }
    }

    #[test]
    fn swimlane_ordering_non_bijective_direct_orders_keep_full_cost_semantics() {
        let order = ["a", "a", "b", "c"].map(str::to_owned).to_vec();
        let source = order.iter().enumerate().map(|(i, id)| (id.as_str(), i)).collect::<HashMap<_, _>>();
        let weights = vec![WeightedSwimlaneEdge { a: "a".into(), b: "c".into(), weight: 3 }];
        let reference = vec![ReferenceWeightedSwimlaneEdge { a: "a".into(), b: "c".into(), weight: 3 }];
        assert_eq!(greedy_swimlane_order(order.clone(), &weights, &source),
            reference_greedy_swimlane_order(order.clone(), &reference, &source));
    }

    #[test]
    fn swimlane_ordering_cost_overflow_is_refused_before_score_comparison() {
        let order = ["a", "b", "c"].map(str::to_owned).to_vec();
        let weights = vec![WeightedSwimlaneEdge { a: "a".into(), b: "c".into(), weight: usize::MAX }];
        let source = order.iter().enumerate().map(|(i, id)| (id.as_str(), i)).collect::<HashMap<_, _>>();
        let error = std::panic::catch_unwind(|| greedy_swimlane_order(order.clone(), &weights, &source));
        assert!(error.is_err(), "an overflowing distance term must never become a lower score");
    }

    #[test]
    fn swimlane_ordering_public_layout_matches_full_reference_across_directions() {
        use diagram_ir::{SwimlaneEdgeKind, SwimlaneLineHops};
        for direction in [DiagramDirection::Lr, DiagramDirection::Rl, DiagramDirection::Tb, DiagramDirection::Bt] {
            for hops in [false, true] {
                for pairs in [vec![], vec![(0, 3, 2), (3, 0, 1), (1, 2, 3), (2, 2, 2)], vec![(0, 1, 1), (1, 2, 1), (2, 3, 1)]] {
                    let mut diagram = ordering_diagram(4, &pairs);
                    diagram.direction = direction.clone();
                    diagram.line_hops = hops.then_some(SwimlaneLineHops::Arc);
                    if let Some(edge) = diagram.edges.first().cloned() {
                        let mut invisible = edge; invisible.kind = SwimlaneEdgeKind::Invisible;
                        diagram.edges.push(invisible);
                    }
                    let mut expected_input = diagram.clone();
                    expected_input.lanes = reference_optimize_swimlane_lane_order(&diagram).into_iter().cloned().collect();
                    expected_input.automatic_lane_ordering = false;
                    let mut expected = layout_swimlane(&expected_input); expected.automatic_lane_ordering = true;
                    let mut actual = layout_swimlane(&diagram);
                    // Preserve every endpoint bit and complete reference
                    // equality; repaired anchors must additionally be finite.
                    for (actual_edge, expected_edge) in actual.edges.iter_mut().zip(expected.edges.iter_mut()) {
                        for (actual_coordinate, expected_coordinate) in [
                            (&mut actual_edge.from.x, &mut expected_edge.from.x),
                            (&mut actual_edge.from.y, &mut expected_edge.from.y),
                            (&mut actual_edge.to.x, &mut expected_edge.to.x),
                            (&mut actual_edge.to.y, &mut expected_edge.to.y),
                        ] {
                            assert_eq!(actual_coordinate.to_bits(), expected_coordinate.to_bits());
                            assert!(actual_coordinate.is_finite() && expected_coordinate.is_finite());
                        }
                    }
                    assert_eq!(actual, expected, "{direction:?}, hops={hops}, pairs={pairs:?}");
                }
            }
        }
    }

    #[test]
    fn coincident_swimlane_centers_have_exact_finite_boundary_anchors() {
        use diagram_ir::SwimlaneLineHops;
        for direction in [DiagramDirection::Lr, DiagramDirection::Rl, DiagramDirection::Tb, DiagramDirection::Bt] {
            for ordering in [false, true] {
                for hops in [false, true] {
                    let mut diagram = ordering_diagram(1, &[(0,0,2)]);
                    diagram.direction = direction.clone();
                    diagram.automatic_lane_ordering = ordering;
                    diagram.line_hops = hops.then_some(SwimlaneLineHops::Arc);
                    let layout = try_layout_swimlane(&diagram).unwrap();
                    let node = &layout.nodes[0];
                    assert_eq!(layout.edges.len(), 2);
                    for edge in &layout.edges {
                        assert_eq!(edge.from, Point { x: node.x + node.width, y: node.y + node.height / 2.0 });
                        assert_eq!(edge.to, Point { x: node.x + node.width / 2.0, y: node.y });
                        assert!([edge.from.x,edge.from.y,edge.to.x,edge.to.y].iter().all(|v|v.is_finite()));
                    }
                }
            }
        }
        assert_eq!(swimlane_edge_endpoints(&Point { x: 10.0, y: 20.0 }, 8.0, 6.0,
            &Point { x: 10.0, y: 20.0 }, 4.0, 12.0),
            (Point { x: 14.0, y: 20.0 }, Point { x: 10.0, y: 14.0 }));
    }

    #[test]
    fn enabled_swimlane_rank_overflow_is_a_checked_refusal_across_directions() {
        let mut failures = Vec::new();
        for direction in [DiagramDirection::Lr, DiagramDirection::Rl, DiagramDirection::Tb, DiagramDirection::Bt] {
            for optimize in [false, true] {
                for (name, count, pairs) in [
                    ("rank-count", 2, vec![(0,1,1)]),
                    ("rank-accumulation", 3, vec![(0,1,1),(1,2,1)]),
                ] {
                    let mut diagram = ordering_diagram(count, &pairs);
                    diagram.direction = direction.clone();
                    diagram.ignore_cross_lane_edges = false;
                    diagram.optimize_ranks_by_crossings = optimize;
                    diagram.edges[0].min_length = usize::MAX;
                    let result = std::panic::catch_unwind(|| try_layout_swimlane(&diagram));
                    let refused = matches!(&result, Ok(Err(SwimlaneLayoutError::RankArithmeticOverflow)));
                    eprintln!("rank-overflow direction={direction:?} optimize={optimize} case={name} checked_refusal={refused} panicked={}",result.is_err());
                    if !refused { failures.push(format!("{direction:?}/{optimize}/{name}")); }
                }
            }
        }
        assert!(failures.is_empty(), "rank overflow must be an explicit checked refusal, failures={failures:?}");
    }

    #[test]
    fn crossing_rank_optimizer_refuses_candidate_and_constraint_overflow_exactly() {
        let diagram = ordering_diagram(1, &[]);
        let mut ranks = HashMap::from([("node0".to_owned(), usize::MAX)]);
        assert_eq!(optimize_swimlane_ranks_by_crossings(&diagram, &[], &mut ranks),
            Err(SwimlaneLayoutError::RankArithmeticOverflow));
        assert_eq!(ranks["node0"], usize::MAX);
        let mut ranks = HashMap::from([("node0".to_owned(), 0)]);
        let constraints = vec![("node0".to_owned(), "external".to_owned(), usize::MAX)];
        assert_eq!(optimize_swimlane_ranks_by_crossings(&diagram, &constraints, &mut ranks),
            Err(SwimlaneLayoutError::RankArithmeticOverflow));
        assert_eq!(ranks["node0"], 0);
        let mut ranks = HashMap::from([("node0".to_owned(), usize::MAX-1)]);
        assert_eq!(optimize_swimlane_ranks_by_crossings(&diagram, &[], &mut ranks), Ok(()));
        assert_eq!(ranks["node0"], usize::MAX-1, "valid nonimproving candidate must not alter accepted ranks");
        assert_eq!(SwimlaneLayoutError::RankArithmeticOverflow.to_string(), "Swimlane rank arithmetic overflow");
    }

    fn layout_gap(layout: &LayoutedSwimlaneDiagram, first: &str, second: &str) -> f64 {
        let from = layout.nodes.iter().find(|node| node.id == first).unwrap();
        let to = layout.nodes.iter().find(|node| node.id == second).unwrap();
        if matches!(layout.direction, DiagramDirection::Lr | DiagramDirection::Rl) {
            (to.x - from.x).abs() - from.width
        } else {
            (to.y - from.y).abs() - from.height
        }
    }

    #[test]
    fn duplicate_swimlane_edges_are_visited_once_for_all_layout_gaps() {
        // These are valid parser-domain constraints, including parallel/reversed
        // edges. Counters observe the actual layout path, not a benchmark model.
        let lengths = (0..512).map(|index| index % 3 + 1).collect::<Vec<_>>();
        for direction in [DiagramDirection::Lr, DiagramDirection::Rl, DiagramDirection::Tb, DiagramDirection::Bt] {
            let diagram = gap_work_diagram(direction, 64, &lengths);
            SWIMLANE_GAP_EDGE_VISITS.with(|count| count.set(0));
            SWIMLANE_GAP_QUERIES.with(|count| count.set(0));
            let layout = layout_swimlane(&diagram);
            assert_eq!(SWIMLANE_GAP_EDGE_VISITS.with(std::cell::Cell::get), lengths.len());
            assert_eq!(SWIMLANE_GAP_QUERIES.with(std::cell::Cell::get), 2 * (diagram.nodes.len() - 1));
            assert_eq!(layout_gap(&layout, "n0", "n1"), 126.0);
            assert_eq!(layout_gap(&layout, "n1", "n2"), 42.0);
            assert_eq!(layout.edges.len(), diagram.edges.len());
            for (before, after) in diagram.edges.iter().zip(&layout.edges) {
                assert_eq!(after.id, before.id);
                assert_eq!(after.classes, before.classes);
                assert_eq!(after.style, before.style);
                assert_eq!(after.animate, before.animate);
                assert_eq!(after.animation, before.animation);
                assert_eq!(after.curve, before.curve);
                assert_eq!(after.label, before.label);
                assert_eq!(after.kind, before.kind);
                assert_eq!(after.start_marker, before.start_marker);
                assert_eq!(after.end_marker, before.end_marker);
                assert_eq!(after.min_length, before.min_length);
            }
        }
    }

    #[test]
    fn enabled_swimlane_ranks_reuse_one_gap_edge_index() {
        let lengths = (0..512).map(|index| index % 3 + 1).collect::<Vec<_>>();
        for direction in [DiagramDirection::Lr, DiagramDirection::Rl, DiagramDirection::Tb, DiagramDirection::Bt] {
            let horizontal = matches!(direction, DiagramDirection::Lr | DiagramDirection::Rl);
            let mut diagram = gap_work_diagram(direction, 8, &lengths);
            diagram.ignore_cross_lane_edges = false;
            diagram.optimize_ranks_by_crossings = false;
            SWIMLANE_GAP_EDGE_VISITS.with(|count| count.set(0));
            SWIMLANE_GAP_QUERIES.with(|count| count.set(0));
            let layout = layout_swimlane(&diagram);
            assert_eq!(SWIMLANE_GAP_EDGE_VISITS.with(std::cell::Cell::get), 512);
            assert_eq!(SWIMLANE_GAP_QUERIES.with(std::cell::Cell::get), 14);
            assert_eq!(layout_gap(&layout, "n0", "n1"), if horizontal { 390.0 } else { 234.0 });
            assert_eq!(layout_gap(&layout, "n1", "n2"), 42.0);
            assert_eq!(layout.edges.len(), lengths.len());
            assert_eq!(layout.nodes.len(), 8);
        }
    }

    #[test]
    fn swimlane_gap_defaults_and_reversed_maxima_preserve_spacing() {
        for direction in [DiagramDirection::Lr, DiagramDirection::Rl, DiagramDirection::Tb, DiagramDirection::Bt] {
            let default = layout_swimlane(&gap_work_diagram(direction.clone(), 3, &[0, 0]));
            assert_eq!(layout_gap(&default, "n0", "n1"), 42.0);
            assert_eq!(layout_gap(&default, "n1", "n2"), 42.0);
            let maximum = layout_swimlane(&gap_work_diagram(direction, 3, &[2, 3, 1]));
            assert_eq!(layout_gap(&maximum, "n0", "n1"), 126.0);
            assert_eq!(layout_gap(&maximum, "n1", "n2"), 42.0);
        }
    }

    #[test]
    fn caller_owned_swimlane_lengths_do_not_overflow_integer_gap_sums() {
        for direction in [DiagramDirection::Lr, DiagramDirection::Rl, DiagramDirection::Tb, DiagramDirection::Bt] {
            let mut diagram = gap_work_diagram(direction, 3, &[usize::MAX]);
            let mut second = diagram.edges[0].clone();
            second.from = "n1".into();
            second.to = "n2".into();
            diagram.edges.push(second);
            let layout = layout_swimlane(&diagram);
            assert!(layout.width.is_finite() && layout.height.is_finite());
            assert!(layout.nodes.iter().all(|node| node.x.is_finite() && node.y.is_finite()));
            assert!(layout.edges.iter().all(|edge| edge.min_length == usize::MAX));
        }
    }

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
}
