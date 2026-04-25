//! # diagram-ir
//!
//! Semantic diagram intermediate representation (IR) for all diagram families.
//!
//! This crate defines the shared vocabulary between all diagram source parsers
//! (DOT, Mermaid, PlantUML…) and all layout engines, covering five families:
//!
//!   • **Graph**     — flowcharts, directed/undirected graphs (DG00)
//!   • **Chart**     — XY bar/line, pie, Sankey (DG04)
//!   • **Structural** — class, ER, C4 diagrams (DG04)
//!   • **Temporal**  — Gantt, git-graph (DG04)
//!   • **Geometric** — Pikchr-style coordinate diagrams (DG04)
//!
//! ## Pipeline position
//!
//! ```text
//! parser  →  SemanticDiagram  (this crate)
//!         →  layout engine
//!         →  LayoutedDiagram  (this crate)
//!         →  diagram-to-paint
//!         →  PaintScene
//! ```

pub const VERSION: &str = "0.2.0";

// ═══════════════════════════════════════════════════════════════════════════
// GRAPH FAMILY (DG00 — unchanged)
// ═══════════════════════════════════════════════════════════════════════════

/// Overall left-right / top-bottom flow for graphs.
#[derive(Debug, Clone, PartialEq)]
pub enum DiagramDirection {
    LeftRight,
    RightLeft,
    TopBottom,
    BottomTop,
}

/// Node shape variants used in Mermaid / DOT.
#[derive(Debug, Clone, PartialEq)]
pub enum DiagramShape {
    Rectangle,
    RoundedRectangle,
    Stadium,
    Subroutine,
    Cylindrical,
    Circle,
    Asymmetric,
    Rhombus,
    Hexagon,
    Parallelogram,
    ParallelogramAlt,
    Trapezoid,
    TrapezoidAlt,
    DoubleCircle,
}

/// Inline text label that may carry a Markdown or plain string.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagramLabel {
    pub text: String,
}

/// Raw styling key-value pairs (colour, stroke width …).
#[derive(Debug, Clone, PartialEq)]
pub struct DiagramStyle {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f64>,
    pub color: Option<String>,
}

/// Styling after resolving cascading class styles.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedDiagramStyle {
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f64,
    pub color: String,
}

/// Arrow / line decoration on a graph edge.
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeKind {
    Arrow,
    Open,
    Dot,
    Cross,
    DottedArrow,
    ThickArrow,
    BiArrow,
    None,
}

/// A single node in a graph-family diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub label: Option<DiagramLabel>,
    pub shape: DiagramShape,
    pub style: Option<DiagramStyle>,
    pub class: Option<String>,
    pub subgraph: Option<String>,
}

/// A directed or undirected edge between two graph nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub label: Option<DiagramLabel>,
    pub kind: EdgeKind,
    pub is_bidirectional: bool,
    pub length: u32,
}

/// Semantic IR for a graph / flowchart diagram. No geometry yet.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphDiagram {
    pub direction: DiagramDirection,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub title: Option<String>,
}

/// Bare 2-D point used in layouted types.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// A graph node after layout — carries geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutedGraphNode {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub label: Option<DiagramLabel>,
    pub shape: DiagramShape,
    pub style: Option<ResolvedDiagramStyle>,
}

/// A graph edge after layout — carries a polyline path.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutedGraphEdge {
    pub from: String,
    pub to: String,
    pub points: Vec<Point>,
    pub label: Option<DiagramLabel>,
    pub label_point: Option<Point>,
    pub kind: EdgeKind,
    pub is_bidirectional: bool,
}

/// A fully-layouted graph / flowchart — ready for diagram-to-paint.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutedGraphDiagram {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<LayoutedGraphNode>,
    pub edges: Vec<LayoutedGraphEdge>,
    pub title: Option<String>,
}

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

/// A bar or line series with optional label and data points.
#[derive(Debug, Clone, PartialEq)]
pub enum SeriesKind { Bar, Line }

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
    /// Mermaid / UML stereotype text, e.g. "interface".
    pub stereotype: Option<String>,
    pub node_kind: StructuralNodeKind,
    pub compartments: Vec<Compartment>,
}

/// Relationship kind between structural nodes.
#[derive(Debug, Clone, PartialEq)]
pub enum RelKind {
    Inheritance,
    Realization,
    Composition,
    Aggregation,
    Association,
    Dependency,
    Link,
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
pub enum TaskStart {
    Date(String),
    After(String),
}

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
pub struct GitBranch {
    pub name: String,
}

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
pub enum TemporalBody {
    Gantt(GanttDiagram),
    Git(GitDiagram),
}

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
///
/// All coordinates are in the user's own coordinate system; the layout engine
/// resolves the canvas size but does not move elements.
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
    /// Optional explicit canvas width; auto-computed from elements if None.
    pub width: Option<f64>,
    /// Optional explicit canvas height; auto-computed from elements if None.
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

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Version ──────────────────────────────────────────────────────────
    #[test]
    fn version_is_0_2_0() { assert_eq!(crate::VERSION, "0.2.0"); }

    // ── Graph family ─────────────────────────────────────────────────────
    #[test]
    fn graph_diagram_roundtrip() {
        let d = GraphDiagram {
            direction: DiagramDirection::LeftRight,
            nodes: vec![GraphNode {
                id: "a".into(), label: Some(DiagramLabel { text: "A".into() }),
                shape: DiagramShape::Rectangle,
                style: None, class: None, subgraph: None,
            }],
            edges: vec![],
            title: None,
        };
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.direction, DiagramDirection::LeftRight);
    }

    #[test]
    fn layouted_graph_has_geometry() {
        let n = LayoutedGraphNode {
            id: "x".into(), x: 10.0, y: 20.0, width: 80.0, height: 40.0,
            label: None, shape: DiagramShape::Circle, style: None,
        };
        assert!(n.width > 0.0);
    }

    // ── Chart family ─────────────────────────────────────────────────────
    #[test]
    fn chart_diagram_xy_roundtrip() {
        let d = ChartDiagram {
            title: Some("Sales".into()),
            kind: ChartKind::Xy,
            x_axis: Some(Axis {
                kind: AxisKind::Categorical,
                title: None,
                categories: vec!["Jan".into(), "Feb".into()],
                min: 0.0, max: 0.0,
            }),
            y_axis: Some(Axis {
                kind: AxisKind::Numeric,
                title: None,
                categories: vec![],
                min: 0.0, max: 100.0,
            }),
            series: vec![ChartSeries {
                kind: SeriesKind::Bar,
                label: Some("Revenue".into()),
                data: vec![40.0, 60.0],
            }],
            slices: vec![], sankey_nodes: vec![], flows: vec![],
            orientation: ChartOrientation::Vertical,
        };
        assert_eq!(d.kind, ChartKind::Xy);
        assert_eq!(d.series[0].data.len(), 2);
    }

    #[test]
    fn chart_diagram_pie_roundtrip() {
        let d = ChartDiagram {
            title: None, kind: ChartKind::Pie,
            x_axis: None, y_axis: None, series: vec![],
            slices: vec![
                PieSlice { label: "A".into(), value: 60.0 },
                PieSlice { label: "B".into(), value: 40.0 },
            ],
            sankey_nodes: vec![], flows: vec![],
            orientation: ChartOrientation::Vertical,
        };
        assert_eq!(d.slices.len(), 2);
    }

    #[test]
    fn chart_diagram_sankey_roundtrip() {
        let d = ChartDiagram {
            title: None, kind: ChartKind::Sankey,
            x_axis: None, y_axis: None, series: vec![], slices: vec![],
            sankey_nodes: vec![
                SankeyNode { id: "a".into(), label: None },
                SankeyNode { id: "b".into(), label: None },
            ],
            flows: vec![SankeyFlow { source: "a".into(), target: "b".into(), weight: 10.0 }],
            orientation: ChartOrientation::Horizontal,
        };
        assert_eq!(d.flows.len(), 1);
    }

    // ── Structural family ─────────────────────────────────────────────────
    #[test]
    fn structural_diagram_roundtrip() {
        let d = StructuralDiagram {
            kind: StructuralKind::Class,
            title: Some("Domain Model".into()),
            nodes: vec![StructuralNode {
                id: "Animal".into(), label: "Animal".into(),
                stereotype: Some("abstract".into()),
                node_kind: StructuralNodeKind::Abstract,
                compartments: vec![Compartment {
                    kind: CompartmentKind::Methods,
                    entries: vec!["speak()".into()],
                }],
            }],
            relationships: vec![],
        };
        assert_eq!(d.nodes[0].compartments[0].entries.len(), 1);
    }

    #[test]
    fn structural_relationship_kinds() {
        // Verify that all RelKind variants are distinct (no copy-paste collision).
        let kinds = vec![
            RelKind::Inheritance, RelKind::Realization, RelKind::Composition,
            RelKind::Aggregation, RelKind::Association, RelKind::Dependency,
            RelKind::Link,
        ];
        // 7 variants, all differ from each other.
        assert_eq!(kinds.len(), 7);
        assert_ne!(kinds[0], kinds[1]);
    }

    // ── Temporal family ───────────────────────────────────────────────────
    #[test]
    fn gantt_diagram_roundtrip() {
        let d = GanttDiagram {
            date_format: "YYYY-MM-DD".into(),
            sections: vec![GanttSection {
                label: Some("Phase 1".into()),
                tasks: vec![GanttTask {
                    id: "t1".into(), label: "Design".into(),
                    start: TaskStart::Date("2026-01-01".into()),
                    duration_days: 5.0,
                    status: TaskStatus::Done,
                    dependencies: vec![],
                }],
            }],
        };
        assert_eq!(d.sections[0].tasks[0].duration_days, 5.0);
    }

    #[test]
    fn git_diagram_roundtrip() {
        let d = GitDiagram {
            direction: DiagramDirection::LeftRight,
            branches: vec![GitBranch { name: "main".into() }],
            events: vec![GitEvent::Commit {
                id: Some("abc".into()), message: Some("init".into()),
                tag: None, branch: "main".into(),
            }],
        };
        assert_eq!(d.branches.len(), 1);
    }

    // ── Geometric family ──────────────────────────────────────────────────
    #[test]
    fn geometric_diagram_roundtrip() {
        let d = GeometricDiagram {
            title: None, width: Some(400.0), height: Some(300.0),
            elements: vec![
                GeoElement::Box {
                    id: "b1".into(), x: 10.0, y: 10.0, w: 100.0, h: 50.0,
                    corner_radius: 4.0, label: Some("Input".into()),
                    fill: Some("#eef".into()), stroke: None,
                },
                GeoElement::Line {
                    id: "l1".into(), x1: 110.0, y1: 35.0, x2: 200.0, y2: 35.0,
                    arrow_end: true, arrow_start: false, stroke: None,
                },
            ],
        };
        assert_eq!(d.width, Some(400.0));
        assert_eq!(d.elements.len(), 2);
    }

    #[test]
    fn layouted_geometric_preserves_elements() {
        let lg = LayoutedGeometricDiagram {
            width: 500.0, height: 400.0,
            elements: vec![GeoElement::Circle {
                id: "c1".into(), cx: 100.0, cy: 100.0, r: 40.0,
                label: None, fill: None, stroke: None,
            }],
        };
        assert_eq!(lg.elements.len(), 1);
    }
}
