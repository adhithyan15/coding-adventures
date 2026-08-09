//! # diagram-layout-structural
//!
//! Layout engine for structural diagrams (DG04): class, ER, and C4.
//!
//! Takes a `StructuralDiagram` (semantic IR) and produces a
//! `LayoutedStructuralDiagram` with bounding boxes for all nodes and
//! polyline paths for all relationships.

use diagram_ir::{
    LayoutedCompartment, LayoutedStructuralDiagram, LayoutedStructuralGroup,
    LayoutedStructuralNode, LayoutedStructuralRelationship, Point, StructuralDiagram, StructuralNode,
};
use std::collections::{HashMap, HashSet};

pub const VERSION: &str = "0.2.0";

const MIN_NODE_W: f64 = 160.0;
const HEADER_H:   f64 = 40.0;
const ROW_H:      f64 = 20.0;
const COMP_PAD:   f64 = 8.0;
const COL_GAP:    f64 = 80.0;
const ROW_GAP:    f64 = 60.0;
const COLS:       usize = 3;
const GROUP_PAD:  f64 = 24.0;
const GROUP_HEADER_H: f64 = 32.0;

/// Lay out a `StructuralDiagram` using a simple grid arrangement.
pub fn layout_structural_diagram(diagram: &StructuralDiagram) -> LayoutedStructuralDiagram {
    let nodes = layout_nodes(&diagram.nodes, &diagram.groups);
    let groups = layout_groups(diagram, &nodes);
    let canvas_w = canvas_width(&nodes, &groups);
    let canvas_h = canvas_height(&nodes, &groups);
    let rels = layout_relationships(diagram, &nodes);
    LayoutedStructuralDiagram {
        width: canvas_w, height: canvas_h, groups, nodes, relationships: rels,
    }
}

fn node_width(node: &StructuralNode) -> f64 {
    let max_entry = node.compartments.iter()
        .flat_map(|c| c.entries.iter())
        .map(|e| e.len())
        .max()
        .unwrap_or(0);
    // Approximate 8 px/char + padding; header is the label
    let text_w = (node.label.len().max(max_entry) as f64 * 8.0 + 24.0).ceil();
    text_w.max(MIN_NODE_W)
}

fn node_height(node: &StructuralNode) -> f64 {
    let mut h = HEADER_H;
    for comp in &node.compartments {
        h += COMP_PAD + comp.entries.len() as f64 * ROW_H + COMP_PAD;
    }
    h
}

fn layout_nodes(
    nodes: &[StructuralNode],
    groups: &[diagram_ir::StructuralGroup],
) -> Vec<LayoutedStructuralNode> {
    let mut out: Vec<LayoutedStructuralNode> = Vec::with_capacity(nodes.len());
    // Track max height per row so rows don't overlap.
    let mut row_y: Vec<f64> = vec![COMP_PAD];

    for (idx, node) in nodes.iter().enumerate() {
        let col   = idx % COLS;
        let row   = idx / COLS;
        let nw    = node_width(node);
        let nh    = node_height(node);

        // Ensure row_y has an entry for this row.
        while row_y.len() <= row { row_y.push(*row_y.last().unwrap_or(&COMP_PAD)); }

        let x = COMP_PAD + col as f64 * (MIN_NODE_W + COL_GAP);
        let y = row_y[row]
            + group_depth(node.parent_group.as_deref(), groups) as f64 * GROUP_HEADER_H;

        // Update the starting y for the next row.
        let next_row_y = y + nh + ROW_GAP;
        if row + 1 >= row_y.len() { row_y.push(next_row_y); }
        else if row_y[row + 1] < next_row_y { row_y[row + 1] = next_row_y; }

        // Build layouted compartments.
        let mut y_off = HEADER_H;
        let mut comps: Vec<LayoutedCompartment> = Vec::new();
        for comp in &node.compartments {
            let ch = COMP_PAD + comp.entries.len() as f64 * ROW_H + COMP_PAD;
            comps.push(LayoutedCompartment {
                y_offset: y_off,
                height:   ch,
                rows:     comp.entries.clone(),
            });
            y_off += ch;
        }

        out.push(LayoutedStructuralNode {
            id: node.id.clone(),
            x, y,
            width:  nw,
            height: nh,
            header: node.label.clone(),
            stereotype: node.stereotype.clone(),
            compartments: comps,
        });
    }
    out
}

fn group_depth(
    parent_group: Option<&str>,
    groups: &[diagram_ir::StructuralGroup],
) -> usize {
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

fn canvas_width(nodes: &[LayoutedStructuralNode], groups: &[LayoutedStructuralGroup]) -> f64 {
    nodes.iter()
        .map(|n| n.x + n.width + COMP_PAD)
        .chain(groups.iter().map(|g| g.x + g.width + COMP_PAD))
        .fold(200.0_f64, f64::max)
}

fn canvas_height(nodes: &[LayoutedStructuralNode], groups: &[LayoutedStructuralGroup]) -> f64 {
    nodes.iter()
        .map(|n| n.y + n.height + COMP_PAD)
        .chain(groups.iter().map(|g| g.y + g.height + COMP_PAD))
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

    let min_x = children.iter().map(|bounds| bounds.0).fold(f64::INFINITY, f64::min);
    let min_y = children.iter().map(|bounds| bounds.1).fold(f64::INFINITY, f64::min);
    let max_x = children.iter().map(|bounds| bounds.2).fold(f64::NEG_INFINITY, f64::max);
    let max_y = children.iter().map(|bounds| bounds.3).fold(f64::NEG_INFINITY, f64::max);
    let bounds = (
        (min_x - GROUP_PAD).max(0.0),
        (min_y - GROUP_HEADER_H).max(0.0),
        max_x + GROUP_PAD,
        max_y + GROUP_PAD,
    );
    cache.insert(group_id.to_string(), bounds);
    Some(bounds)
}

fn find_node<'a>(nodes: &'a [LayoutedStructuralNode], id: &str) -> Option<&'a LayoutedStructuralNode> {
    nodes.iter().find(|n| n.id == id)
}

fn closest_sides(a: &LayoutedStructuralNode, b: &LayoutedStructuralNode) -> (Point, Point) {
    let a_cx = a.x + a.width  / 2.0;
    let a_cy = a.y + a.height / 2.0;
    let b_cx = b.x + b.width  / 2.0;
    let b_cy = b.y + b.height / 2.0;
    // Use dominant axis to pick left/right vs top/bottom connector sides.
    let dx = (b_cx - a_cx).abs();
    let dy = (b_cy - a_cy).abs();
    if dx >= dy {
        // Horizontal dominant — connect right edge of A to left edge of B (or vice versa).
        if b_cx >= a_cx {
            (Point { x: a.x + a.width, y: a_cy }, Point { x: b.x, y: b_cy })
        } else {
            (Point { x: a.x, y: a_cy }, Point { x: b.x + b.width, y: b_cy })
        }
    } else {
        // Vertical dominant — connect bottom edge of A to top edge of B (or vice versa).
        if b_cy >= a_cy {
            (Point { x: a_cx, y: a.y + a.height }, Point { x: b_cx, y: b.y })
        } else {
            (Point { x: a_cx, y: a.y }, Point { x: b_cx, y: b.y + b.height })
        }
    }
}

fn layout_relationships(
    diagram: &StructuralDiagram,
    nodes: &[LayoutedStructuralNode],
) -> Vec<LayoutedStructuralRelationship> {
    diagram.relationships.iter().filter_map(|rel| {
        let a = find_node(nodes, &rel.from)?;
        let b = find_node(nodes, &rel.to)?;
        let (p0, p1) = closest_sides(a, b);
        Some(LayoutedStructuralRelationship {
            from_id: rel.from.clone(),
            to_id:   rel.to.clone(),
            kind:    rel.kind.clone(),
            points:  vec![p0, p1],
            from_mult: rel.from_mult.clone(),
            to_mult:   rel.to_mult.clone(),
            label:   rel.label.as_ref().map(|l| {
                let a = find_node(nodes, &rel.from).unwrap();
                let b = find_node(nodes, &rel.to).unwrap();
                let mx = (a.x + a.width / 2.0 + b.x + b.width / 2.0) / 2.0;
                let my = (a.y + a.height / 2.0 + b.y + b.height / 2.0) / 2.0;
                (Point { x: mx, y: my }, l.clone())
            }),
        })
    }).collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::*;

    fn two_class_diagram() -> StructuralDiagram {
        StructuralDiagram {
            kind: StructuralKind::Class,
            title: Some("Domain".into()),
            nodes: vec![
                StructuralNode {
                    id: "Animal".into(), label: "Animal".into(),
                    stereotype: None,
                    node_kind: StructuralNodeKind::Abstract,
                    compartments: vec![Compartment {
                        kind: CompartmentKind::Methods,
                        entries: vec!["speak() void".into()],
                    }],
                    parent_group: None,
                },
                StructuralNode {
                    id: "Dog".into(), label: "Dog".into(),
                    stereotype: None,
                    node_kind: StructuralNodeKind::Class,
                    compartments: vec![Compartment {
                        kind: CompartmentKind::Fields,
                        entries: vec!["name: String".into()],
                    }],
                    parent_group: None,
                },
            ],
            groups: vec![],
            relationships: vec![StructuralRelationship {
                from: "Dog".into(), to: "Animal".into(),
                kind: RelKind::Inheritance,
                from_mult: None, to_mult: None, label: None,
            }],
        }
    }

    #[test] fn version_exists() { assert_eq!(crate::VERSION, "0.2.0"); }

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
    fn relationship_has_two_points() {
        let r = layout_structural_diagram(&two_class_diagram());
        assert_eq!(r.relationships[0].points.len(), 2);
    }

    #[test]
    fn canvas_size_positive() {
        let r = layout_structural_diagram(&two_class_diagram());
        assert!(r.width > 0.0);
        assert!(r.height > 0.0);
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
            parent_group: None,
        });

        let layout = layout_structural_diagram(&diagram);
        let group = layout.groups.iter().find(|group| group.id == "domain").unwrap();
        let animal = layout.nodes.iter().find(|node| node.id == "Animal").unwrap();
        assert!(group.x <= animal.x);
        assert!(group.y <= animal.y);
        assert!(group.x + group.width >= animal.x + animal.width);
        assert!(group.y + group.height >= animal.y + animal.height);
    }
}
