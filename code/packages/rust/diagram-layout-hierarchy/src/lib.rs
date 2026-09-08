//! Deterministic, backend-neutral hierarchy layout.

use std::collections::HashMap;

use diagram_ir::{
    LayoutedTreeViewDiagram, LayoutedTreeViewNode, LayoutedTreemapDiagram, LayoutedTreemapNode,
    TreeViewDiagram, TreemapDiagram,
};

pub const VERSION: &str = "0.1.0";

const OUTER_PADDING: f64 = 8.0;
const NODE_GAP: f64 = 3.0;
const PARENT_HEADER: f64 = 24.0;

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
}
