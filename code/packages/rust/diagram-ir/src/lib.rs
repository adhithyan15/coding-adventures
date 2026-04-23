//! # diagram-ir
//!
//! Semantic diagram intermediate representation (IR) for the DG00 pipeline.
//!
//! This crate is the Rust counterpart of the TypeScript
//! `@coding-adventures/diagram-ir` package. It defines the *shared vocabulary*
//! between all diagram source parsers (DOT, Mermaid, PlantUML…) and all
//! layout engines (`diagram-layout-graph`, `diagram-layout-sequence`, …).
//!
//! ## Pipeline position
//!
//! ```text
//! dot-parser / mermaid-parser / …
//!   → GraphDiagram  ←── this crate
//!   → diagram-layout-graph
//!   → LayoutedGraphDiagram  ←── this crate
//!   → diagram-to-paint
//!   → PaintScene
//! ```
//!
//! ## Design rules (from DG00)
//!
//! - No absolute `x` / `y` coordinates in the semantic IR.
//! - No bezier control points.
//! - No backend-specific colour strings.
//! - No glyph IDs.
//!
//! Those belong in `LayoutedGraphDiagram` (geometry) or `PaintScene` (render).

pub const VERSION: &str = "0.1.0";

// ============================================================================
// DiagramDirection — which axis is the "rank" axis
// ============================================================================

/// Direction of graph layout — which way the main flow runs.
///
/// Think of a flowchart:
///
/// ```text
/// TB = "Top to Bottom" — boxes flow downward (the most common flowchart layout)
/// LR = "Left to Right" — boxes flow rightward (good for timelines)
/// RL = "Right to Left" — reverse of LR
/// BT = "Bottom to Top" — reverse of TB
/// ```
///
/// DOT uses the `rankdir` attribute: `rankdir=LR` maps to [`DiagramDirection::Lr`].
#[derive(Clone, Debug, PartialEq)]
pub enum DiagramDirection {
    /// Top to bottom (default). Ranks increase downward.
    Tb,
    /// Left to right. Ranks increase rightward.
    Lr,
    /// Right to left. Ranks increase leftward.
    Rl,
    /// Bottom to top. Ranks increase upward.
    Bt,
}

impl Default for DiagramDirection {
    fn default() -> Self {
        DiagramDirection::Tb
    }
}

// ============================================================================
// DiagramShape — the visual shape of a graph node
// ============================================================================

/// The visual shape used to draw a graph node's outline.
///
/// These are the shapes that the layout and paint layers know how to render.
/// Source-specific shapes (e.g., DOT's `hexagon`) that have no canonical
/// mapping are rounded rectangles.
///
/// ```text
/// Rect           RoundedRect      Ellipse         Diamond
/// ┌───────┐      ╭───────╮         ╭───────╮      ◇
/// │       │      │       │        ╱         ╲    ╱ ╲
/// └───────┘      ╰───────╯        ╲         ╱   ╲   ╱
///                                  ╰───────╯     ◇
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum DiagramShape {
    /// Plain rectangle with sharp corners.
    Rect,
    /// Rectangle with rounded corners (most common default).
    RoundedRect,
    /// Ellipse or circle.
    Ellipse,
    /// Diamond / rhombus — used for decision nodes.
    Diamond,
}

impl Default for DiagramShape {
    fn default() -> Self {
        DiagramShape::RoundedRect
    }
}

// ============================================================================
// DiagramLabel — a text label attached to a node or edge
// ============================================================================

/// A text label attached to a node or edge.
///
/// v1 only carries a plain string. Future versions will carry rich text
/// (bold, italic, HTML) and a style reference.
#[derive(Clone, Debug, PartialEq)]
pub struct DiagramLabel {
    pub text: String,
}

impl DiagramLabel {
    pub fn new(text: impl Into<String>) -> Self {
        DiagramLabel { text: text.into() }
    }
}

// ============================================================================
// DiagramStyle — optional per-element style overrides
// ============================================================================

/// Optional style overrides for a node or edge.
///
/// Every field is `Option<_>`. A `None` field means "use the resolved
/// default from [`ResolvedDiagramStyle`]". This lets authors override only
/// the fields they care about:
///
/// ```rust
/// use diagram_ir::DiagramStyle;
/// let red_border = DiagramStyle {
///     stroke: Some("#ef4444".to_string()),
///     ..Default::default()
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DiagramStyle {
    /// Fill colour of the node body (CSS colour string).
    pub fill: Option<String>,
    /// Stroke (border) colour.
    pub stroke: Option<String>,
    /// Border width in pixels.
    pub stroke_width: Option<f64>,
    /// Colour of text labels.
    pub text_color: Option<String>,
    /// Font size for labels, in pixels.
    pub font_size: Option<f64>,
    /// Corner rounding radius for `RoundedRect` nodes.
    pub corner_radius: Option<f64>,
}

// ============================================================================
// ResolvedDiagramStyle — fully-resolved style with no optional fields
// ============================================================================

/// Fully-resolved style with concrete values for every field.
///
/// Produced by [`resolve_style`] by filling in defaults wherever the
/// source [`DiagramStyle`] left a field as `None`.
///
/// The defaults match the TypeScript `@coding-adventures/diagram-ir`
/// package so diagrams look the same in Rust and TypeScript pipelines.
///
/// | Field          | Default     |
/// |----------------|-------------|
/// | `fill`         | `#eff6ff`   |
/// | `stroke`       | `#2563eb`   |
/// | `stroke_width` | `2.0`       |
/// | `text_color`   | `#1e40af`   |
/// | `font_size`    | `14.0`      |
/// | `corner_radius`| `8.0`       |
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedDiagramStyle {
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f64,
    pub text_color: String,
    pub font_size: f64,
    pub corner_radius: f64,
}

impl Default for ResolvedDiagramStyle {
    fn default() -> Self {
        ResolvedDiagramStyle {
            fill:         "#eff6ff".to_string(),
            stroke:       "#2563eb".to_string(),
            stroke_width: 2.0,
            text_color:   "#1e40af".to_string(),
            font_size:    14.0,
            corner_radius: 8.0,
        }
    }
}

/// Resolve an optional [`DiagramStyle`] against the package-level defaults.
///
/// Any `None` field in `style` falls back to the value in
/// [`ResolvedDiagramStyle::default()`].
pub fn resolve_style(style: Option<&DiagramStyle>) -> ResolvedDiagramStyle {
    resolve_style_with_base(style, ResolvedDiagramStyle::default())
}

/// Resolve an optional [`DiagramStyle`] against a caller-supplied base.
///
/// Useful for edges, which use a different default fill (`"none"`) than nodes.
pub fn resolve_style_with_base(
    style: Option<&DiagramStyle>,
    base: ResolvedDiagramStyle,
) -> ResolvedDiagramStyle {
    match style {
        None => base,
        Some(s) => ResolvedDiagramStyle {
            fill:          s.fill.clone().unwrap_or(base.fill),
            stroke:        s.stroke.clone().unwrap_or(base.stroke),
            stroke_width:  s.stroke_width.unwrap_or(base.stroke_width),
            text_color:    s.text_color.clone().unwrap_or(base.text_color),
            font_size:     s.font_size.unwrap_or(base.font_size),
            corner_radius: s.corner_radius.unwrap_or(base.corner_radius),
        },
    }
}

// ============================================================================
// EdgeKind — directed or undirected
// ============================================================================

/// Whether an edge is directed (has an arrowhead) or undirected (plain line).
///
/// In DOT: `->` produces a directed edge; `--` produces an undirected edge.
#[derive(Clone, Debug, PartialEq)]
pub enum EdgeKind {
    Directed,
    Undirected,
}

// ============================================================================
// GraphNode — a single node in the semantic diagram
// ============================================================================

/// A node in the semantic graph diagram.
///
/// This is the parsed, pre-layout representation. It carries the node's
/// identity, label, shape preference, and optional style. It does NOT
/// carry x/y coordinates — those are assigned by `diagram-layout-graph`.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphNode {
    /// Unique node identifier (from DOT node ID, Mermaid node key, etc.).
    pub id: String,
    /// Human-readable label shown inside the node shape.
    pub label: DiagramLabel,
    /// Preferred shape; `None` → layout will use `RoundedRect`.
    pub shape: Option<DiagramShape>,
    /// Optional per-node style overrides.
    pub style: Option<DiagramStyle>,
}

// ============================================================================
// GraphEdge — a connection between two nodes
// ============================================================================

/// A directed or undirected connection between two nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdge {
    /// Optional stable identifier (useful for debugging and incremental layout).
    pub id: Option<String>,
    /// Source node id.
    pub from: String,
    /// Target node id.
    pub to: String,
    /// Optional label shown along the edge.
    pub label: Option<DiagramLabel>,
    /// Whether the edge has an arrowhead (`Directed`) or not (`Undirected`).
    pub kind: EdgeKind,
    /// Optional per-edge style overrides.
    pub style: Option<DiagramStyle>,
}

// ============================================================================
// GraphDiagram — the complete semantic diagram (pre-layout)
// ============================================================================

/// A complete semantic graph diagram — the output of all DOT / Mermaid parsers.
///
/// This is the input to `diagram-layout-graph`. It carries:
///
/// - the flow direction
/// - an ordered list of nodes (order is preserved but not semantically
///   significant — the layout engine computes placement)
/// - an ordered list of edges
/// - an optional diagram title
///
/// What it does NOT carry:
///
/// - Absolute geometry (x, y, width, height)
/// - Render commands
/// - Backend-specific handles
#[derive(Clone, Debug, PartialEq)]
pub struct GraphDiagram {
    /// Direction of the main flow axis.
    pub direction: DiagramDirection,
    /// Optional title shown at the top of the rendered diagram.
    pub title: Option<String>,
    /// All nodes in the diagram.
    pub nodes: Vec<GraphNode>,
    /// All edges in the diagram.
    pub edges: Vec<GraphEdge>,
}

// ============================================================================
// Geometry types — used in the layouted IR
// ============================================================================

/// A 2D point with floating-point coordinates.
///
/// Coordinate system: top-left origin, Y increasing downward (same as
/// PaintScene, SVG, and HTML Canvas).
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

// ============================================================================
// LayoutedGraphNode — a node after geometry has been assigned
// ============================================================================

/// A graph node after layout has assigned absolute geometry.
///
/// Produced by `diagram-layout-graph::layout_graph_diagram`.
/// Consumed by `diagram-to-paint::diagram_to_paint`.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedGraphNode {
    pub id: String,
    pub label: DiagramLabel,
    /// Resolved shape (never `None` after layout; defaults to `RoundedRect`).
    pub shape: DiagramShape,
    /// Left edge of the bounding box.
    pub x: f64,
    /// Top edge of the bounding box.
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Fully-resolved style (no `Option` fields).
    pub style: ResolvedDiagramStyle,
}

// ============================================================================
// LayoutedGraphEdge — an edge after routing has been computed
// ============================================================================

/// A graph edge after layout has computed its route.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedGraphEdge {
    pub id: Option<String>,
    pub from_node_id: String,
    pub to_node_id: String,
    pub kind: EdgeKind,
    /// Ordered waypoints of the edge route (2 points for straight edges,
    /// 5 points for self-loops).
    pub points: Vec<Point>,
    pub label: Option<DiagramLabel>,
    /// Midpoint where the edge label is positioned (if any).
    pub label_position: Option<Point>,
    pub style: ResolvedDiagramStyle,
}

// ============================================================================
// LayoutedGraphDiagram — the complete diagram after layout
// ============================================================================

/// A fully-laid-out graph diagram with absolute geometry.
///
/// This is the output of `diagram-layout-graph` and the input to
/// `diagram-to-paint`. Every node has a bounding box; every edge has a
/// routed polyline.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedGraphDiagram {
    pub direction: DiagramDirection,
    pub title: Option<String>,
    /// Total canvas width in pixels.
    pub width: f64,
    /// Total canvas height in pixels.
    pub height: f64,
    pub nodes: Vec<LayoutedGraphNode>,
    pub edges: Vec<LayoutedGraphEdge>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn default_direction_is_tb() {
        assert_eq!(DiagramDirection::default(), DiagramDirection::Tb);
    }

    #[test]
    fn default_shape_is_rounded_rect() {
        assert_eq!(DiagramShape::default(), DiagramShape::RoundedRect);
    }

    #[test]
    fn resolve_style_none_gives_defaults() {
        let s = resolve_style(None);
        assert_eq!(s.fill, "#eff6ff");
        assert_eq!(s.stroke, "#2563eb");
        assert_eq!(s.stroke_width, 2.0);
        assert_eq!(s.text_color, "#1e40af");
        assert_eq!(s.font_size, 14.0);
        assert_eq!(s.corner_radius, 8.0);
    }

    #[test]
    fn resolve_style_partial_override() {
        let style = DiagramStyle {
            fill: Some("#ff0000".to_string()),
            ..Default::default()
        };
        let s = resolve_style(Some(&style));
        assert_eq!(s.fill, "#ff0000");
        assert_eq!(s.stroke, "#2563eb"); // unchanged
    }

    #[test]
    fn resolve_style_with_base_overrides_base() {
        let base = ResolvedDiagramStyle {
            fill: "none".to_string(),
            stroke: "#4b5563".to_string(),
            stroke_width: 2.0,
            text_color: "#374151".to_string(),
            font_size: 12.0,
            corner_radius: 0.0,
        };
        let s = resolve_style_with_base(None, base.clone());
        assert_eq!(s.fill, "none");
        assert_eq!(s.stroke, "#4b5563");
    }

    #[test]
    fn graph_diagram_builds_correctly() {
        let node = GraphNode {
            id: "A".to_string(),
            label: DiagramLabel::new("Node A"),
            shape: None,
            style: None,
        };
        let edge = GraphEdge {
            id: None,
            from: "A".to_string(),
            to: "B".to_string(),
            label: None,
            kind: EdgeKind::Directed,
            style: None,
        };
        let diagram = GraphDiagram {
            direction: DiagramDirection::Lr,
            title: Some("My Graph".to_string()),
            nodes: vec![node],
            edges: vec![edge],
        };
        assert_eq!(diagram.nodes.len(), 1);
        assert_eq!(diagram.edges.len(), 1);
        assert_eq!(diagram.direction, DiagramDirection::Lr);
    }

    #[test]
    fn diagram_label_new() {
        let label = DiagramLabel::new("hello");
        assert_eq!(label.text, "hello");
    }
}
