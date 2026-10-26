//! # diagram-layout-structural
//!
//! Layout engine for structural diagrams (DG04): class, ER, C4, and Requirement.
//!
//! Takes a `StructuralDiagram` (semantic IR) and produces a
//! `LayoutedStructuralDiagram` with bounding boxes for all nodes and
//! polyline paths for all relationships.

use diagram_ir::{
    resolve_style_with_base, ArchitectureConfig, DiagramDirection, LayoutedCompartment,
    LayoutedStructuralDiagram,
    LayoutedStructuralGroup, LayoutedStructuralNode, LayoutedStructuralRelationship, Point,
    StructuralAlignmentAxis, StructuralDiagram, StructuralNode, StructuralNodeKind,
    StructuralGroupMetadata, StructuralNodeMetadata, StructuralPort, StructuralRouting,
};
use std::collections::{HashMap, HashSet};

pub const VERSION: &str = "0.30.0";

const MIN_NODE_W: f64 = 160.0;
const HEADER_H: f64 = 40.0;
const ROW_H: f64 = 20.0;
const COMP_PAD: f64 = 8.0;
const COL_GAP: f64 = 80.0;
const ROW_GAP: f64 = 60.0;
const COLS: usize = 3;
const GROUP_PAD: f64 = 24.0;
const GROUP_HEADER_H: f64 = 32.0;
const TITLE_H: f64 = 44.0;
const TITLE_LINE_H: f64 = 24.0;
const ROUTE_CLEARANCE: f64 = 12.0;
const ROUTE_BEND_COST: f64 = 8.0;
const SELF_RELATIONSHIP_GAP: f64 = 48.0;
const PARALLEL_RELATIONSHIP_GAP: f64 = 48.0;
const RELATIONSHIP_LABEL_HALF_WIDTH: f64 = 44.0;
const DEFAULT_EDGE_ELASTICITY: f64 = 0.45;

fn horizontal_gap(config: Option<&ArchitectureConfig>) -> f64 {
    config.map_or(COL_GAP, |config| config.node_separation)
}

fn vertical_gap(config: Option<&ArchitectureConfig>) -> f64 {
    config.map_or(ROW_GAP, |config| config.node_separation)
}

fn outer_padding(config: Option<&ArchitectureConfig>) -> f64 {
    config.map_or(COMP_PAD, |config| config.padding)
}

fn group_padding(diagram: &StructuralDiagram) -> f64 {
    diagram
        .architecture_config
        .as_ref()
        .map_or(GROUP_PAD, |config| config.padding)
}

fn alignment_gap(config: Option<&ArchitectureConfig>, fallback: f64) -> f64 {
    config.map_or(fallback, |config| {
        config.icon_size * config.ideal_edge_length_multiplier
    })
}

fn connected_node_gap(
    previous: &StructuralNode,
    current: &StructuralNode,
    diagram: &StructuralDiagram,
    fallback: f64,
) -> f64 {
    let Some(config) = diagram.architecture_config.as_ref() else {
        return fallback;
    };
    let connected = diagram.relationships.iter().any(|relationship| {
        !relationship.from_group
            && !relationship.to_group
            && ((relationship.from == previous.id && relationship.to == current.id)
                || (relationship.from == current.id && relationship.to == previous.id))
    });
    if previous.parent_group != current.parent_group || !connected {
        return fallback;
    }
    if config.edge_elasticity == DEFAULT_EDGE_ELASTICITY {
        fallback
    } else {
        // Preserve Mermaid's default spacing while giving stronger springs a shorter target gap.
        fallback * (1.0 + DEFAULT_EDGE_ELASTICITY - config.edge_elasticity)
    }
}

fn seeded_placement_order(
    nodes: &[StructuralNode],
    config: Option<&ArchitectureConfig>,
) -> Vec<usize> {
    let mut order = (0..nodes.len()).collect::<Vec<_>>();
    let Some(config) = config.filter(|config| config.randomize) else {
        return order;
    };
    let mut sibling_slots: HashMap<Option<&str>, Vec<usize>> = HashMap::new();
    for (index, node) in nodes.iter().enumerate() {
        sibling_slots
            .entry(node.parent_group.as_deref())
            .or_default()
            .push(index);
    }
    for (parent, slots) in sibling_slots {
        let mut values = slots.clone();
        let mut state = (config.seed as u32) ^ stable_group_hash(parent);
        for index in (1..values.len()).rev() {
            let swap_index = seeded_random(&mut state) as usize % (index + 1);
            values.swap(index, swap_index);
        }
        for (slot, value) in slots.into_iter().zip(values) {
            order[slot] = value;
        }
    }
    order
}

fn stable_group_hash(parent: Option<&str>) -> u32 {
    parent.unwrap_or("").bytes().fold(2_166_136_261, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
    })
}

fn seeded_random(state: &mut u32) -> u32 {
    *state = state.wrapping_add(0x6d2b_79f5);
    let mut value = *state;
    value = (value ^ (value >> 15)).wrapping_mul(value | 1);
    value ^= value.wrapping_add((value ^ (value >> 7)).wrapping_mul(value | 61));
    value ^ (value >> 14)
}

fn structural_style(
    node: &StructuralNode,
    architecture_config: Option<&ArchitectureConfig>,
) -> diagram_ir::ResolvedDiagramStyle {
    let font_size = architecture_config.map_or(14.0, |config| config.font_size);
    resolve_style_with_base(
        node.style.as_ref(),
        diagram_ir::ResolvedDiagramStyle {
            fill: "#f9fafb".into(),
            stroke: "#374151".into(),
            stroke_width: 1.5,
            stroke_dash: None,
            text_color: "#111827".into(),
            font_size,
            font_weight: 400,
            font_italic: false,
            font_family: "Helvetica".into(),
            corner_radius: 4.0,
        },
    )
}

/// Lay out a `StructuralDiagram` using an explicit axis or the legacy grid.
pub fn layout_structural_diagram(diagram: &StructuralDiagram) -> LayoutedStructuralDiagram {
    let mut nodes = match diagram.direction.as_ref() {
        Some(direction) => layout_directional_nodes(diagram, direction),
        None => layout_nodes(diagram),
    };
    apply_alignments(&mut nodes, diagram);
    if let Some(title) = &diagram.title {
        let title_height = structural_title_height(title);
        for node in &mut nodes {
            node.y += title_height;
        }
    }
    let groups = layout_groups(diagram, &nodes);
    let mut canvas_w = canvas_width(&nodes, &groups, diagram.architecture_config.as_ref());
    let mut canvas_h = canvas_height(&nodes, &groups, diagram.architecture_config.as_ref());
    let rels = layout_relationships(diagram, &nodes, &groups);
    for point in rels.iter().flat_map(|relationship| &relationship.points) {
        canvas_w = canvas_w.max(point.x + COMP_PAD);
        canvas_h = canvas_h.max(point.y + COMP_PAD);
    }
    for (position, _) in rels.iter().filter_map(|relationship| relationship.label.as_ref()) {
        canvas_w = canvas_w.max(position.x + RELATIONSHIP_LABEL_HALF_WIDTH + COMP_PAD);
        canvas_h = canvas_h.max(position.y + ROW_H + COMP_PAD);
    }
    LayoutedStructuralDiagram {
        width: canvas_w,
        height: canvas_h,
        title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        architecture_config: diagram.architecture_config.clone(),
        groups,
        nodes,
        relationships: rels,
    }
}

fn structural_title_height(title: &str) -> f64 {
    TITLE_H + title.split('\n').count().saturating_sub(1) as f64 * TITLE_LINE_H
}

fn apply_alignments(nodes: &mut [LayoutedStructuralNode], diagram: &StructuralDiagram) {
    let column_gap = alignment_gap(diagram.architecture_config.as_ref(), COL_GAP);
    let row_gap = alignment_gap(diagram.architecture_config.as_ref(), ROW_GAP);
    let padding = outer_padding(diagram.architecture_config.as_ref());
    for alignment in &diagram.alignments {
        let indices = alignment
            .members
            .iter()
            .filter_map(|id| nodes.iter().position(|node| &node.id == id))
            .collect::<Vec<_>>();
        if indices.len() < 2 {
            continue;
        }
        match alignment.axis {
            StructuralAlignmentAxis::Row => {
                let mut cursor = indices
                    .iter()
                    .map(|index| nodes[*index].x)
                    .fold(f64::INFINITY, f64::min);
                let natural_y = indices
                    .iter()
                    .map(|index| nodes[*index].y)
                    .fold(f64::INFINITY, f64::min);
                let group_header_y = indices
                    .iter()
                    .filter_map(|index| {
                        diagram.nodes.iter().find(|node| node.id == nodes[*index].id)
                    })
                    .map(|node| {
                        padding
                            + group_depth(node.parent_group.as_deref(), &diagram.groups) as f64
                                * GROUP_HEADER_H
                    })
                    .fold(padding, f64::max);
                let y = natural_y.max(group_header_y);
                for index in indices {
                    nodes[index].x = cursor;
                    nodes[index].y = y;
                    cursor += nodes[index].width + column_gap;
                }
            }
            StructuralAlignmentAxis::Column => {
                let x = indices
                    .iter()
                    .map(|index| nodes[*index].x)
                    .fold(f64::INFINITY, f64::min);
                let mut cursor = indices
                    .iter()
                    .map(|index| nodes[*index].y)
                    .fold(f64::INFINITY, f64::min);
                for index in indices {
                    nodes[index].x = x;
                    nodes[index].y = cursor;
                    cursor += nodes[index].height + row_gap;
                }
            }
        }
    }
}

fn node_width(node: &StructuralNode, architecture_config: Option<&ArchitectureConfig>) -> f64 {
    if node.node_kind == StructuralNodeKind::Junction {
        return 18.0 * architecture_config.map_or(1.0, |config| config.icon_size / 80.0);
    }
    let style = structural_style(node, architecture_config);
    let max_entry = node
        .compartments
        .iter()
        .flat_map(|c| c.entries.iter())
        .flat_map(|entry| entry.split('\n'))
        .map(str::len)
        .max()
        .unwrap_or(0);
    // Approximate 8 px/char + padding; header is the label
    let char_width = style.font_size * 0.57;
    let label_width = node
        .label
        .split('\n')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let text_w = (label_width.max(max_entry) as f64 * char_width + 24.0).ceil();
    let minimum_width = architecture_config.map_or(MIN_NODE_W, |config| {
        MIN_NODE_W * config.icon_size / 80.0
    });
    text_w.max(minimum_width)
}

fn node_height(node: &StructuralNode, architecture_config: Option<&ArchitectureConfig>) -> f64 {
    if node.node_kind == StructuralNodeKind::Junction {
        return 18.0 * architecture_config.map_or(1.0, |config| config.icon_size / 80.0);
    }
    let font_size = structural_style(node, architecture_config).font_size;
    let header_height = node_header_height(node, architecture_config, font_size);
    let row_height = ROW_H.max(font_size * 1.4);
    let mut h = header_height;
    for comp in &node.compartments {
        let line_count = comp
            .entries
            .iter()
            .map(|entry| entry.split('\n').count())
            .sum::<usize>();
        h += COMP_PAD + line_count as f64 * row_height + COMP_PAD;
    }
    h
}

fn node_header_height(
    node: &StructuralNode,
    architecture_config: Option<&ArchitectureConfig>,
    font_size: f64,
) -> f64 {
    let base_height = if architecture_icon_name(node).is_some() || architecture_icon_text(node).is_some() {
        architecture_config.map_or(72.0, |config| config.icon_size * 0.9).max(font_size * 2.4)
    } else {
        HEADER_H.max(font_size * 2.4)
    };
    let label_lines = node.label.split('\n').count();
    if label_lines > 1 {
        let stereotype_lines = usize::from(node.stereotype.is_some());
        base_height.max((label_lines + stereotype_lines) as f64 * font_size * 1.4 + 16.0)
    } else {
        base_height
    }
}

fn architecture_icon_name(node: &StructuralNode) -> Option<&str> {
    match node.metadata.as_ref() {
        Some(StructuralNodeMetadata::ArchitectureService(metadata)) => {
            metadata.icon_name.as_deref()
        }
        _ => None,
    }
}

fn architecture_icon_text(node: &StructuralNode) -> Option<&str> {
    match node.metadata.as_ref() {
        Some(StructuralNodeMetadata::ArchitectureService(metadata)) => {
            metadata.icon_text.as_deref()
        }
        _ => None,
    }
}

fn layout_nodes(diagram: &StructuralDiagram) -> Vec<LayoutedStructuralNode> {
    let nodes = &diagram.nodes;
    let groups = &diagram.groups;
    let architecture_config = diagram.architecture_config.as_ref();
    let mut positioned: Vec<(usize, LayoutedStructuralNode)> = Vec::with_capacity(nodes.len());
    let placement_order = seeded_placement_order(nodes, architecture_config);
    // Track max height per row so rows don't overlap.
    let padding = outer_padding(architecture_config);
    let mut row_y: Vec<f64> = vec![padding];
    let mut row_x: Vec<f64> = vec![padding];

    let mut previous_index = None;
    for (slot, node_index) in placement_order.into_iter().enumerate() {
        let node = &nodes[node_index];
        let col = slot % COLS;
        let row = slot / COLS;
        let nw = node_width(node, architecture_config);
        let nh = node_height(node, architecture_config);

        // Ensure row_y has an entry for this row.
        while row_y.len() <= row {
            row_y.push(*row_y.last().unwrap_or(&padding));
        }
        while row_x.len() <= row {
            row_x.push(padding);
        }

        let gap = if col > 0 {
            let fallback = horizontal_gap(architecture_config);
            previous_index.map_or(fallback, |previous| {
                connected_node_gap(&nodes[previous], node, diagram, fallback)
            })
        } else {
            0.0
        };
        let x = row_x[row] + gap;
        row_x[row] = x + nw;
        let y =
            row_y[row] + group_depth(node.parent_group.as_deref(), groups) as f64 * GROUP_HEADER_H;

        // Update the starting y for the next row.
        let next_row_y = y + nh + vertical_gap(architecture_config);
        if row + 1 >= row_y.len() {
            row_y.push(next_row_y);
        } else if row_y[row + 1] < next_row_y {
            row_y[row + 1] = next_row_y;
        }

        // Build layouted compartments.
        let style = structural_style(node, architecture_config);
        let row_height = ROW_H.max(style.font_size * 1.4);
        let mut y_off = node_header_height(node, architecture_config, style.font_size);
        let mut comps: Vec<LayoutedCompartment> = Vec::new();
        for comp in &node.compartments {
            let line_count = comp
                .entries
                .iter()
                .map(|entry| entry.split('\n').count())
                .sum::<usize>();
            let ch = COMP_PAD + line_count as f64 * row_height + COMP_PAD;
            comps.push(LayoutedCompartment {
                y_offset: y_off,
                height: ch,
                rows: comp.entries.clone(),
            });
            y_off += ch;
        }

        positioned.push((
            node_index,
            LayoutedStructuralNode {
                id: node.id.clone(),
                node_kind: node.node_kind.clone(),
                x,
                y,
                width: nw,
                height: nh,
                header: node.label.clone(),
                stereotype: node.stereotype.clone(),
                icon_name: architecture_icon_name(node).map(str::to_string),
                icon_text: architecture_icon_text(node).map(str::to_string),
                style: structural_style(node, architecture_config),
                compartments: comps,
            },
        ));
        previous_index = Some(node_index);
    }
    positioned.sort_by_key(|(index, _)| *index);
    positioned.into_iter().map(|(_, node)| node).collect()
}

fn layout_directional_nodes(
    diagram: &StructuralDiagram,
    direction: &DiagramDirection,
) -> Vec<LayoutedStructuralNode> {
    let nodes = &diagram.nodes;
    let groups = &diagram.groups;
    let architecture_config = diagram.architecture_config.as_ref();
    let reverse = matches!(direction, DiagramDirection::Bt | DiagramDirection::Rl);
    let vertical = matches!(direction, DiagramDirection::Tb | DiagramDirection::Bt);
    let mut indices = (0..nodes.len()).collect::<Vec<_>>();
    if reverse {
        indices.reverse();
    }

    let padding = outer_padding(architecture_config);
    let mut cursor = padding;
    let mut previous_index: Option<usize> = None;
    let mut positioned = Vec::with_capacity(nodes.len());
    for index in indices {
        let node = &nodes[index];
        let width = node_width(node, architecture_config);
        let height = node_height(node, architecture_config);
        let group_offset =
            group_depth(node.parent_group.as_deref(), groups) as f64 * GROUP_HEADER_H;
        let fallback_gap = if vertical {
            vertical_gap(architecture_config)
        } else {
            horizontal_gap(architecture_config)
        };
        if let Some(previous) = previous_index {
            cursor += connected_node_gap(&nodes[previous], node, diagram, fallback_gap);
        }
        let (x, y) = if vertical {
            let position = (padding, cursor + group_offset);
            cursor = position.1 + height;
            position
        } else {
            let position = (cursor, padding + group_offset);
            cursor = position.0 + width;
            position
        };
        previous_index = Some(index);

        let style = structural_style(node, architecture_config);
        let row_height = ROW_H.max(style.font_size * 1.4);
        let mut y_offset = if architecture_icon_name(node).is_some() || architecture_icon_text(node).is_some() {
            architecture_config.map_or(72.0, |config| config.icon_size * 0.9)
                .max(style.font_size * 2.4)
        } else {
            HEADER_H.max(style.font_size * 2.4)
        };
        let compartments = node
            .compartments
            .iter()
            .map(|compartment| {
                let compartment_height =
                    COMP_PAD + compartment.entries.len() as f64 * row_height + COMP_PAD;
                let layouted = LayoutedCompartment {
                    y_offset,
                    height: compartment_height,
                    rows: compartment.entries.clone(),
                };
                y_offset += compartment_height;
                layouted
            })
            .collect();
        positioned.push((
            index,
            LayoutedStructuralNode {
                id: node.id.clone(),
                node_kind: node.node_kind.clone(),
                x,
                y,
                width,
                height,
                header: node.label.clone(),
                stereotype: node.stereotype.clone(),
                icon_name: architecture_icon_name(node).map(str::to_string),
                icon_text: architecture_icon_text(node).map(str::to_string),
                style: structural_style(node, architecture_config),
                compartments,
            },
        ));
    }
    positioned.sort_by_key(|(index, _)| *index);
    positioned.into_iter().map(|(_, node)| node).collect()
}

fn group_depth(parent_group: Option<&str>, groups: &[diagram_ir::StructuralGroup]) -> usize {
    let mut depth = 0;
    let mut current = parent_group;
    let mut visited = HashSet::new();
    while let Some(group_id) = current {
        if !visited.insert(group_id) {
            break;
        }
        depth += 1;
        current = groups
            .iter()
            .find(|group| group.id == group_id)
            .and_then(|group| group.parent_group.as_deref());
    }
    depth
}

fn canvas_width(
    nodes: &[LayoutedStructuralNode],
    groups: &[LayoutedStructuralGroup],
    architecture_config: Option<&ArchitectureConfig>,
) -> f64 {
    let padding = outer_padding(architecture_config);
    nodes
        .iter()
        .map(|n| n.x + n.width + padding)
        .chain(groups.iter().map(|g| g.x + g.width + padding))
        .fold(200.0_f64, f64::max)
}

fn canvas_height(
    nodes: &[LayoutedStructuralNode],
    groups: &[LayoutedStructuralGroup],
    architecture_config: Option<&ArchitectureConfig>,
) -> f64 {
    let padding = outer_padding(architecture_config);
    nodes
        .iter()
        .map(|n| n.y + n.height + padding)
        .chain(groups.iter().map(|g| g.y + g.height + padding))
        .fold(100.0_f64, f64::max)
}

fn layout_groups(
    diagram: &StructuralDiagram,
    nodes: &[LayoutedStructuralNode],
) -> Vec<LayoutedStructuralGroup> {
    let mut cache: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    let mut visiting = HashSet::new();

    for group in &diagram.groups {
        group_bounds(&group.id, diagram, nodes, &mut cache, &mut visiting);
    }

    let mut groups: Vec<LayoutedStructuralGroup> = diagram
        .groups
        .iter()
        .filter_map(|group| {
            let (x, y, max_x, max_y) = cache.get(&group.id).copied()?;
            Some(LayoutedStructuralGroup {
                id: group.id.clone(),
                x,
                y,
                width: max_x - x,
                height: max_y - y,
                label: group.label.clone(),
                stereotype: group.stereotype.clone(),
                icon_name: group.metadata.as_ref().map(
                    |StructuralGroupMetadata::Architecture { icon_name }| icon_name.clone(),
                ),
                parent_group: group.parent_group.clone(),
            })
        })
        .collect();
    groups.sort_by(|a, b| {
        (b.width * b.height)
            .partial_cmp(&(a.width * a.height))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    groups
}

fn group_bounds(
    group_id: &str,
    diagram: &StructuralDiagram,
    nodes: &[LayoutedStructuralNode],
    cache: &mut HashMap<String, (f64, f64, f64, f64)>,
    visiting: &mut HashSet<String>,
) -> Option<(f64, f64, f64, f64)> {
    if let Some(bounds) = cache.get(group_id).copied() {
        return Some(bounds);
    }
    if !visiting.insert(group_id.to_string()) {
        return None;
    }

    let mut children: Vec<(f64, f64, f64, f64)> = diagram
        .nodes
        .iter()
        .filter(|node| node.parent_group.as_deref() == Some(group_id))
        .filter_map(|node| {
            nodes
                .iter()
                .find(|layouted| layouted.id == node.id)
                .map(|layouted| {
                    (
                        layouted.x,
                        layouted.y,
                        layouted.x + layouted.width,
                        layouted.y + layouted.height,
                    )
                })
        })
        .collect();

    for child in diagram
        .groups
        .iter()
        .filter(|group| group.parent_group.as_deref() == Some(group_id))
    {
        if let Some(bounds) = group_bounds(&child.id, diagram, nodes, cache, visiting) {
            children.push(bounds);
        }
    }
    visiting.remove(group_id);
    if children.is_empty() {
        return None;
    }

    let min_x = children
        .iter()
        .map(|bounds| bounds.0)
        .fold(f64::INFINITY, f64::min);
    let min_y = children
        .iter()
        .map(|bounds| bounds.1)
        .fold(f64::INFINITY, f64::min);
    let max_x = children
        .iter()
        .map(|bounds| bounds.2)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = children
        .iter()
        .map(|bounds| bounds.3)
        .fold(f64::NEG_INFINITY, f64::max);
    let padding = group_padding(diagram);
    let bounds = (
        (min_x - padding).max(0.0),
        (min_y - padding.max(GROUP_HEADER_H)).max(0.0),
        max_x + padding,
        max_y + padding,
    );
    cache.insert(group_id.to_string(), bounds);
    Some(bounds)
}

fn find_node<'a>(
    nodes: &'a [LayoutedStructuralNode],
    id: &str,
) -> Option<&'a LayoutedStructuralNode> {
    nodes.iter().find(|n| n.id == id)
}

#[derive(Clone, Copy, Debug)]
struct StructuralBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AutomaticSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl StructuralBounds {
    fn expanded(self, amount: f64) -> Self {
        Self {
            x: self.x - amount,
            y: self.y - amount,
            width: self.width + amount * 2.0,
            height: self.height + amount * 2.0,
        }
    }
}

fn closest_sides(a: StructuralBounds, b: StructuralBounds) -> (Point, Point) {
    let a_cx = a.x + a.width / 2.0;
    let a_cy = a.y + a.height / 2.0;
    let b_cx = b.x + b.width / 2.0;
    let b_cy = b.y + b.height / 2.0;
    // Use dominant axis to pick left/right vs top/bottom connector sides.
    let dx = (b_cx - a_cx).abs();
    let dy = (b_cy - a_cy).abs();
    if dx >= dy {
        // Horizontal dominant — connect right edge of A to left edge of B (or vice versa).
        if b_cx >= a_cx {
            (
                Point {
                    x: a.x + a.width,
                    y: a_cy,
                },
                Point { x: b.x, y: b_cy },
            )
        } else {
            (
                Point { x: a.x, y: a_cy },
                Point {
                    x: b.x + b.width,
                    y: b_cy,
                },
            )
        }
    } else {
        // Vertical dominant — connect bottom edge of A to top edge of B (or vice versa).
        if b_cy >= a_cy {
            (
                Point {
                    x: a_cx,
                    y: a.y + a.height,
                },
                Point { x: b_cx, y: b.y },
            )
        } else {
            (
                Point { x: a_cx, y: a.y },
                Point {
                    x: b_cx,
                    y: b.y + b.height,
                },
            )
        }
    }
}

fn automatic_side(bounds: StructuralBounds, point: &Point) -> AutomaticSide {
    if point.x == bounds.x {
        AutomaticSide::Left
    } else if point.x == bounds.x + bounds.width {
        AutomaticSide::Right
    } else if point.y == bounds.y {
        AutomaticSide::Top
    } else {
        AutomaticSide::Bottom
    }
}

fn fan_out_point(
    bounds: StructuralBounds,
    side: AutomaticSide,
    slot: usize,
    slot_count: usize,
) -> Point {
    let fraction = (slot + 1) as f64 / (slot_count + 1) as f64;
    match side {
        AutomaticSide::Left => Point {
            x: bounds.x,
            y: bounds.y + bounds.height * fraction,
        },
        AutomaticSide::Right => Point {
            x: bounds.x + bounds.width,
            y: bounds.y + bounds.height * fraction,
        },
        AutomaticSide::Top => Point {
            x: bounds.x + bounds.width * fraction,
            y: bounds.y,
        },
        AutomaticSide::Bottom => Point {
            x: bounds.x + bounds.width * fraction,
            y: bounds.y + bounds.height,
        },
    }
}

fn point_on_port(bounds: StructuralBounds, port: StructuralPort) -> Point {
    match port {
        StructuralPort::Left => Point {
            x: bounds.x,
            y: bounds.y + bounds.height / 2.0,
        },
        StructuralPort::Right => Point {
            x: bounds.x + bounds.width,
            y: bounds.y + bounds.height / 2.0,
        },
        StructuralPort::Top => Point {
            x: bounds.x + bounds.width / 2.0,
            y: bounds.y,
        },
        StructuralPort::Bottom => Point {
            x: bounds.x + bounds.width / 2.0,
            y: bounds.y + bounds.height,
        },
    }
}

fn port_is_horizontal(port: StructuralPort) -> bool {
    matches!(port, StructuralPort::Left | StructuralPort::Right)
}

fn push_distinct(points: &mut Vec<Point>, point: Point) {
    if points.last() != Some(&point) {
        points.push(point);
    }
}

fn simplify_orthogonal_path(points: Vec<Point>) -> Vec<Point> {
    let mut simplified = Vec::with_capacity(points.len());
    for point in points {
        push_distinct(&mut simplified, point);
        while simplified.len() >= 3 {
            let len = simplified.len();
            let a = &simplified[len - 3];
            let b = &simplified[len - 2];
            let c = &simplified[len - 1];
            if (a.x == b.x && b.x == c.x) || (a.y == b.y && b.y == c.y) {
                simplified.remove(len - 2);
            } else {
                break;
            }
        }
    }
    simplified
}

fn segment_crosses_obstacle(from: &Point, to: &Point, obstacle: StructuralBounds) -> bool {
    let right = obstacle.x + obstacle.width;
    let bottom = obstacle.y + obstacle.height;
    if from.y == to.y {
        let min_x = from.x.min(to.x);
        let max_x = from.x.max(to.x);
        from.y > obstacle.y && from.y < bottom && max_x > obstacle.x && min_x < right
    } else if from.x == to.x {
        let min_y = from.y.min(to.y);
        let max_y = from.y.max(to.y);
        from.x > obstacle.x && from.x < right && max_y > obstacle.y && min_y < bottom
    } else {
        true
    }
}

fn path_is_clear(points: &[Point], obstacles: &[StructuralBounds]) -> bool {
    points.windows(2).all(|segment| {
        obstacles
            .iter()
            .all(|obstacle| !segment_crosses_obstacle(&segment[0], &segment[1], *obstacle))
    })
}

fn path_cost(points: &[Point]) -> f64 {
    let distance = points
        .windows(2)
        .map(|segment| {
            (segment[1].x - segment[0].x).abs() + (segment[1].y - segment[0].y).abs()
        })
        .sum::<f64>();
    distance + points.len().saturating_sub(2) as f64 * ROUTE_BEND_COST
}

fn route_candidates(from: &Point, to: &Point, obstacles: &[StructuralBounds]) -> Vec<Vec<Point>> {
    let mut xs = vec![from.x, to.x];
    let mut ys = vec![from.y, to.y];
    for obstacle in obstacles {
        xs.extend([obstacle.x, obstacle.x + obstacle.width]);
        ys.extend([obstacle.y, obstacle.y + obstacle.height]);
    }
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    ys.sort_by(f64::total_cmp);
    ys.dedup();

    let mut candidates = vec![
        vec![from.clone(), Point { x: to.x, y: from.y }, to.clone()],
        vec![from.clone(), Point { x: from.x, y: to.y }, to.clone()],
    ];
    for x in &xs {
        candidates.push(vec![
            from.clone(),
            Point { x: *x, y: from.y },
            Point { x: *x, y: to.y },
            to.clone(),
        ]);
    }
    for y in &ys {
        candidates.push(vec![
            from.clone(),
            Point { x: from.x, y: *y },
            Point { x: to.x, y: *y },
            to.clone(),
        ]);
    }
    for x in &xs {
        for y in &ys {
            candidates.push(vec![
                from.clone(),
                Point { x: *x, y: from.y },
                Point { x: *x, y: *y },
                Point { x: to.x, y: *y },
                to.clone(),
            ]);
            candidates.push(vec![
                from.clone(),
                Point { x: from.x, y: *y },
                Point { x: *x, y: *y },
                Point { x: *x, y: to.y },
                to.clone(),
            ]);
        }
    }
    candidates
}

fn port_lead(point: &Point, port: StructuralPort) -> Point {
    match port {
        StructuralPort::Left => Point { x: point.x - ROUTE_CLEARANCE, y: point.y },
        StructuralPort::Right => Point { x: point.x + ROUTE_CLEARANCE, y: point.y },
        StructuralPort::Top => Point { x: point.x, y: point.y - ROUTE_CLEARANCE },
        StructuralPort::Bottom => Point { x: point.x, y: point.y + ROUTE_CLEARANCE },
    }
}

fn obstacle_avoiding_orthogonal_path(
    from: Point,
    to: Point,
    from_port: StructuralPort,
    to_port: StructuralPort,
    obstacles: &[StructuralBounds],
) -> Vec<Point> {
    let direct = orthogonal_path(from.clone(), to.clone(), from_port, to_port);
    if path_is_clear(&direct, obstacles) {
        return direct;
    }

    let from_lead = port_lead(&from, from_port);
    let to_lead = port_lead(&to, to_port);
    let best = route_candidates(&from_lead, &to_lead, obstacles)
        .into_iter()
        .map(simplify_orthogonal_path)
        .filter(|candidate| path_is_clear(candidate, obstacles))
        .min_by(|a, b| path_cost(a).total_cmp(&path_cost(b)));
    let Some(best) = best else {
        return direct;
    };

    simplify_orthogonal_path(
        std::iter::once(from)
            .chain(std::iter::once(from_lead))
            .chain(best)
            .chain(std::iter::once(to_lead))
            .chain(std::iter::once(to))
            .collect(),
    )
}

fn orthogonal_path(
    from: Point,
    to: Point,
    from_port: StructuralPort,
    to_port: StructuralPort,
) -> Vec<Point> {
    let mut points = vec![from.clone()];
    match (port_is_horizontal(from_port), port_is_horizontal(to_port)) {
        (true, true) => {
            let middle_x = (from.x + to.x) / 2.0;
            push_distinct(&mut points, Point { x: middle_x, y: from.y });
            push_distinct(&mut points, Point { x: middle_x, y: to.y });
        }
        (false, false) => {
            let middle_y = (from.y + to.y) / 2.0;
            push_distinct(&mut points, Point { x: from.x, y: middle_y });
            push_distinct(&mut points, Point { x: to.x, y: middle_y });
        }
        (true, false) => push_distinct(&mut points, Point { x: to.x, y: from.y }),
        (false, true) => push_distinct(&mut points, Point { x: from.x, y: to.y }),
    }
    push_distinct(&mut points, to);
    points
}

fn relationship_path(
    from: Point,
    to: Point,
    routing: StructuralRouting,
    from_port: Option<StructuralPort>,
    to_port: Option<StructuralPort>,
    obstacles: &[StructuralBounds],
) -> Vec<Point> {
    match (routing, from_port, to_port) {
        (StructuralRouting::Orthogonal, Some(from_port), Some(to_port)) => {
            obstacle_avoiding_orthogonal_path(from, to, from_port, to_port, obstacles)
        }
        (StructuralRouting::Direct, None, None)
            if !obstacles.is_empty()
                && (from.x == to.x || from.y == to.y)
                && !path_is_clear(&[from.clone(), to.clone()], obstacles) =>
        {
            let (from_port, to_port) = if from.y == to.y {
                if from.x < to.x {
                    (StructuralPort::Right, StructuralPort::Left)
                } else {
                    (StructuralPort::Left, StructuralPort::Right)
                }
            } else if from.y < to.y {
                (StructuralPort::Bottom, StructuralPort::Top)
            } else {
                (StructuralPort::Top, StructuralPort::Bottom)
            };
            obstacle_avoiding_direct_path(from, to, from_port, to_port, obstacles)
        }
        _ => vec![from, to],
    }
}

fn obstacle_avoiding_direct_path(
    from: Point,
    to: Point,
    from_port: StructuralPort,
    to_port: StructuralPort,
    obstacles: &[StructuralBounds],
) -> Vec<Point> {
    let from_lead = port_lead(&from, from_port);
    let to_lead = port_lead(&to, to_port);
    let mut points = vec![from, from_lead.clone()];
    if port_is_horizontal(from_port) {
        let lane = obstacles
            .iter()
            .map(|obstacle| obstacle.y + obstacle.height)
            .fold(0.0, f64::max)
            + ROUTE_CLEARANCE;
        points.extend([
            Point { x: from_lead.x, y: lane },
            Point { x: to_lead.x, y: lane },
        ]);
    } else {
        let lane = obstacles
            .iter()
            .map(|obstacle| obstacle.x + obstacle.width)
            .fold(0.0, f64::max)
            + ROUTE_CLEARANCE;
        points.extend([
            Point { x: lane, y: from_lead.y },
            Point { x: lane, y: to_lead.y },
        ]);
    }
    points.extend([to_lead, to]);
    simplify_orthogonal_path(points)
}

fn self_relationship_path(bounds: StructuralBounds) -> Vec<Point> {
    let right = bounds.x + bounds.width;
    let bottom = bounds.y + bounds.height;
    let start_y = bounds.y + bounds.height * 0.35;
    let end_x = bounds.x + bounds.width * 0.65;
    vec![
        Point { x: right, y: start_y },
        Point {
            x: right + SELF_RELATIONSHIP_GAP,
            y: start_y,
        },
        Point {
            x: right + SELF_RELATIONSHIP_GAP,
            y: bottom + SELF_RELATIONSHIP_GAP,
        },
        Point {
            x: end_x,
            y: bottom + SELF_RELATIONSHIP_GAP,
        },
        Point { x: end_x, y: bottom },
    ]
}

fn parallel_relationship_lane(
    diagram: &StructuralDiagram,
    relationship_index: usize,
) -> Option<f64> {
    let relationship = &diagram.relationships[relationship_index];
    if relationship.from == relationship.to
        || relationship.routing != StructuralRouting::Direct
    {
        return None;
    }
    let matching: Vec<usize> = diagram
        .relationships
        .iter()
        .enumerate()
        .filter(|(_, candidate)| {
            candidate.routing == StructuralRouting::Direct
                && ((candidate.from == relationship.from
                    && candidate.to == relationship.to
                    && candidate.from_group == relationship.from_group
                    && candidate.to_group == relationship.to_group)
                    || (candidate.from == relationship.to
                        && candidate.to == relationship.from
                        && candidate.from_group == relationship.to_group
                        && candidate.to_group == relationship.from_group))
        })
        .map(|(index, _)| index)
        .collect();
    if matching.len() < 2 {
        return None;
    }
    let ordinal = matching
        .iter()
        .position(|index| *index == relationship_index)? as f64;
    let centered = ordinal - (matching.len() as f64 - 1.0) / 2.0;
    let direction = if relationship.from <= relationship.to { 1.0 } else { -1.0 };
    Some(centered * PARALLEL_RELATIONSHIP_GAP * direction)
}

fn offset_parallel_relationship(points: Vec<Point>, offset: Option<f64>) -> Vec<Point> {
    let Some(offset) = offset else {
        return points;
    };
    let (Some(from), Some(to)) = (points.first(), points.last()) else {
        return points;
    };
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let length = dx.hypot(dy);
    if length == 0.0 {
        return points;
    }
    let normal_x = -dy / length * offset;
    let normal_y = dx / length * offset;
    if points.len() == 2 {
        return vec![
            from.clone(),
            Point {
                x: from.x + dx / 3.0 + normal_x,
                y: from.y + dy / 3.0 + normal_y,
            },
            Point {
                x: from.x + dx * 2.0 / 3.0 + normal_x,
                y: from.y + dy * 2.0 / 3.0 + normal_y,
            },
            to.clone(),
        ];
    }
    let point_count = points.len();
    points
        .into_iter()
        .enumerate()
        .map(|(index, point)| {
            if index == 0 || index + 1 == point_count {
                point
            } else {
                Point {
                    x: point.x + normal_x,
                    y: point.y + normal_y,
                }
            }
        })
        .collect()
}

fn path_midpoint(points: &[Point]) -> Point {
    let total_length: f64 = points
        .windows(2)
        .map(|segment| {
            let dx = segment[1].x - segment[0].x;
            let dy = segment[1].y - segment[0].y;
            dx.hypot(dy)
        })
        .sum();
    let mut remaining = total_length / 2.0;
    for segment in points.windows(2) {
        let dx = segment[1].x - segment[0].x;
        let dy = segment[1].y - segment[0].y;
        let length = dx.hypot(dy);
        if remaining <= length && length > 0.0 {
            let progress = remaining / length;
            return Point {
                x: segment[0].x + dx * progress,
                y: segment[0].y + dy * progress,
            };
        }
        remaining -= length;
    }
    points.last().cloned().unwrap_or(Point { x: 0.0, y: 0.0 })
}

fn group_contains_node(diagram: &StructuralDiagram, group_id: &str, node_id: &str) -> bool {
    let mut parent = diagram
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .and_then(|node| node.parent_group.as_deref());
    let mut visited = HashSet::new();
    while let Some(parent_id) = parent {
        if parent_id == group_id {
            return true;
        }
        if !visited.insert(parent_id) {
            break;
        }
        parent = diagram
            .groups
            .iter()
            .find(|group| group.id == parent_id)
            .and_then(|group| group.parent_group.as_deref());
    }
    false
}

fn relationship_obstacles(
    diagram: &StructuralDiagram,
    from_id: &str,
    to_id: &str,
    nodes: &[LayoutedStructuralNode],
    groups: &[LayoutedStructuralGroup],
) -> Vec<StructuralBounds> {
    let node_obstacles = nodes
        .iter()
        .filter(|node| node.id != from_id && node.id != to_id)
        .map(|node| StructuralBounds {
            x: node.x,
            y: node.y,
            width: node.width,
            height: node.height,
        });
    let group_obstacles = groups
        .iter()
        .filter(|group| {
            !group_contains_node(diagram, &group.id, from_id)
                && !group_contains_node(diagram, &group.id, to_id)
        })
        .map(|group| StructuralBounds {
            x: group.x,
            y: group.y,
            width: group.width,
            height: group.height,
        });
    node_obstacles
        .chain(group_obstacles)
        .map(|bounds| bounds.expanded(ROUTE_CLEARANCE))
        .collect()
}

fn automatic_endpoint_side(
    relationship: &diagram_ir::StructuralRelationship,
    endpoint_id: &str,
    diagram: &StructuralDiagram,
    nodes: &[LayoutedStructuralNode],
    groups: &[LayoutedStructuralGroup],
) -> Option<AutomaticSide> {
    if relationship.from == relationship.to {
        return None;
    }
    let from = find_node(nodes, &relationship.from)?;
    let to = find_node(nodes, &relationship.to)?;
    let from_bounds = relationship_endpoint_bounds(
        from,
        relationship.from_group,
        diagram,
        groups,
    );
    let to_bounds = relationship_endpoint_bounds(to, relationship.to_group, diagram, groups);
    let (from_point, to_point) = closest_sides(from_bounds, to_bounds);
    if endpoint_id == relationship.from && relationship.from_port.is_none() {
        Some(automatic_side(from_bounds, &from_point))
    } else if endpoint_id == relationship.to && relationship.to_port.is_none() {
        Some(automatic_side(to_bounds, &to_point))
    } else {
        None
    }
}

fn fanned_automatic_endpoint(
    diagram: &StructuralDiagram,
    relationship_index: usize,
    endpoint_id: &str,
    bounds: StructuralBounds,
    point: Point,
    nodes: &[LayoutedStructuralNode],
    groups: &[LayoutedStructuralGroup],
) -> Point {
    let side = automatic_side(bounds, &point);
    let matching = diagram
        .relationships
        .iter()
        .enumerate()
        .filter_map(|(index, relationship)| {
            (automatic_endpoint_side(relationship, endpoint_id, diagram, nodes, groups)
                == Some(side))
            .then_some(index)
        })
        .collect::<Vec<_>>();
    if matching.len() < 3 {
        return point;
    }
    let slot = matching
        .iter()
        .position(|index| *index == relationship_index)
        .expect("current relationship must be in its endpoint fan-out group");
    fan_out_point(bounds, side, slot, matching.len())
}

fn layout_relationships(
    diagram: &StructuralDiagram,
    nodes: &[LayoutedStructuralNode],
    groups: &[LayoutedStructuralGroup],
) -> Vec<LayoutedStructuralRelationship> {
    diagram
        .relationships
        .iter()
        .enumerate()
        .filter_map(|(relationship_index, rel)| {
            let a = find_node(nodes, &rel.from)?;
            let b = find_node(nodes, &rel.to)?;
            let a_bounds = relationship_endpoint_bounds(a, rel.from_group, diagram, groups);
            let b_bounds = relationship_endpoint_bounds(b, rel.to_group, diagram, groups);
            let (default_p0, default_p1) = closest_sides(a_bounds, b_bounds);
            let route_p0 = rel
                .from_port
                .map_or(default_p0.clone(), |port| point_on_port(a_bounds, port));
            let route_p1 = rel
                .to_port
                .map_or(default_p1.clone(), |port| point_on_port(b_bounds, port));
            let p0 = rel.from_port.map_or_else(
                || {
                    fanned_automatic_endpoint(
                        diagram,
                        relationship_index,
                        &rel.from,
                        a_bounds,
                        default_p0,
                        nodes,
                        groups,
                    )
                },
                |port| point_on_port(a_bounds, port),
            );
            let p1 = rel.to_port.map_or_else(
                || {
                    fanned_automatic_endpoint(
                        diagram,
                        relationship_index,
                        &rel.to,
                        b_bounds,
                        default_p1,
                        nodes,
                        groups,
                    )
                },
                |port| point_on_port(b_bounds, port),
            );
            let obstacles =
                relationship_obstacles(diagram, &rel.from, &rel.to, nodes, groups);
            let points = if rel.from == rel.to && !rel.from_group && !rel.to_group {
                self_relationship_path(a_bounds)
            } else {
                let mut points = relationship_path(
                    route_p0,
                    route_p1,
                    rel.routing,
                    rel.from_port,
                    rel.to_port,
                    &obstacles,
                );
                if let Some(first) = points.first_mut() {
                    *first = p0;
                }
                if let Some(last) = points.last_mut() {
                    *last = p1;
                }
                offset_parallel_relationship(
                    points,
                    parallel_relationship_lane(diagram, relationship_index),
                )
            };
            let label = rel.label.as_ref().map(|label| {
                (path_midpoint(&points), label.clone())
            });
            Some(LayoutedStructuralRelationship {
                from_id: rel.from.clone(),
                to_id: rel.to.clone(),
                kind: rel.kind.clone(),
                start_arrow: rel.start_arrow,
                end_arrow: rel.end_arrow,
                from_group: rel.from_group,
                to_group: rel.to_group,
                from_port: rel.from_port,
                to_port: rel.to_port,
                routing: rel.routing,
                points,
                from_mult: rel.from_mult.clone(),
                to_mult: rel.to_mult.clone(),
                label,
            })
        })
        .collect()
}

fn relationship_endpoint_bounds(
    node: &LayoutedStructuralNode,
    use_group: bool,
    diagram: &StructuralDiagram,
    groups: &[LayoutedStructuralGroup],
) -> StructuralBounds {
    if use_group {
        let parent = diagram
            .nodes
            .iter()
            .find(|candidate| candidate.id == node.id)
            .and_then(|candidate| candidate.parent_group.as_deref());
        if let Some(group) = parent.and_then(|id| groups.iter().find(|group| group.id == id)) {
            return StructuralBounds {
                x: group.x,
                y: group.y,
                width: group.width,
                height: group.height,
            };
        }
    }
    StructuralBounds { x: node.x, y: node.y, width: node.width, height: node.height }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::*;

    fn two_class_diagram() -> StructuralDiagram {
        StructuralDiagram {
            kind: StructuralKind::Class,
            architecture_config: None,
            title: Some("Domain".into()),
            accessibility_title: None,
            accessibility_description: None,
            direction: None,
            alignments: vec![],
            nodes: vec![
                StructuralNode {
                    id: "Animal".into(),
                    label: "Animal".into(),
                    stereotype: None,
                    node_kind: StructuralNodeKind::Abstract,
                    metadata: None,
                    style: None,
                    compartments: vec![Compartment {
                        kind: CompartmentKind::Methods,
                        entries: vec!["speak() void".into()],
                    }],
                    parent_group: None,
                },
                StructuralNode {
                    id: "Dog".into(),
                    label: "Dog".into(),
                    stereotype: None,
                    node_kind: StructuralNodeKind::Class,
                    metadata: None,
                    style: None,
                    compartments: vec![Compartment {
                        kind: CompartmentKind::Fields,
                        entries: vec!["name: String".into()],
                    }],
                    parent_group: None,
                },
            ],
            groups: vec![],
            relationships: vec![StructuralRelationship {
                from: "Dog".into(),
                to: "Animal".into(),
                kind: RelKind::Inheritance,
                start_arrow: false,
                end_arrow: true,
                from_group: false,
                to_group: false,
                from_port: None,
                to_port: None,
                routing: StructuralRouting::Direct,
                from_mult: None,
                to_mult: None,
                label: None,
            }],
        }
    }

    #[test]
    fn version_exists() {
        assert_eq!(crate::VERSION, "0.30.0");
    }

    #[test]
    fn two_nodes_laid_out() {
        let r = layout_structural_diagram(&two_class_diagram());
        assert_eq!(r.nodes.len(), 2);
        assert_eq!(r.relationships.len(), 1);
    }

    #[test]
    fn nodes_have_positive_dimensions() {
        let r = layout_structural_diagram(&two_class_diagram());
        for n in &r.nodes {
            assert!(n.width > 0.0, "width must be positive");
            assert!(n.height > 0.0, "height must be positive");
        }
    }

    #[test]
    fn wide_grid_nodes_do_not_overlap_following_nodes() {
        let mut diagram = two_class_diagram();
        diagram.nodes[0].compartments[0].entries = vec![
            "title: string [PK] - A deliberately wide field comment".into(),
        ];
        let layout = layout_structural_diagram(&diagram);

        assert!(layout.nodes[1].x >= layout.nodes[0].x + layout.nodes[0].width + COL_GAP);
    }

    #[test]
    fn relationship_has_two_points() {
        let r = layout_structural_diagram(&two_class_diagram());
        assert_eq!(r.relationships[0].points.len(), 2);
    }

    #[test]
    fn recursive_relationship_routes_outside_the_node_and_expands_the_canvas() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        diagram.nodes.truncate(1);
        diagram.relationships[0].from = "Animal".into();
        diagram.relationships[0].to = "Animal".into();
        diagram.relationships[0].label = Some("loops".into());
        let layout = layout_structural_diagram(&diagram);
        let node = &layout.nodes[0];
        let points = &layout.relationships[0].points;
        let right = node.x + node.width;
        let bottom = node.y + node.height;

        assert_eq!(points.len(), 5);
        assert!(points.iter().any(|point| point.x > right));
        assert!(points.iter().any(|point| point.y > bottom));
        assert!(layout.width > points.iter().map(|point| point.x).fold(0.0, f64::max));
        assert!(layout.height > points.iter().map(|point| point.y).fold(0.0, f64::max));
        let label_position = &layout.relationships[0].label.as_ref().expect("loop label").0;
        assert!(layout.width > label_position.x + RELATIONSHIP_LABEL_HALF_WIDTH);
    }

    #[test]
    fn parallel_relationships_use_distinct_backend_neutral_lanes() {
        let mut diagram = two_class_diagram();
        let mut second = diagram.relationships[0].clone();
        diagram.relationships[0].label = Some("insured for".into());
        second.label = Some("owned by".into());
        diagram.relationships.push(second);
        let layout = layout_structural_diagram(&diagram);

        assert_eq!(layout.relationships.len(), 2);
        assert_eq!(layout.relationships[0].points.len(), 4);
        assert_eq!(layout.relationships[1].points.len(), 4);
        assert_ne!(
            layout.relationships[0].points,
            layout.relationships[1].points
        );
        assert_ne!(
            layout.relationships[0].label.as_ref().map(|label| &label.0),
            layout.relationships[1].label.as_ref().map(|label| &label.0)
        );
    }

    #[test]
    fn direct_cycle_relationship_routes_around_intervening_nodes() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        diagram.nodes[0].id = "A".into();
        diagram.nodes[0].label = "A".into();
        diagram.nodes[1].id = "B".into();
        diagram.nodes[1].label = "B".into();
        let mut node_c = diagram.nodes[1].clone();
        node_c.id = "C".into();
        node_c.label = "C".into();
        diagram.nodes.push(node_c);
        let mut relationship = diagram.relationships[0].clone();
        relationship.from = "A".into();
        relationship.to = "B".into();
        let mut second = relationship.clone();
        second.from = "B".into();
        second.to = "C".into();
        let mut back = relationship.clone();
        back.from = "C".into();
        back.to = "A".into();
        diagram.relationships = vec![relationship, second, back];

        let layout = layout_structural_diagram(&diagram);
        let middle = layout.nodes.iter().find(|node| node.id == "B").expect("B");
        let back = layout
            .relationships
            .iter()
            .find(|relationship| relationship.from_id == "C" && relationship.to_id == "A")
            .expect("C to A relationship");

        assert!(back.points.len() >= 5);
        assert!(back.points.iter().any(|point| point.y > middle.y + middle.height));
    }

    #[test]
    fn automatic_relationships_fan_out_across_a_shared_node_side() {
        let mut diagram = two_class_diagram();
        diagram.relationships.push(diagram.relationships[0].clone());
        diagram.relationships.push(diagram.relationships[0].clone());

        let layout = layout_structural_diagram(&diagram);
        let endpoints = layout
            .relationships
            .iter()
            .map(|relationship| {
                relationship
                    .points
                    .last()
                    .expect("relationship endpoint")
                    .y
            })
            .collect::<Vec<_>>();

        assert_ne!(endpoints[0], endpoints[1]);
        assert_ne!(endpoints[1], endpoints[2]);
    }

    #[test]
    fn long_entity_and_attribute_names_reserve_distinct_node_widths() {
        let mut diagram = two_class_diagram();
        diagram.nodes[0].label = "PRIVATE_FINANCIAL_INSTITUTION".into();
        diagram.nodes[0].compartments[0].entries =
            vec!["officer_of_firm: bool".into()];

        let layout = layout_structural_diagram(&diagram);

        assert!(layout.nodes[0].width > layout.nodes[1].width);
        assert!(layout.nodes[1].x >= layout.nodes[0].x + layout.nodes[0].width + COL_GAP);
    }

    #[test]
    fn mixed_special_topology_keeps_isolated_and_connected_nodes_distinct() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        let template = diagram.nodes[0].clone();
        diagram.nodes = ["CUSTOMER", "1", "u", "1.5", "2.5"]
            .into_iter()
            .map(|id| StructuralNode { id: id.into(), label: id.into(), ..template.clone() })
            .collect();
        let template = diagram.relationships[0].clone();
        diagram.relationships = [("CUSTOMER", "1"), ("1", "u"), ("u", "2.5")]
            .into_iter()
            .map(|(from, to)| StructuralRelationship {
                from: from.into(),
                to: to.into(),
                ..template.clone()
            })
            .collect();

        let layout = layout_structural_diagram(&diagram);

        assert_eq!(layout.nodes.len(), 5);
        assert_eq!(layout.relationships.len(), 3);
        for (index, left) in layout.nodes.iter().enumerate() {
            for right in &layout.nodes[index + 1..] {
                let separated = left.x + left.width <= right.x
                    || right.x + right.width <= left.x
                    || left.y + left.height <= right.y
                    || right.y + right.height <= left.y;
                assert!(separated, "{} overlaps {}", left.id, right.id);
            }
        }
    }

    #[test]
    fn entity_alias_keyword_substrings_reserve_label_geometry() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        diagram.nodes.truncate(1);
        diagram.nodes[0].id = "p".into();
        diagram.nodes[0].label = "Photograph".into();
        diagram.nodes[0].compartments[0].entries =
            vec!["jobId: varchar(12)".into(), "dateCreated: date".into()];
        diagram.relationships.clear();

        let layout = layout_structural_diagram(&diagram);

        assert_eq!(layout.nodes.len(), 1);
        assert_eq!(layout.nodes[0].header, "Photograph");
        assert!(layout.nodes[0].width >= 168.0);
        assert!(layout.nodes[0].height > 0.0);
    }

    #[test]
    fn disconnected_nodes_and_recursive_relationship_share_layout_safely() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        let template = diagram.nodes[0].clone();
        diagram.nodes = ["DEAD_PARROT", "HERMIT", "RECLUSE", "SOCIALITE"]
            .into_iter()
            .map(|id| StructuralNode { id: id.into(), label: id.into(), ..template.clone() })
            .collect();
        let template = diagram.relationships[0].clone();
        diagram.relationships = vec![
            StructuralRelationship {
                from: "SOCIALITE".into(),
                to: "SOCIALITE".into(),
                label: Some("interacts with".into()),
                ..template.clone()
            },
            StructuralRelationship {
                from: "RECLUSE".into(),
                to: "SOCIALITE".into(),
                label: Some("avoids".into()),
                ..template
            },
        ];

        let layout = layout_structural_diagram(&diagram);
        let socialite = layout.nodes.iter().find(|node| node.id == "SOCIALITE").expect("SOCIALITE");
        let recursive = layout
            .relationships
            .iter()
            .find(|relationship| relationship.from_id == "SOCIALITE" && relationship.to_id == "SOCIALITE")
            .expect("recursive relationship");

        assert_eq!(layout.nodes.len(), 4);
        assert_eq!(recursive.points.len(), 5);
        assert!(recursive.points.iter().any(|point| point.x > socialite.x + socialite.width));
        assert!(recursive.points.iter().any(|point| point.y > socialite.y + socialite.height));
    }

    #[test]
    fn constrained_attribute_rows_reserve_backend_neutral_geometry() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        diagram.nodes.truncate(1);
        diagram.nodes[0].id = "CUSTOMER".into();
        diagram.nodes[0].label = "CUSTOMER".into();
        diagram.nodes[0].compartments[0].entries = vec![
            "customer_number: int [PK, FK] - comment1".into(),
            "customer_status_start_datetime: datetime [PK, UK, FK]".into(),
            "customer_status_end_datetime: datetime [PK, UK] - comment3".into(),
            "customer_firstname: string".into(),
            "customer_lastname: string - comment5".into(),
        ];
        diagram.relationships.clear();

        let layout = layout_structural_diagram(&diagram);

        assert_eq!(layout.nodes[0].compartments[0].rows.len(), 5);
        assert!(layout.nodes[0].width > 400.0);
        assert!(layout.nodes[0].height > 100.0);
    }

    #[test]
    fn key_prefix_attribute_names_remain_visible_rows() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        diagram.nodes.truncate(1);
        diagram.nodes[0].id = "BOOK".into();
        diagram.nodes[0].label = "BOOK".into();
        diagram.nodes[0].compartments[0].entries = vec![
            "fk_title: int [FK]".into(),
            "pk_author: string [PK]".into(),
            "uk_address: string [UK]".into(),
            "pk_price: float [PK] - comment".into(),
        ];
        diagram.relationships.clear();

        let layout = layout_structural_diagram(&diagram);

        assert_eq!(layout.nodes[0].compartments[0].rows.len(), 4);
        assert!(layout.nodes[0].width > 240.0);
        assert!(layout.nodes[0].height > 80.0);
    }

    #[test]
    fn embedded_numeric_attribute_names_reserve_rows() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        diagram.nodes.truncate(1);
        diagram.nodes[0].id = "ENTITY".into();
        diagram.nodes[0].label = "ENTITY".into();
        diagram.nodes[0].compartments[0].entries =
            vec!["name1: string".into(), "value2: int".into()];
        diagram.relationships.clear();

        let layout = layout_structural_diagram(&diagram);

        assert_eq!(layout.nodes[0].compartments[0].rows.len(), 2);
        assert!(layout.nodes[0].height > 60.0);
    }

    #[test]
    fn key_prefix_attribute_types_reserve_rows() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        diagram.nodes.truncate(1);
        diagram.nodes[0].id = "ENTITY".into();
        diagram.nodes[0].label = "ENTITY".into();
        diagram.nodes[0].compartments[0].entries = vec![
            "primary: PKType".into(),
            "foreign: fk_type".into(),
            "alias: string? [UK]".into(),
        ];
        diagram.relationships.clear();

        let layout = layout_structural_diagram(&diagram);

        assert_eq!(layout.nodes[0].compartments[0].rows.len(), 3);
        assert!(layout.nodes[0].height > 70.0);
    }

    #[test]
    fn multiline_attribute_comments_reserve_each_semantic_line() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        diagram.nodes.truncate(1);
        diagram.nodes[0].id = "ENTITY".into();
        diagram.nodes[0].label = "ENTITY".into();
        diagram.nodes[0].compartments[0].entries = vec![
            "first: string - Primary\nidentifier".into(),
            "second: string - Second\nline".into(),
            "third: string [PK] - Third\nline".into(),
        ];
        diagram.relationships.clear();

        let layout = layout_structural_diagram(&diagram);

        assert_eq!(layout.nodes[0].compartments[0].rows.len(), 3);
        assert!(layout.nodes[0].height > 130.0);
        assert!(layout.nodes[0].width < 300.0);
    }

    #[test]
    fn multiline_entity_aliases_reserve_header_geometry() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        diagram.nodes.truncate(1);
        diagram.nodes[0].id = "ENTITY".into();
        diagram.nodes[0].label = "Primary\nEntity".into();
        diagram.nodes[0].compartments[0].entries = vec!["id: string [PK]".into()];
        diagram.relationships.clear();

        let layout = layout_structural_diagram(&diagram);
        let node = &layout.nodes[0];

        assert!(node.compartments[0].y_offset > 50.0);
        assert!(node.height > 90.0);
        assert_eq!(node.header, "Primary\nEntity");
    }

    #[test]
    fn multiline_titles_reserve_vertical_geometry() {
        let mut diagram = two_class_diagram();
        diagram.nodes.truncate(1);
        diagram.relationships.clear();
        diagram.title = Some("Domain\nModel\nAcross\nLines".into());

        let layout = layout_structural_diagram(&diagram);

        assert_eq!(layout.title.as_deref(), Some("Domain\nModel\nAcross\nLines"));
        assert!(layout.nodes[0].y > 120.0);
        assert!(layout.height > 190.0);
    }

    #[test]
    fn unicode_symbol_er_text_reserves_layout_space() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        diagram.nodes[0].id = "_**testẽζ➕Ø😀㌕ぼ**_".into();
        diagram.nodes[0].label = "_**testẽζ➕Ø😀㌕ぼ**_".into();
        diagram.nodes[0].compartments[0].entries = vec!["_age_: int".into()];
        diagram.nodes[1].id = "person".into();
        diagram.nodes[1].label = "😀".into();
        diagram.nodes[1].compartments[0].entries =
            vec!["*first*Name: string".into(), "_**last**Name_: string".into()];
        diagram.relationships[0].from = "person".into();
        diagram.relationships[0].to = "_**testẽζ➕Ø😀㌕ぼ**_".into();
        diagram.relationships[0].label = Some("__hẽ😀__".into());

        let layout = layout_structural_diagram(&diagram);

        assert_eq!(layout.nodes[1].compartments[0].rows.len(), 2);
        assert!(layout.nodes[0].width > 180.0);
        assert_eq!(
            layout.relationships[0]
                .label
                .as_ref()
                .map(|(_, label)| label.as_str()),
            Some("__hẽ😀__")
        );
    }

    #[test]
    fn numeric_entity_boundaries_layout_distinct_targets() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        diagram.nodes[0].id = "a".into();
        diagram.nodes[0].label = "a".into();
        diagram.nodes[0].compartments.clear();
        diagram.nodes[1].id = "1".into();
        diagram.nodes[1].label = "1".into();
        diagram.nodes[1].compartments.clear();
        let mut twelve = diagram.nodes[1].clone();
        twelve.id = "12".into();
        twelve.label = "12".into();
        diagram.nodes.push(twelve);
        diagram.relationships[0].from = "a".into();
        diagram.relationships[0].to = "1".into();
        diagram.relationships[0].from_mult = Some("0..*".into());
        diagram.relationships[0].to_mult = Some("1".into());
        diagram.relationships[0].label = Some("label".into());
        let mut second = diagram.relationships[0].clone();
        second.to = "12".into();
        diagram.relationships.push(second);

        let layout = layout_structural_diagram(&diagram);

        assert_eq!(layout.nodes.len(), 3);
        assert_eq!(layout.relationships.len(), 2);
        assert!(layout.nodes.iter().any(|node| node.id == "1"));
        assert!(layout.nodes.iter().any(|node| node.id == "12"));
    }

    #[test]
    fn relationship_labels_resolve_to_backend_neutral_geometry() {
        let mut diagram = two_class_diagram();
        diagram.relationships[0].label = Some("inherits from".into());
        let layout = layout_structural_diagram(&diagram);
        let (position, label) = layout.relationships[0]
            .label
            .as_ref()
            .expect("relationship label geometry");
        assert_eq!(label, "inherits from");
        assert!(position.x.is_finite());
        assert!(position.y.is_finite());
    }

    #[test]
    fn title_reserves_vertical_geometry() {
        let mut diagram = two_class_diagram();
        diagram.title = None;
        let without_title = layout_structural_diagram(&diagram);
        diagram.title = Some("Domain model".into());
        let with_title = layout_structural_diagram(&diagram);
        assert_eq!(with_title.title.as_deref(), Some("Domain model"));
        assert_eq!(with_title.nodes[0].y, without_title.nodes[0].y + TITLE_H);
        assert!(with_title.height > without_title.height);
    }

    #[test]
    fn canvas_size_positive() {
        let r = layout_structural_diagram(&two_class_diagram());
        assert!(r.width > 0.0);
        assert!(r.height > 0.0);
    }

    #[test]
    fn explicit_direction_controls_primary_axis_and_order() {
        let mut diagram = two_class_diagram();

        diagram.direction = Some(DiagramDirection::Tb);
        let layout = layout_structural_diagram(&diagram);
        assert!(layout.nodes[1].y > layout.nodes[0].y);

        diagram.direction = Some(DiagramDirection::Bt);
        let layout = layout_structural_diagram(&diagram);
        assert!(layout.nodes[0].y > layout.nodes[1].y);

        diagram.direction = Some(DiagramDirection::Lr);
        let layout = layout_structural_diagram(&diagram);
        assert!(layout.nodes[1].x > layout.nodes[0].x);

        diagram.direction = Some(DiagramDirection::Rl);
        let layout = layout_structural_diagram(&diagram);
        assert!(layout.nodes[0].x > layout.nodes[1].x);
    }

    #[test]
    fn compartments_have_correct_rows() {
        let r = layout_structural_diagram(&two_class_diagram());
        let animal = r.nodes.iter().find(|n| n.id == "Animal").unwrap();
        assert_eq!(animal.compartments[0].rows, vec!["speak() void"]);
    }

    #[test]
    fn structural_group_bounds_contain_member_nodes() {
        let mut diagram = two_class_diagram();
        diagram.nodes[0].parent_group = Some("domain".into());
        diagram.groups.push(StructuralGroup {
            id: "domain".into(),
            label: "Domain".into(),
            stereotype: Some("System_Boundary".into()),
            metadata: None,
            parent_group: None,
        });

        let layout = layout_structural_diagram(&diagram);
        let group = layout
            .groups
            .iter()
            .find(|group| group.id == "domain")
            .unwrap();
        let animal = layout
            .nodes
            .iter()
            .find(|node| node.id == "Animal")
            .unwrap();
        assert!(group.x <= animal.x);
        assert!(group.y <= animal.y);
        assert!(group.x + group.width >= animal.x + animal.width);
        assert!(group.y + group.height >= animal.y + animal.height);
    }

    #[test]
    fn group_edge_endpoints_resolve_to_group_bounds() {
        let mut diagram = two_class_diagram();
        diagram.nodes[0].parent_group = Some("domain".into());
        diagram.groups.push(StructuralGroup {
            id: "domain".into(),
            label: "Domain".into(),
            stereotype: None,
            metadata: None,
            parent_group: None,
        });
        diagram.relationships[0].to_group = true;

        let layout = layout_structural_diagram(&diagram);
        let group = &layout.groups[0];
        let relationship = &layout.relationships[0];
        let endpoint = &relationship.points[1];
        assert!(relationship.to_group);
        assert!(
            endpoint.x == group.x
                || endpoint.x == group.x + group.width
                || endpoint.y == group.y
                || endpoint.y == group.y + group.height
        );
    }

    #[test]
    fn explicit_ports_resolve_to_orthogonal_boundary_routes() {
        let mut diagram = two_class_diagram();
        diagram.relationships[0].from_port = Some(StructuralPort::Top);
        diagram.relationships[0].to_port = Some(StructuralPort::Bottom);
        diagram.relationships[0].routing = StructuralRouting::Orthogonal;

        let layout = layout_structural_diagram(&diagram);
        let dog = layout.nodes.iter().find(|node| node.id == "Dog").unwrap();
        let animal = layout.nodes.iter().find(|node| node.id == "Animal").unwrap();
        let relationship = &layout.relationships[0];
        assert_eq!(relationship.from_port, Some(StructuralPort::Top));
        assert_eq!(relationship.to_port, Some(StructuralPort::Bottom));
        assert_eq!(relationship.routing, StructuralRouting::Orthogonal);
        assert_eq!(
            relationship.points[0],
            Point {
                x: dog.x + dog.width / 2.0,
                y: dog.y,
            }
        );
        assert_eq!(
            *relationship.points.last().unwrap(),
            Point {
                x: animal.x + animal.width / 2.0,
                y: animal.y + animal.height,
            }
        );
        assert!(relationship.points.len() >= 3);
        assert!(relationship.points.windows(2).all(|segment| {
            segment[0].x == segment[1].x || segment[0].y == segment[1].y
        }));
    }

    #[test]
    fn orthogonal_routes_avoid_unrelated_node_bounds() {
        let obstacle = StructuralBounds {
            x: 80.0,
            y: 40.0,
            width: 40.0,
            height: 40.0,
        }
        .expanded(ROUTE_CLEARANCE);
        let points = obstacle_avoiding_orthogonal_path(
            Point { x: 0.0, y: 60.0 },
            Point { x: 200.0, y: 60.0 },
            StructuralPort::Right,
            StructuralPort::Left,
            &[obstacle],
        );

        assert!(points.len() >= 4);
        assert!(points.windows(2).all(|segment| {
            segment[0].x == segment[1].x || segment[0].y == segment[1].y
        }));
        assert!(path_is_clear(&points, &[obstacle]));
        assert_eq!(points[1], Point { x: ROUTE_CLEARANCE, y: 60.0 });
        assert_eq!(
            points[points.len() - 2],
            Point {
                x: 200.0 - ROUTE_CLEARANCE,
                y: 60.0,
            }
        );
    }

    #[test]
    fn relationship_obstacles_include_unrelated_group_bounds() {
        let mut diagram = two_class_diagram();
        diagram.groups.push(StructuralGroup {
            id: "cache-group".into(),
            label: "Cache".into(),
            stereotype: None,
            metadata: None,
            parent_group: None,
        });
        diagram.nodes.push(StructuralNode {
            id: "Cache".into(),
            label: "Cache".into(),
            stereotype: None,
            node_kind: StructuralNodeKind::Class,
            metadata: None,
            style: None,
            compartments: vec![],
            parent_group: Some("cache-group".into()),
        });

        let layout = layout_structural_diagram(&diagram);
        let obstacles = relationship_obstacles(
            &diagram,
            "Dog",
            "Animal",
            &layout.nodes,
            &layout.groups,
        );

        assert_eq!(obstacles.len(), 2);
        let group = &layout.groups[0];
        assert!(obstacles.iter().any(|obstacle| {
            obstacle.x == group.x - ROUTE_CLEARANCE
                && obstacle.width == group.width + ROUTE_CLEARANCE * 2.0
        }));
    }

    #[test]
    fn obstacle_free_routes_keep_the_simple_orthogonal_path() {
        let from = Point { x: 0.0, y: 20.0 };
        let to = Point { x: 200.0, y: 80.0 };
        assert_eq!(
            obstacle_avoiding_orthogonal_path(
                from.clone(),
                to.clone(),
                StructuralPort::Right,
                StructuralPort::Left,
                &[],
            ),
            orthogonal_path(from, to, StructuralPort::Right, StructuralPort::Left)
        );
    }

    #[test]
    fn junctions_resolve_to_compact_typed_geometry() {
        let mut diagram = two_class_diagram();
        diagram.nodes[0].node_kind = StructuralNodeKind::Junction;
        diagram.nodes[0].label.clear();
        diagram.nodes[0].compartments.clear();
        let layout = layout_structural_diagram(&diagram);
        let junction = &layout.nodes[0];
        assert_eq!(junction.node_kind, StructuralNodeKind::Junction);
        assert_eq!((junction.width, junction.height), (18.0, 18.0));
        assert_eq!(
            layout.relationships[0].points[1].x,
            junction.x + junction.width
        );
    }

    #[test]
    fn architecture_icon_text_reserves_header_geometry() {
        let mut diagram = two_class_diagram();
        diagram.kind = StructuralKind::Architecture;
        diagram.nodes[0].metadata = Some(StructuralNodeMetadata::ArchitectureService(
            ArchitectureServiceMetadata {
                icon_name: None,
                icon_text: Some("API".into()),
            },
        ));
        let layout = layout_structural_diagram(&diagram);
        assert_eq!(layout.nodes[0].icon_text.as_deref(), Some("API"));
        assert!(layout.nodes[0].height >= 72.0);
        assert!(layout.nodes[0].height > layout.nodes[1].height);
    }

    #[test]
    fn architecture_named_icon_reserves_header_geometry() {
        let mut diagram = two_class_diagram();
        diagram.kind = StructuralKind::Architecture;
        diagram.nodes[0].metadata = Some(StructuralNodeMetadata::ArchitectureService(
            ArchitectureServiceMetadata {
                icon_name: Some("database".into()),
                icon_text: None,
            },
        ));
        let layout = layout_structural_diagram(&diagram);
        assert_eq!(layout.nodes[0].icon_name.as_deref(), Some("database"));
        assert!(layout.nodes[0].height >= 72.0);
        assert!(layout.nodes[0].height > layout.nodes[1].height);
    }

    #[test]
    fn architecture_config_scales_icon_geometry_and_typography() {
        let mut diagram = two_class_diagram();
        diagram.kind = StructuralKind::Architecture;
        diagram.architecture_config = Some(ArchitectureConfig {
            icon_size: 120.0,
            font_size: 22.0,
            node_separation: 110.0,
            padding: 48.0,
            ideal_edge_length_multiplier: 1.25,
            edge_elasticity: 0.45,
            num_iter: 2500,
            randomize: false,
            seed: 1,
        });
        diagram.nodes[0].metadata = Some(StructuralNodeMetadata::ArchitectureService(
            ArchitectureServiceMetadata {
                icon_name: Some("server".into()),
                icon_text: None,
            },
        ));
        diagram.nodes[0].parent_group = Some("platform".into());
        diagram.groups.push(StructuralGroup {
            id: "platform".into(),
            label: "Platform".into(),
            stereotype: None,
            metadata: None,
            parent_group: None,
        });
        diagram.alignments.push(StructuralAlignment {
            axis: StructuralAlignmentAxis::Row,
            members: vec!["Animal".into(), "Dog".into()],
        });

        let layout = layout_structural_diagram(&diagram);
        assert_eq!(layout.nodes[0].style.font_size, 22.0);
        assert!(layout.nodes[0].width >= 240.0);
        assert!(layout.nodes[0].height >= 108.0);
        assert_eq!(layout.nodes[0].x, 48.0);
        assert_eq!(layout.groups[0].x, layout.nodes[0].x - 48.0);
        assert_eq!(
            layout.groups[0].x + layout.groups[0].width
                - (layout.nodes[0].x + layout.nodes[0].width),
            48.0
        );
        assert_eq!(
            layout.nodes[1].x - (layout.nodes[0].x + layout.nodes[0].width),
            150.0
        );
        assert_eq!(layout.width - (layout.nodes[1].x + layout.nodes[1].width), 48.0);
    }

    #[test]
    fn alignments_resolve_declared_rows_and_columns() {
        let mut diagram = two_class_diagram();
        diagram.nodes.push(StructuralNode {
            id: "Cat".into(),
            label: "Cat".into(),
            stereotype: None,
            node_kind: StructuralNodeKind::Class,
            metadata: None,
            style: None,
            compartments: vec![],
            parent_group: None,
        });
        diagram.alignments.push(StructuralAlignment {
            axis: StructuralAlignmentAxis::Column,
            members: vec!["Animal".into(), "Dog".into(), "Cat".into()],
        });
        let layout = layout_structural_diagram(&diagram);
        assert!(layout.nodes.windows(2).all(|nodes| nodes[0].x == nodes[1].x));
        assert!(layout.nodes.windows(2).all(|nodes| nodes[0].y < nodes[1].y));

        diagram.alignments[0].axis = StructuralAlignmentAxis::Row;
        let layout = layout_structural_diagram(&diagram);
        assert!(layout.nodes.windows(2).all(|nodes| nodes[0].y == nodes[1].y));
        assert!(layout.nodes.windows(2).all(|nodes| nodes[0].x < nodes[1].x));
    }

    #[test]
    fn architecture_edge_elasticity_controls_connected_sibling_spacing() {
        let mut loose = two_class_diagram();
        loose.kind = StructuralKind::Architecture;
        loose.architecture_config = Some(ArchitectureConfig {
            edge_elasticity: 0.1,
            ..ArchitectureConfig::default()
        });
        let mut tight = loose.clone();
        tight.architecture_config.as_mut().unwrap().edge_elasticity = 0.9;

        let loose_layout = layout_structural_diagram(&loose);
        let tight_layout = layout_structural_diagram(&tight);
        let loose_gap = loose_layout.nodes[1].x
            - (loose_layout.nodes[0].x + loose_layout.nodes[0].width);
        let tight_gap = tight_layout.nodes[1].x
            - (tight_layout.nodes[0].x + tight_layout.nodes[0].width);
        assert!(tight_gap < loose_gap);
        assert_eq!(loose_gap, 101.25);
        assert_eq!(tight_gap, 41.25);
    }

    #[test]
    fn architecture_randomize_uses_repeatable_seeded_sibling_placement() {
        let mut diagram = two_class_diagram();
        diagram.kind = StructuralKind::Architecture;
        for id in ["Cat", "Bird", "Fish", "Horse"] {
            let mut node = diagram.nodes[0].clone();
            node.id = id.into();
            node.label = id.into();
            diagram.nodes.push(node);
        }
        diagram.architecture_config = Some(ArchitectureConfig {
            randomize: true,
            seed: 7,
            ..ArchitectureConfig::default()
        });

        let first = layout_structural_diagram(&diagram);
        let repeated = layout_structural_diagram(&diagram);
        let first_positions = first
            .nodes
            .iter()
            .map(|node| (node.x, node.y))
            .collect::<Vec<_>>();
        let repeated_positions = repeated
            .nodes
            .iter()
            .map(|node| (node.x, node.y))
            .collect::<Vec<_>>();
        assert_eq!(first_positions, repeated_positions);

        diagram.architecture_config.as_mut().unwrap().seed = 11;
        let alternate = layout_structural_diagram(&diagram);
        let alternate_positions = alternate
            .nodes
            .iter()
            .map(|node| (node.x, node.y))
            .collect::<Vec<_>>();
        assert_ne!(first_positions, alternate_positions);
    }
}
