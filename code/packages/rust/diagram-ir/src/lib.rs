//! diagram-ir v0.2.0 — DG00/DG04 semantic IR

pub const VERSION: &str = "0.2.0";

#[derive(Clone, Debug, PartialEq)]
pub enum DiagramDirection { Tb, Lr, Rl, Bt }
impl Default for DiagramDirection { fn default() -> Self { DiagramDirection::Tb } }

#[derive(Clone, Debug, PartialEq)]
pub enum DiagramShape { Rect, RoundedRect, Ellipse, Diamond }
impl Default for DiagramShape { fn default() -> Self { DiagramShape::RoundedRect } }

#[derive(Clone, Debug, PartialEq)]
pub struct DiagramLabel { pub text: String }
impl DiagramLabel { pub fn new(text: impl Into<String>) -> Self { DiagramLabel { text: text.into() } } }

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DiagramStyle {
    pub fill: Option<String>, pub stroke: Option<String>,
    pub stroke_width: Option<f64>, pub text_color: Option<String>,
    pub font_size: Option<f64>, pub corner_radius: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedDiagramStyle {
    pub fill: String, pub stroke: String, pub stroke_width: f64,
    pub text_color: String, pub font_size: f64, pub corner_radius: f64,
}
impl Default for ResolvedDiagramStyle {
    fn default() -> Self {
        ResolvedDiagramStyle { fill:"#eff6ff".into(), stroke:"#2563eb".into(),
            stroke_width:2.0, text_color:"#1e40af".into(), font_size:14.0, corner_radius:8.0 }
    }
}
pub fn resolve_style(style: Option<&DiagramStyle>) -> ResolvedDiagramStyle {
    resolve_style_with_base(style, ResolvedDiagramStyle::default())
}
pub fn resolve_style_with_base(style: Option<&DiagramStyle>, base: ResolvedDiagramStyle) -> ResolvedDiagramStyle {
    match style {
        None => base,
        Some(s) => ResolvedDiagramStyle {
            fill: s.fill.clone().unwrap_or(base.fill),
            stroke: s.stroke.clone().unwrap_or(base.stroke),
            stroke_width: s.stroke_width.unwrap_or(base.stroke_width),
            text_color: s.text_color.clone().unwrap_or(base.text_color),
            font_size: s.font_size.unwrap_or(base.font_size),
            corner_radius: s.corner_radius.unwrap_or(base.corner_radius),
        },
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EdgeKind { Directed, Undirected }

#[derive(Clone, Debug, PartialEq)]
pub struct GraphNode { pub id: String, pub label: DiagramLabel, pub shape: Option<DiagramShape>, pub style: Option<DiagramStyle> }

#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdge { pub id: Option<String>, pub from: String, pub to: String, pub label: Option<DiagramLabel>, pub kind: EdgeKind, pub style: Option<DiagramStyle> }

#[derive(Clone, Debug, PartialEq)]
pub struct GraphDiagram { pub direction: DiagramDirection, pub title: Option<String>, pub nodes: Vec<GraphNode>, pub edges: Vec<GraphEdge> }

#[derive(Clone, Debug, PartialEq)]
pub struct Point { pub x: f64, pub y: f64 }

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedGraphNode { pub id: String, pub label: DiagramLabel, pub shape: DiagramShape, pub x: f64, pub y: f64, pub width: f64, pub height: f64, pub style: ResolvedDiagramStyle }

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedGraphEdge { pub id: Option<String>, pub from_node_id: String, pub to_node_id: String, pub kind: EdgeKind, pub points: Vec<Point>, pub label: Option<DiagramLabel>, pub label_position: Option<Point>, pub style: ResolvedDiagramStyle }

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedGraphDiagram { pub direction: DiagramDirection, pub title: Option<String>, pub width: f64, pub height: f64, pub nodes: Vec<LayoutedGraphNode>, pub edges: Vec<LayoutedGraphEdge> }

// CHART FAMILY
#[derive(Clone, Debug, PartialEq)]
pub enum ChartKind { Xy, Pie, Sankey }

#[derive(Clone, Debug, PartialEq)]
pub enum ChartOrientation { Vertical, Horizontal }
impl Default for ChartOrientation { fn default() -> Self { ChartOrientation::Vertical } }

#[derive(Clone, Debug, PartialEq)]
pub enum AxisKind { Categorical, Numeric }

#[derive(Clone, Debug, PartialEq)]
pub struct Axis { pub kind: AxisKind, pub title: Option<String>, pub categories: Vec<String>, pub min: f64, pub max: f64 }

#[derive(Clone, Debug, PartialEq)]
pub enum SeriesKind { Bar, Line }

#[derive(Clone, Debug, PartialEq)]
pub struct ChartSeries { pub kind: SeriesKind, pub label: Option<String>, pub data: Vec<f64> }

#[derive(Clone, Debug, PartialEq)]
pub struct PieSlice { pub label: String, pub value: f64 }

#[derive(Clone, Debug, PartialEq)]
pub struct SankeyNode { pub id: String, pub label: Option<String> }

#[derive(Clone, Debug, PartialEq)]
pub struct SankeyFlow { pub source: String, pub target: String, pub weight: f64 }

#[derive(Clone, Debug, PartialEq)]
pub struct ChartDiagram {
    pub title: Option<String>, pub kind: ChartKind,
    pub x_axis: Option<Axis>, pub y_axis: Option<Axis>,
    pub series: Vec<ChartSeries>, pub slices: Vec<PieSlice>,
    pub sankey_nodes: Vec<SankeyNode>, pub flows: Vec<SankeyFlow>,
    pub orientation: ChartOrientation,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Orientation { Horizontal, Vertical }

#[derive(Clone, Debug, PartialEq)]
pub struct LegendEntry { pub color: String, pub label: String }

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedLabel { pub x: f64, pub y: f64, pub text: String }

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutedChartItem {
    AxisSpine { x1: f64, y1: f64, x2: f64, y2: f64, orientation: Orientation },
    AxisTick { x: f64, y: f64, label: String, orientation: Orientation },
    GridLine { x1: f64, y1: f64, x2: f64, y2: f64 },
    Bar { x: f64, y: f64, width: f64, height: f64, color: String },
    LinePath { points: Vec<Point>, color: String },
    PieArc { cx: f64, cy: f64, r: f64, start_angle: f64, end_angle: f64, color: String, label: String },
    SankeyBand { from_x: f64, from_y: f64, to_x: f64, to_y: f64, width: f64, color: String },
    DataLabel { x: f64, y: f64, text: String },
    Legend { x: f64, y: f64, entries: Vec<LegendEntry> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedChartDiagram { pub width: f64, pub height: f64, pub title_box: Option<LayoutedLabel>, pub items: Vec<LayoutedChartItem> }

// STRUCTURAL FAMILY
#[derive(Clone, Debug, PartialEq)]
pub enum StructuralKind { Class, Er, C4 }

#[derive(Clone, Debug, PartialEq)]
pub enum StructuralNodeKind { Class, Interface, Abstract, Enum, Entity }
impl Default for StructuralNodeKind { fn default() -> Self { StructuralNodeKind::Class } }

#[derive(Clone, Debug, PartialEq)]
pub enum CompartmentKind { Header, Fields, Methods, Values }

#[derive(Clone, Debug, PartialEq)]
pub struct Compartment { pub kind: CompartmentKind, pub entries: Vec<String> }

#[derive(Clone, Debug, PartialEq)]
pub struct StructuralNode { pub id: String, pub label: String, pub stereotype: Option<String>, pub node_kind: StructuralNodeKind, pub compartments: Vec<Compartment> }

#[derive(Clone, Debug, PartialEq)]
pub enum RelKind { Inheritance, Realization, Composition, Aggregation, Association, Dependency, Link }

#[derive(Clone, Debug, PartialEq)]
pub struct StructuralRelationship { pub from: String, pub to: String, pub kind: RelKind, pub from_mult: Option<String>, pub to_mult: Option<String>, pub label: Option<String> }

#[derive(Clone, Debug, PartialEq)]
pub struct StructuralDiagram { pub kind: StructuralKind, pub title: Option<String>, pub nodes: Vec<StructuralNode>, pub relationships: Vec<StructuralRelationship> }

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedCompartment { pub y_offset: f64, pub height: f64, pub rows: Vec<String> }

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedStructuralNode { pub id: String, pub x: f64, pub y: f64, pub width: f64, pub height: f64, pub header: String, pub stereotype: Option<String>, pub compartments: Vec<LayoutedCompartment> }

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedStructuralRelationship { pub from_id: String, pub to_id: String, pub kind: RelKind, pub points: Vec<Point>, pub from_mult: Option<String>, pub to_mult: Option<String>, pub label: Option<(Point, String)> }

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedStructuralDiagram { pub width: f64, pub height: f64, pub nodes: Vec<LayoutedStructuralNode>, pub relationships: Vec<LayoutedStructuralRelationship> }

// TEMPORAL FAMILY
#[derive(Clone, Debug, PartialEq)]
pub enum TemporalKind { Gantt, Git }

#[derive(Clone, Debug, PartialEq)]
pub enum TaskStart { Date(String), After(String) }

#[derive(Clone, Debug, PartialEq)]
pub enum TaskStatus { Normal, Done, Active, Crit, Milestone }
impl Default for TaskStatus { fn default() -> Self { TaskStatus::Normal } }

#[derive(Clone, Debug, PartialEq)]
pub struct GanttTask { pub id: String, pub label: String, pub start: TaskStart, pub duration_days: f64, pub status: TaskStatus, pub dependencies: Vec<String> }

#[derive(Clone, Debug, PartialEq)]
pub struct GanttSection { pub label: Option<String>, pub tasks: Vec<GanttTask> }

#[derive(Clone, Debug, PartialEq)]
pub struct GanttDiagram { pub date_format: String, pub sections: Vec<GanttSection> }

#[derive(Clone, Debug, PartialEq)]
pub struct GitBranch { pub name: String }

#[derive(Clone, Debug, PartialEq)]
pub enum GitEvent {
    Commit { id: Option<String>, message: Option<String>, tag: Option<String>, branch: String },
    Checkout { branch: String },
    Merge { from: String, id: Option<String>, tag: Option<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GitDiagram { pub direction: DiagramDirection, pub branches: Vec<GitBranch>, pub events: Vec<GitEvent> }

#[derive(Clone, Debug, PartialEq)]
pub enum TemporalBody { Gantt(GanttDiagram), Git(GitDiagram) }

#[derive(Clone, Debug, PartialEq)]
pub struct TemporalDiagram { pub kind: TemporalKind, pub title: Option<String>, pub body: TemporalBody }

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutedTemporalItem {
    TimeAxisSpine { x1: f64, y1: f64, x2: f64, y2: f64 },
    TimeAxisTick { x: f64, y: f64, label: String },
    SectionHeader { x: f64, y: f64, width: f64, height: f64, label: String },
    TaskBar { x: f64, y: f64, width: f64, height: f64, status: TaskStatus, label: String },
    MilestoneMarker { x: f64, y: f64, label: String },
    TodayMarker { x: f64, y1: f64, y2: f64 },
    BranchLane { y: f64, color: String, label: String },
    CommitNode { x: f64, y: f64, id: String, message: Option<String>, tag: Option<String> },
    MergeArc { from_x: f64, from_y: f64, to_x: f64, to_y: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedTemporalDiagram { pub width: f64, pub height: f64, pub items: Vec<LayoutedTemporalItem> }

// GEOMETRIC FAMILY
#[derive(Clone, Debug, PartialEq)]
pub enum TextAlign { Left, Center, Right }
impl Default for TextAlign { fn default() -> Self { TextAlign::Center } }

#[derive(Clone, Debug, PartialEq)]
pub enum GeoElement {
    Box { id: String, x: f64, y: f64, w: f64, h: f64, corner_radius: f64, label: Option<String>, fill: Option<String>, stroke: Option<String> },
    Circle { id: String, cx: f64, cy: f64, r: f64, label: Option<String>, fill: Option<String>, stroke: Option<String> },
    Line { id: String, x1: f64, y1: f64, x2: f64, y2: f64, arrow_end: bool, arrow_start: bool, stroke: Option<String> },
    Arc { id: String, cx: f64, cy: f64, r: f64, start_deg: f64, end_deg: f64, stroke: Option<String> },
    Text { id: String, x: f64, y: f64, text: String, align: TextAlign },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeometricDiagram { pub title: Option<String>, pub width: Option<f64>, pub height: Option<f64>, pub elements: Vec<GeoElement> }

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedGeometricDiagram { pub width: f64, pub height: f64, pub elements: Vec<GeoElement> }

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn version_is_0_2_0() { assert_eq!(VERSION, "0.2.0"); }
    #[test] fn default_direction_is_tb() { assert_eq!(DiagramDirection::default(), DiagramDirection::Tb); }
    #[test] fn default_shape_is_rounded_rect() { assert_eq!(DiagramShape::default(), DiagramShape::RoundedRect); }
    #[test] fn resolve_style_none_gives_defaults() {
        let s = resolve_style(None);
        assert_eq!(s.fill, "#eff6ff"); assert_eq!(s.stroke, "#2563eb");
    }
    #[test] fn resolve_style_partial_override() {
        let style = DiagramStyle { fill: Some("#ff0000".to_string()), ..Default::default() };
        let s = resolve_style(Some(&style));
        assert_eq!(s.fill, "#ff0000"); assert_eq!(s.stroke, "#2563eb");
    }
    #[test] fn resolve_style_with_base_overrides_base() {
        let base = ResolvedDiagramStyle { fill:"none".into(), stroke:"#4b5563".into(), stroke_width:2.0, text_color:"#374151".into(), font_size:12.0, corner_radius:0.0 };
        let s = resolve_style_with_base(None, base); assert_eq!(s.fill, "none");
    }
    #[test] fn graph_diagram_builds_correctly() {
        let node = GraphNode { id:"A".into(), label:DiagramLabel::new("Node A"), shape:None, style:None };
        let edge = GraphEdge { id:None, from:"A".into(), to:"B".into(), label:None, kind:EdgeKind::Directed, style:None };
        let d = GraphDiagram { direction:DiagramDirection::Lr, title:Some("G".into()), nodes:vec![node], edges:vec![edge] };
        assert_eq!(d.nodes.len(), 1); assert_eq!(d.edges.len(), 1);
    }
    #[test] fn diagram_label_new() { assert_eq!(DiagramLabel::new("hello").text, "hello"); }
    #[test] fn chart_diagram_xy_builds() {
        let d = ChartDiagram { title:None, kind:ChartKind::Xy, x_axis:None, y_axis:None,
            series:vec![ChartSeries{kind:SeriesKind::Bar,label:None,data:vec![40.0,60.0]}],
            slices:vec![], sankey_nodes:vec![], flows:vec![], orientation:ChartOrientation::Vertical };
        assert_eq!(d.series[0].data.len(), 2);
    }
    #[test] fn structural_diagram_builds() {
        let node = StructuralNode { id:"A".into(), label:"A".into(), stereotype:None, node_kind:StructuralNodeKind::Class, compartments:vec![] };
        let d = StructuralDiagram { kind:StructuralKind::Class, title:None, nodes:vec![node], relationships:vec![] };
        assert_eq!(d.nodes[0].id, "A");
    }
    #[test] fn gantt_diagram_builds() {
        let task = GanttTask { id:"t1".into(), label:"D".into(), start:TaskStart::Date("2026-01-01".into()), duration_days:5.0, status:TaskStatus::Done, dependencies:vec![] };
        let d = TemporalDiagram { kind:TemporalKind::Gantt, title:None, body:TemporalBody::Gantt(GanttDiagram { date_format:"YYYY-MM-DD".into(), sections:vec![GanttSection{label:None,tasks:vec![task]}] }) };
        if let TemporalBody::Gantt(ref g) = d.body { assert_eq!(g.sections[0].tasks[0].id, "t1"); }
    }
    #[test] fn geometric_diagram_builds() {
        let d = GeometricDiagram { title:None, width:Some(400.0), height:Some(200.0),
            elements:vec![GeoElement::Box{id:"a".into(),x:10.0,y:10.0,w:100.0,h:50.0,corner_radius:0.0,label:None,fill:None,stroke:None}] };
        assert_eq!(d.width, Some(400.0));
    }
}
