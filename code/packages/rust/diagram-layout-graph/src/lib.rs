//! # diagram-layout-graph
//!
//! Topological rank assignment and absolute geometry layout for graph diagrams.
//!
//! Takes a [`GraphDiagram`] (pre-layout semantic IR from `diagram-ir`) and
//! produces a [`LayoutedGraphDiagram`] with concrete bounding boxes and edge
//! routes. The output feeds directly into `diagram-to-paint`.
//!
//! ## Algorithm
//!
//! ```text
//! 1. Build a directed-graph from the diagram's edge list.
//! 2. Topological sort → ordered node list.
//!    • If a cycle is detected, fall back to one-rank-per-node flat layout.
//! 3. Assign rank(n) = max(rank(predecessor)) + 1, or 0 for roots.
//!    This groups nodes into "levels" — the depth layers of the DAG.
//! 4. Place bounding boxes:
//!    • TB/BT: rank is the Y axis; nodes within a rank are spaced on X.
//!    • LR/RL: rank is the X axis; nodes within a rank are spaced on Y.
//! 5. Route edges as 2-point polylines between node boundary midpoints.
//!    Self-loops use a 5-point detour above the node.
//! 6. Compute edge label midpoints (if any).
//! ```
//!
//! ## Constants
//!
//! All layout constants have sensible defaults matching the TypeScript
//! `@coding-adventures/diagram-layout-graph` package.
//!
//! | Constant        | Default | Meaning                                  |
//! |-----------------|---------|------------------------------------------|
//! | `margin`        | 24      | Canvas padding on all sides (px)         |
//! | `rank_gap`      | 96      | Gap between adjacent ranks (px)          |
//! | `node_gap`      | 56      | Gap between nodes in the same rank (px)  |
//! | `title_gap`     | 48      | Extra Y inset when a title is present    |
//! | `min_node_width`| 96      | Minimum node bounding-box width (px)     |
//! | `node_height`   | 52      | Fixed bounding-box height (px)           |
//! | `h_padding`     | 24      | Horizontal text padding inside a node    |
//! | `char_width`    | 8       | Approximate width per character (px)     |

pub const VERSION: &str = "0.1.0";

use diagram_ir::{
    DiagramDirection, GraphDiagram, LayoutedGraphDiagram, LayoutedGraphEdge,
    LayoutedGraphNode, Point, ResolvedDiagramStyle, resolve_style, resolve_style_with_base,
};
use directed_graph::Graph;
use layout_ir::{FontSpec, TextMeasurer};

// ============================================================================
// Layout options
// ============================================================================

/// Tuning knobs for the graph layout algorithm.
///
/// All fields are optional — a `None` value falls back to the constant default
/// listed in the module documentation.
#[derive(Clone, Debug, Default)]
pub struct GraphLayoutOptions {
    pub margin:          Option<f64>,
    pub rank_gap:        Option<f64>,
    pub node_gap:        Option<f64>,
    pub title_gap:       Option<f64>,
    pub min_node_width:  Option<f64>,
    pub node_height:     Option<f64>,
    pub h_padding:       Option<f64>,
    pub char_width:      Option<f64>,
}

struct Opts {
    margin:         f64,
    rank_gap:       f64,
    node_gap:       f64,
    title_gap:      f64,
    min_node_width: f64,
    node_height:    f64,
    h_padding:      f64,
    char_width:     f64,
}

impl Opts {
    fn from(options: Option<&GraphLayoutOptions>) -> Self {
        let o = options.cloned().unwrap_or_default();
        Opts {
            margin:         o.margin.unwrap_or(24.0),
            rank_gap:       o.rank_gap.unwrap_or(96.0),
            node_gap:       o.node_gap.unwrap_or(56.0),
            title_gap:      o.title_gap.unwrap_or(48.0),
            min_node_width: o.min_node_width.unwrap_or(96.0),
            node_height:    o.node_height.unwrap_or(52.0),
            h_padding:      o.h_padding.unwrap_or(24.0),
            char_width:     o.char_width.unwrap_or(8.0),
        }
    }
}

// ============================================================================
// Node width calculation
// ============================================================================

/// Default label font spec: Helvetica 14 pt weight 400 (matches diagram-to-paint).
fn label_font_spec() -> FontSpec {
    FontSpec {
        family: "Helvetica".to_string(),
        size: 14.0,
        weight: 400,
        italic: false,
        line_height: 1.2,
    }
}

/// Compute the bounding-box width for a node given its label.
///
/// When `measurer` is supplied, the real glyph-advance measurement is used:
///   width = max(min_node_width, h_padding × 2 + measured.width)
///
/// When no measurer is supplied (tests, environments without a font stack),
/// the heuristic fallback is used:
///   width = max(min_node_width, h_padding × 2 + label_chars × char_width)
fn node_width(label: &str, opts: &Opts, measurer: Option<&dyn TextMeasurer>) -> f64 {
    let text_width = if let Some(m) = measurer {
        let result = m.measure(label, &label_font_spec(), None);
        opts.h_padding * 2.0 + result.width
    } else {
        opts.h_padding * 2.0 + label.len() as f64 * opts.char_width
    };
    text_width.max(opts.min_node_width)
}

// ============================================================================
// Rank assignment (topological)
// ============================================================================

/// Group node ids into rank layers via topological sort.
///
/// Returns `Vec<Vec<node_id>>` ordered from rank-0 (roots) to rank-N (leaves).
///
/// The rank of a node is `max(rank(predecessors)) + 1`. Roots (nodes with no
/// predecessors) get rank 0. This mirrors the Sugiyama-style layering used in
/// most DAG layout algorithms.
///
/// If the graph contains a cycle, `Graph::topological_sort` returns an error.
/// We catch that and fall back to a flat layout: one rank per node in
/// declaration order.
fn assign_ranks(diagram: &GraphDiagram) -> Vec<Vec<String>> {
    let mut g = Graph::new_allow_self_loops();
    for node in &diagram.nodes {
        g.add_node(&node.id);
    }
    for edge in &diagram.edges {
        // Ignore self-loops for rank assignment (they don't change depth).
        if edge.from != edge.to {
            let _ = g.add_edge(&edge.from, &edge.to);
        }
    }

    let topo = match g.topological_sort() {
        Ok(order) => order,
        Err(_)    => {
            // Cycle detected — fall back to flat layout.
            return diagram.nodes.iter().map(|n| vec![n.id.clone()]).collect();
        }
    };

    // Walk the topological order, assigning each node the depth one beyond
    // the deepest of its predecessors.
    let mut rank_map: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for node_id in &topo {
        let predecessors = g.predecessors(node_id).unwrap_or_default();
        let rank = if predecessors.is_empty() {
            0
        } else {
            predecessors
                .iter()
                .filter_map(|p| rank_map.get(p).copied())
                .max()
                .map(|m| m + 1)
                .unwrap_or(0)
        };
        rank_map.insert(node_id.clone(), rank);
    }

    // Group by rank, preserving the topological order within each rank.
    let max_rank = rank_map.values().copied().max().unwrap_or(0);
    let mut ranks: Vec<Vec<String>> = vec![Vec::new(); max_rank + 1];
    for node_id in &topo {
        let rank = rank_map[node_id];
        ranks[rank].push(node_id.clone());
    }

    // Append any nodes that didn't appear in the topo order (isolated nodes
    // with no edges), assigning them rank 0.
    let seen: std::collections::HashSet<&str> =
        topo.iter().map(|s| s.as_str()).collect();
    for node in &diagram.nodes {
        if !seen.contains(node.id.as_str()) {
            ranks[0].push(node.id.clone());
        }
    }

    // Remove empty rank buckets (shouldn't happen, but be defensive).
    ranks.retain(|r| !r.is_empty());
    ranks
}

// ============================================================================
// Node placement
// ============================================================================

fn place_nodes(
    diagram: &GraphDiagram,
    opts: &Opts,
    measurer: Option<&dyn TextMeasurer>,
) -> (Vec<LayoutedGraphNode>, f64, f64) {
    let direction = &diagram.direction;
    let ranks = assign_ranks(diagram);
    let top_inset = if diagram.title.is_some() { opts.title_gap } else { 0.0 };

    // Compute the maximum node width within each rank (used to set column widths
    // in LR/RL mode so all nodes in a column line up).
    let rank_sizes: Vec<f64> = ranks
        .iter()
        .map(|rank| {
            rank.iter()
                .map(|id| {
                    let node = diagram.nodes.iter().find(|n| n.id == *id).unwrap();
                    node_width(&node.label.text, opts, measurer)
                })
                .fold(0.0_f64, f64::max)
        })
        .collect();

    let max_rank_size = rank_sizes.iter().cloned().fold(0.0_f64, f64::max);

    let mut nodes: Vec<LayoutedGraphNode> = Vec::new();

    for (rank_index, rank) in ranks.iter().enumerate() {
        for (item_index, node_id) in rank.iter().enumerate() {
            let node = diagram.nodes.iter().find(|n| n.id == *node_id).unwrap();
            let width  = node_width(&node.label.text, opts, measurer);
            let height = opts.node_height;

            // For BT/RL, reverse the rank axis so rank 0 appears at the
            // "start" of the reversed direction.
            let major_index = match direction {
                DiagramDirection::Rl | DiagramDirection::Bt => {
                    ranks.len() - rank_index - 1
                }
                _ => rank_index,
            };

            let (x, y) = match direction {
                DiagramDirection::Lr | DiagramDirection::Rl => {
                    let x = opts.margin + major_index as f64 * (max_rank_size + opts.rank_gap);
                    let y = opts.margin + top_inset
                        + item_index as f64 * (height + opts.node_gap);
                    (x, y)
                }
                _ => {
                    // TB or BT
                    let x = opts.margin + item_index as f64 * (width + opts.node_gap);
                    let y = opts.margin + top_inset
                        + major_index as f64 * (height + opts.rank_gap);
                    (x, y)
                }
            };

            let style = resolve_style(node.style.as_ref());

            nodes.push(LayoutedGraphNode {
                id:     node_id.clone(),
                label:  node.label.clone(),
                shape:  node.shape.clone().unwrap_or_default(),
                x, y, width, height,
                style,
            });
        }
    }

    let max_x = nodes
        .iter()
        .map(|n| n.x + n.width)
        .fold(opts.margin * 2.0, f64::max);
    let max_y = nodes
        .iter()
        .map(|n| n.y + n.height)
        .fold(opts.margin * 2.0, f64::max);

    (nodes, max_x + opts.margin, max_y + opts.margin)
}

// ============================================================================
// Edge routing
// ============================================================================

/// Compute the start and end attachment points for a straight edge.
///
/// The attachment points depend on direction:
///
/// - `LR`: right midpoint of from-node → left midpoint of to-node.
/// - `RL`: left midpoint → right midpoint (reversed).
/// - `TB`: bottom midpoint → top midpoint.
/// - `BT`: top midpoint → bottom midpoint.
fn edge_endpoints(
    direction: &DiagramDirection,
    from: &LayoutedGraphNode,
    to: &LayoutedGraphNode,
) -> (Point, Point) {
    // Self-loops: special-case; handled separately.
    if from.id == to.id {
        let cx = from.x + from.width / 2.0;
        return (
            Point { x: from.x + from.width, y: from.y + from.height / 2.0 },
            Point { x: cx, y: from.y },
        );
    }

    match direction {
        DiagramDirection::Lr => (
            Point { x: from.x + from.width, y: from.y + from.height / 2.0 },
            Point { x: to.x,                y: to.y   + to.height   / 2.0 },
        ),
        DiagramDirection::Rl => (
            Point { x: from.x,              y: from.y + from.height / 2.0 },
            Point { x: to.x + to.width,     y: to.y   + to.height   / 2.0 },
        ),
        DiagramDirection::Bt => (
            Point { x: from.x + from.width / 2.0, y: from.y },
            Point { x: to.x   + to.width   / 2.0, y: to.y + to.height },
        ),
        DiagramDirection::Tb => (
            Point { x: from.x + from.width / 2.0, y: from.y + from.height },
            Point { x: to.x   + to.width   / 2.0, y: to.y },
        ),
    }
}

fn route_edge(
    edge: &diagram_ir::GraphEdge,
    direction: &DiagramDirection,
    nodes_by_id: &std::collections::HashMap<String, &LayoutedGraphNode>,
) -> LayoutedGraphEdge {
    let from_node = nodes_by_id[&edge.from];
    let to_node   = nodes_by_id[&edge.to];

    let (start, end) = edge_endpoints(direction, from_node, to_node);

    // Self-loops use a 5-point detour that loops above the node.
    let points = if from_node.id == to_node.id {
        vec![
            start.clone(),
            Point { x: start.x + 28.0, y: start.y },
            Point { x: start.x + 28.0, y: from_node.y - 28.0 },
            Point { x: from_node.x + from_node.width / 2.0, y: from_node.y - 28.0 },
            end.clone(),
        ]
    } else {
        vec![start.clone(), end.clone()]
    };

    let label_position = edge.label.as_ref().map(|_| Point {
        x: (start.x + end.x) / 2.0,
        y: (start.y + end.y) / 2.0 - 8.0,
    });

    // Default edge style: no fill, dark grey stroke.
    let edge_base = ResolvedDiagramStyle {
        fill:          "none".to_string(),
        stroke:        "#4b5563".to_string(),
        stroke_width:  2.0,
        text_color:    "#374151".to_string(),
        font_size:     12.0,
        corner_radius: 0.0,
    };
    let style = resolve_style_with_base(edge.style.as_ref(), edge_base);

    LayoutedGraphEdge {
        id:           edge.id.clone(),
        from_node_id: from_node.id.clone(),
        to_node_id:   to_node.id.clone(),
        kind:         edge.kind.clone(),
        points,
        label:        edge.label.clone(),
        label_position,
        style,
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Lay out a [`GraphDiagram`] and return a [`LayoutedGraphDiagram`] with
/// absolute geometry for every node and edge.
///
/// # Example
///
/// ```rust
/// use diagram_ir::{GraphDiagram, GraphNode, GraphEdge, DiagramDirection,
///                   DiagramLabel, EdgeKind};
/// use diagram_layout_graph::layout_graph_diagram;
///
/// let diagram = GraphDiagram {
///     direction: DiagramDirection::Lr,
///     title: None,
///     nodes: vec![
///         GraphNode { id: "A".into(), label: DiagramLabel::new("A"),
///                     shape: None, style: None },
///         GraphNode { id: "B".into(), label: DiagramLabel::new("B"),
///                     shape: None, style: None },
///     ],
///     edges: vec![
///         GraphEdge { id: None, from: "A".into(), to: "B".into(),
///                     label: None, kind: EdgeKind::Directed, style: None },
///     ],
/// };
///
/// let layout = layout_graph_diagram(&diagram, None, None);
/// assert_eq!(layout.nodes.len(), 2);
/// assert_eq!(layout.edges.len(), 1);
/// assert!(layout.width > 0.0);
/// assert!(layout.height > 0.0);
/// ```
pub fn layout_graph_diagram(
    diagram: &GraphDiagram,
    options: Option<&GraphLayoutOptions>,
    measurer: Option<&dyn TextMeasurer>,
) -> LayoutedGraphDiagram {
    let opts = Opts::from(options);
    let (nodes, width, height) = place_nodes(diagram, &opts, measurer);

    let nodes_by_id: std::collections::HashMap<String, &LayoutedGraphNode> =
        nodes.iter().map(|n| (n.id.clone(), n)).collect();

    let edges = diagram
        .edges
        .iter()
        .filter_map(|edge| {
            if nodes_by_id.contains_key(&edge.from) && nodes_by_id.contains_key(&edge.to) {
                Some(route_edge(edge, &diagram.direction, &nodes_by_id))
            } else {
                None
            }
        })
        .collect();

    LayoutedGraphDiagram {
        direction: diagram.direction.clone(),
        title:     diagram.title.clone(),
        width,
        height,
        nodes,
        edges,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::{DiagramDirection, DiagramLabel, EdgeKind, GraphDiagram, GraphEdge, GraphNode};

    fn simple_node(id: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            label: DiagramLabel::new(id),
            shape: None,
            style: None,
        }
    }

    fn directed_edge(from: &str, to: &str) -> GraphEdge {
        GraphEdge {
            id: None,
            from: from.to_string(),
            to: to.to_string(),
            label: None,
            kind: EdgeKind::Directed,
            style: None,
        }
    }

    fn two_node_diagram(dir: DiagramDirection) -> GraphDiagram {
        GraphDiagram {
            direction: dir,
            title: None,
            nodes: vec![simple_node("A"), simple_node("B")],
            edges: vec![directed_edge("A", "B")],
        }
    }

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn layout_produces_correct_node_count() {
        let d = two_node_diagram(DiagramDirection::Tb);
        let l = layout_graph_diagram(&d, None, None);
        assert_eq!(l.nodes.len(), 2);
    }

    #[test]
    fn layout_produces_correct_edge_count() {
        let d = two_node_diagram(DiagramDirection::Tb);
        let l = layout_graph_diagram(&d, None, None);
        assert_eq!(l.edges.len(), 1);
    }

    #[test]
    fn layout_scene_has_positive_dimensions() {
        let d = two_node_diagram(DiagramDirection::Tb);
        let l = layout_graph_diagram(&d, None, None);
        assert!(l.width > 0.0);
        assert!(l.height > 0.0);
    }

    #[test]
    fn tb_nodes_have_different_y_coordinates() {
        let d = two_node_diagram(DiagramDirection::Tb);
        let l = layout_graph_diagram(&d, None, None);
        let ya = l.nodes.iter().find(|n| n.id == "A").unwrap().y;
        let yb = l.nodes.iter().find(|n| n.id == "B").unwrap().y;
        assert!(ya < yb, "in TB layout A should be above B");
    }

    #[test]
    fn lr_nodes_have_different_x_coordinates() {
        let d = two_node_diagram(DiagramDirection::Lr);
        let l = layout_graph_diagram(&d, None, None);
        let xa = l.nodes.iter().find(|n| n.id == "A").unwrap().x;
        let xb = l.nodes.iter().find(|n| n.id == "B").unwrap().x;
        assert!(xa < xb, "in LR layout A should be left of B");
    }

    #[test]
    fn edge_points_connect_nodes() {
        let d = two_node_diagram(DiagramDirection::Tb);
        let l = layout_graph_diagram(&d, None, None);
        let edge = &l.edges[0];
        assert_eq!(edge.points.len(), 2, "straight edge should have 2 points");
    }

    #[test]
    fn self_loop_has_five_points() {
        let d = GraphDiagram {
            direction: DiagramDirection::Tb,
            title: None,
            nodes: vec![simple_node("A")],
            edges: vec![directed_edge("A", "A")],
        };
        let l = layout_graph_diagram(&d, None, None);
        assert_eq!(l.edges[0].points.len(), 5, "self-loop should have 5-point detour");
    }

    #[test]
    fn title_adds_top_inset() {
        let d_no_title = two_node_diagram(DiagramDirection::Tb);
        let mut d_title = d_no_title.clone();
        d_title.title = Some("My Diagram".to_string());

        let l_no = layout_graph_diagram(&d_no_title, None, None);
        let l_yes = layout_graph_diagram(&d_title, None, None);

        let y_no = l_no.nodes[0].y;
        let y_yes = l_yes.nodes[0].y;
        assert!(y_yes > y_no, "title should push nodes down");
    }

    #[test]
    fn cycle_falls_back_to_flat_layout() {
        // A -> B -> A: cycle — should not panic.
        let d = GraphDiagram {
            direction: DiagramDirection::Tb,
            title: None,
            nodes: vec![simple_node("A"), simple_node("B")],
            edges: vec![directed_edge("A", "B"), directed_edge("B", "A")],
        };
        let l = layout_graph_diagram(&d, None, None);
        assert_eq!(l.nodes.len(), 2);
        // In flat fallback each node is its own rank, so they have different Y.
        let ya = l.nodes.iter().find(|n| n.id == "A").unwrap().y;
        let yb = l.nodes.iter().find(|n| n.id == "B").unwrap().y;
        assert_ne!(ya, yb);
    }

    #[test]
    fn node_width_respects_label_length() {
        let d = GraphDiagram {
            direction: DiagramDirection::Tb,
            title: None,
            nodes: vec![
                simple_node("A"),
                GraphNode {
                    id: "LongLabel".to_string(),
                    label: DiagramLabel::new("A very long label"),
                    shape: None,
                    style: None,
                },
            ],
            edges: vec![],
        };
        let l = layout_graph_diagram(&d, None, None);
        let w_short = l.nodes.iter().find(|n| n.id == "A").unwrap().width;
        let w_long  = l.nodes.iter().find(|n| n.id == "LongLabel").unwrap().width;
        assert!(w_long > w_short, "longer label should produce wider node");
    }

    #[test]
    fn three_rank_chain_has_increasing_y_in_tb() {
        let d = GraphDiagram {
            direction: DiagramDirection::Tb,
            title: None,
            nodes: vec![simple_node("A"), simple_node("B"), simple_node("C")],
            edges: vec![directed_edge("A", "B"), directed_edge("B", "C")],
        };
        let l = layout_graph_diagram(&d, None, None);
        let ya = l.nodes.iter().find(|n| n.id == "A").unwrap().y;
        let yb = l.nodes.iter().find(|n| n.id == "B").unwrap().y;
        let yc = l.nodes.iter().find(|n| n.id == "C").unwrap().y;
        assert!(ya < yb && yb < yc, "TB chain should have A above B above C");
    }
}
