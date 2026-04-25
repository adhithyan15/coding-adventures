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

pub const VERSION: &str = "0.2.0";

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

// ═══════════════════════════════════════════════════════════════════════════
// CHART FAMILY (DG04) — XY bar/line, pie, Sankey
// ═══════════════════════════════════════════════════════════════════════════

/// Top-level chart variant selector.
#[derive(Debug, Clone, PartialEq)]
pub enum ChartKind { Xy, Pie, Sankey }

/// Plot orientation (horizontal bar charts flip axes).
#[derive(Debug, Clone, PartialEq)]
pub enum ChartOrientation { Vertical, Horizontal }

/// Axis type — categorical (labelled buckets) or numeric (continuous).
#[derive(Debug, Clone, PartialEq)]
pub enum AxisKind { Categorical, Numeric }

/// One axis description in a chart.
#[derive(Debug, Clone, PartialEq)]
pub struct Axis {
    pub kind: AxisKind,
    pub title: Option<String>,
    pub categories: Vec<String>,
    pub min: f64,
    pub max: f64,
}

/// Bar or line series kind.
#[derive(Debug, Clone, PartialEq)]
pub enum SeriesKind { Bar, Line }

/// A bar or line series with optional label and data points.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartSeries {
    pub kind: SeriesKind,
    pub label: Option<String>,
    pub data: Vec<f64>,
}

/// One slice in a pie chart.
#[derive(Debug, Clone, PartialEq)]
pub struct PieSlice {
    pub label: String,
    pub value: f64,
}

/// A Sankey node (source or target of flows).
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyNode {
    pub id: String,
    pub label: Option<String>,
}

/// A weighted flow between two Sankey nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyFlow {
    pub source: String,
    pub target: String,
    pub weight: f64,
}

/// Semantic IR for a chart diagram. No geometry yet.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartDiagram {
    pub title: Option<String>,
    pub kind: ChartKind,
    pub x_axis: Option<Axis>,
    pub y_axis: Option<Axis>,
    pub series: Vec<ChartSeries>,
    pub slices: Vec<PieSlice>,
    pub sankey_nodes: Vec<SankeyNode>,
    pub flows: Vec<SankeyFlow>,
    pub orientation: ChartOrientation,
}

/// Spine / tick axis orientation in layouted output.
#[derive(Debug, Clone, PartialEq)]
pub enum Orientation { Horizontal, Vertical }

/// One entry in a chart legend.
#[derive(Debug, Clone, PartialEq)]
pub struct LegendEntry {
    pub color: String,
    pub label: String,
}

/// A positioned text label used in layouted charts.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutedLabel {
    pub x: f64,
    pub y: f64,
    pub text: String,
}

/// A geometry primitive produced by the chart layout engine.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutedChartItem {
    AxisSpine { x1: f64, y1: f64, x2: f64, y2: f64, orientation: Orientation },
    AxisTick  { x: f64, y: f64, label: String, orientation: Orientation },
    GridLine  { x1: f64, y1: f64, x2: f64, y2: f64 },
    Bar       { x: f64, y: f64, width: f64, height: f64, color: String },
    LinePath  { points: Vec<Point>, color: String },
    PieArc    { cx: f64, cy: f64, r: f64, start_angle: f64, end_angle: f64,
                color: String, label: String },
    SankeyBand{ from_x: f64, from_y: f64, to_x: f64, to_y: f64,
                width: f64, color: String },
    DataLabel { x: f64, y: f64, text: String },
    Legend    { x: f64, y: f64, entries: Vec<LegendEntry> },
}

/// Fully-layouted chart — ready for diagram-to-paint.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutedChartDiagram {
    pub width: f64,
    pub height: f64,
    pub title_box: Option<LayoutedLabel>,
    pub items: Vec<LayoutedChartItem>,
}

// ═══════════════════════════════════════════════════════════════════════════
// STRUCTURAL FAMILY (DG04) — class, ER, C4
// ═══════════════════════════════════════════════════════════════════════════

/// Top-level structural diagram variant.
#[derive(Debug, Clone, PartialEq)]
pub enum StructuralKind { Class, Er, C4 }

/// Node category within a structural diagram.
#[derive(Debug, Clone, PartialEq)]
pub enum StructuralNodeKind { Class, Interface, Abstract, Enum, Entity }

/// Compartment category (header / fields / methods / enum values).
#[derive(Debug, Clone, PartialEq)]
pub enum CompartmentKind { Header, Fields, Methods, Values }

/// A compartment inside a structural node.
#[derive(Debug, Clone, PartialEq)]
pub struct Compartment {
    pub kind: CompartmentKind,
    pub entries: Vec<String>,
}

/// A node in a structural diagram (class, entity, component …).
#[derive(Debug, Clone, PartialEq)]
pub struct StructuralNode {
    pub id: String,
    pub label: String,
    pub stereotype: Option<String>,
    pub node_kind: StructuralNodeKind,
    pub compartments: Vec<Compartment>,
}

/// Relationship kind between structural nodes.
#[derive(Debug, Clone, PartialEq)]
pub enum RelKind {
    Inheritance, Realization, Composition, Aggregation,
    Association, Dependency, Link,
}

/// A directed relationship between two structural nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct StructuralRelationship {
    pub from: String,
    pub to: String,
    pub kind: RelKind,
    pub from_mult: Option<String>,
    pub to_mult: Option<String>,
    pub label: Option<String>,
}

/// Semantic IR for a structural (class / ER / C4) diagram. No geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct StructuralDiagram {
    pub kind: StructuralKind,
    pub title: Option<String>,
    pub nodes: Vec<StructuralNode>,
    pub relationships: Vec<StructuralRelationship>,
}

/// A compartment after layout — knows its y-offset and per-row strings.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutedCompartment {
    pub y_offset: f64,
    pub height: f64,
    pub rows: Vec<String>,
}

/// A structural node after layout — carries its bounding box.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutedStructuralNode {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub header: String,
    pub stereotype: Option<String>,
    pub compartments: Vec<LayoutedCompartment>,
}

/// A structural relationship after layout — carries a polyline.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutedStructuralRelationship {
    pub from_id: String,
    pub to_id: String,
    pub kind: RelKind,
    pub points: Vec<Point>,
    pub from_mult: Option<String>,
    pub to_mult: Option<String>,
    pub label: Option<(Point, String)>,
}

/// Fully-layouted structural diagram — ready for diagram-to-paint.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutedStructuralDiagram {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<LayoutedStructuralNode>,
    pub relationships: Vec<LayoutedStructuralRelationship>,
}

// ═══════════════════════════════════════════════════════════════════════════
// TEMPORAL FAMILY (DG04) — Gantt, git-graph
// ═══════════════════════════════════════════════════════════════════════════

/// Top-level temporal diagram variant.
#[derive(Debug, Clone, PartialEq)]
pub enum TemporalKind { Gantt, Git }

/// Task start anchor — fixed date or relative ("after X").
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStart { Date(String), After(String) }

/// Visual status of a Gantt task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus { Normal, Done, Active, Crit, Milestone }

/// A single Gantt task.
#[derive(Debug, Clone, PartialEq)]
pub struct GanttTask {
    pub id: String,
    pub label: String,
    pub start: TaskStart,
    pub duration_days: f64,
    pub status: TaskStatus,
    pub dependencies: Vec<String>,
}

/// A labelled section grouping Gantt tasks.
#[derive(Debug, Clone, PartialEq)]
pub struct GanttSection {
    pub label: Option<String>,
    pub tasks: Vec<GanttTask>,
}

/// Semantic IR for a Gantt diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct GanttDiagram {
    pub date_format: String,
    pub sections: Vec<GanttSection>,
}

/// A named git branch.
#[derive(Debug, Clone, PartialEq)]
pub struct GitBranch { pub name: String }

/// A git graph event — commit, branch checkout, or merge.
#[derive(Debug, Clone, PartialEq)]
pub enum GitEvent {
    Commit  { id: Option<String>, message: Option<String>, tag: Option<String>, branch: String },
    Checkout{ branch: String },
    Merge   { from: String, id: Option<String>, tag: Option<String> },
}

/// Semantic IR for a git-graph diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct GitDiagram {
    pub direction: DiagramDirection,
    pub branches: Vec<GitBranch>,
    pub events: Vec<GitEvent>,
}

/// Union body for a temporal diagram.
#[derive(Debug, Clone, PartialEq)]
pub enum TemporalBody { Gantt(GanttDiagram), Git(GitDiagram) }

/// Semantic IR for a temporal (Gantt / git-graph) diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalDiagram {
    pub kind: TemporalKind,
    pub title: Option<String>,
    pub body: TemporalBody,
}

/// A geometry primitive produced by the temporal layout engine.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutedTemporalItem {
    TimeAxisSpine   { x1: f64, y1: f64, x2: f64, y2: f64 },
    TimeAxisTick    { x: f64, y: f64, label: String },
    SectionHeader   { x: f64, y: f64, width: f64, height: f64, label: String },
    TaskBar         { x: f64, y: f64, width: f64, height: f64,
                      status: TaskStatus, label: String },
    MilestoneMarker { x: f64, y: f64, label: String },
    TodayMarker     { x: f64, y1: f64, y2: f64 },
    BranchLane      { y: f64, color: String, label: String },
    CommitNode      { x: f64, y: f64, id: String,
                      message: Option<String>, tag: Option<String> },
    MergeArc        { from_x: f64, from_y: f64, to_x: f64, to_y: f64 },
}

/// Fully-layouted temporal diagram — ready for diagram-to-paint.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutedTemporalDiagram {
    pub width: f64,
    pub height: f64,
    pub items: Vec<LayoutedTemporalItem>,
}

// ═══════════════════════════════════════════════════════════════════════════
// GEOMETRIC FAMILY (DG04) — coordinate-first diagrams
// ═══════════════════════════════════════════════════════════════════════════

/// Text alignment within a geometric text element.
#[derive(Debug, Clone, PartialEq)]
pub enum TextAlign { Left, Center, Right }

/// A primitive element in a geometric diagram.
#[derive(Debug, Clone, PartialEq)]
pub enum GeoElement {
    Box {
        id: String, x: f64, y: f64, w: f64, h: f64,
        corner_radius: f64, label: Option<String>,
        fill: Option<String>, stroke: Option<String>,
    },
    Circle {
        id: String, cx: f64, cy: f64, r: f64,
        label: Option<String>, fill: Option<String>, stroke: Option<String>,
    },
    Line {
        id: String, x1: f64, y1: f64, x2: f64, y2: f64,
        arrow_end: bool, arrow_start: bool, stroke: Option<String>,
    },
    Arc {
        id: String, cx: f64, cy: f64, r: f64,
        start_deg: f64, end_deg: f64, stroke: Option<String>,
    },
    Text {
        id: String, x: f64, y: f64, text: String, align: TextAlign,
    },
}

/// Semantic IR for a geometric diagram. Element coordinates are authoritative.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometricDiagram {
    pub title: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub elements: Vec<GeoElement>,
}

/// Fully-layouted geometric diagram — canvas size resolved, elements unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutedGeometricDiagram {
    pub width: f64,
    pub height: f64,
    pub elements: Vec<GeoElement>,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_0_2_0() {
        assert_eq!(crate::VERSION, "0.2.0");
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
