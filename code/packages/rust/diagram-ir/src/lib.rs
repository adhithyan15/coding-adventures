//! diagram-ir v0.42.0 - DG00/DG04 semantic IR

pub const VERSION: &str = "0.63.0";

#[derive(Clone, Debug, PartialEq, Default)]
pub enum DiagramDirection {
    #[default]
    Tb,
    Lr,
    Rl,
    Bt,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum DiagramShape {
    Rect,
    Bar,
    #[default]
    RoundedRect,
    Ellipse,
    Diamond,
    Note,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiagramLabel {
    pub text: String,
}
impl DiagramLabel {
    pub fn new(text: impl Into<String>) -> Self {
        DiagramLabel { text: text.into() }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DiagramStyle {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f64>,
    pub text_color: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<u16>,
    pub font_italic: Option<bool>,
    pub font_family: Option<String>,
    pub corner_radius: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedDiagramStyle {
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f64,
    pub text_color: String,
    pub font_size: f64,
    pub font_weight: u16,
    pub font_italic: bool,
    pub font_family: String,
    pub corner_radius: f64,
}
impl Default for ResolvedDiagramStyle {
    fn default() -> Self {
        ResolvedDiagramStyle {
            fill: "#eff6ff".into(),
            stroke: "#2563eb".into(),
            stroke_width: 2.0,
            text_color: "#1e40af".into(),
            font_size: 14.0,
            font_weight: 400,
            font_italic: false,
            font_family: "Helvetica".into(),
            corner_radius: 8.0,
        }
    }
}
pub fn resolve_style(style: Option<&DiagramStyle>) -> ResolvedDiagramStyle {
    resolve_style_with_base(style, ResolvedDiagramStyle::default())
}
pub fn resolve_style_with_base(
    style: Option<&DiagramStyle>,
    base: ResolvedDiagramStyle,
) -> ResolvedDiagramStyle {
    match style {
        None => base,
        Some(s) => ResolvedDiagramStyle {
            fill: s.fill.clone().unwrap_or(base.fill),
            stroke: s.stroke.clone().unwrap_or(base.stroke),
            stroke_width: s.stroke_width.unwrap_or(base.stroke_width),
            text_color: s.text_color.clone().unwrap_or(base.text_color),
            font_size: s.font_size.unwrap_or(base.font_size),
            font_weight: s.font_weight.unwrap_or(base.font_weight),
            font_italic: s.font_italic.unwrap_or(base.font_italic),
            font_family: s.font_family.clone().unwrap_or(base.font_family),
            corner_radius: s.corner_radius.unwrap_or(base.corner_radius),
        },
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EdgeKind {
    Directed,
    Undirected,
    NoteAssociation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub label: DiagramLabel,
    pub shape: Option<DiagramShape>,
    pub style: Option<DiagramStyle>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdge {
    pub id: Option<String>,
    pub from: String,
    pub to: String,
    pub label: Option<DiagramLabel>,
    pub kind: EdgeKind,
    pub style: Option<DiagramStyle>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphLink {
    pub node_id: String,
    pub url: String,
    pub tooltip: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphGroup {
    pub id: String,
    pub label: DiagramLabel,
    pub parent_id: Option<String>,
    pub node_ids: Vec<String>,
    pub regions: Vec<Vec<String>>,
    pub direction: Option<DiagramDirection>,
    pub style: Option<DiagramStyle>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphDiagram {
    pub direction: DiagramDirection,
    pub requested_width: Option<f64>,
    pub hide_empty_descriptions: bool,
    pub title: Option<String>,
    pub accessibility_title: Option<String>,
    pub accessibility_description: Option<String>,
    pub links: Vec<GraphLink>,
    pub groups: Vec<GraphGroup>,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedGraphNode {
    pub id: String,
    pub label: DiagramLabel,
    pub shape: DiagramShape,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub style: ResolvedDiagramStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedGraphEdge {
    pub id: Option<String>,
    pub from_node_id: String,
    pub to_node_id: String,
    pub kind: EdgeKind,
    pub points: Vec<Point>,
    pub label: Option<DiagramLabel>,
    pub label_position: Option<Point>,
    pub style: ResolvedDiagramStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedGraphGroup {
    pub id: String,
    pub label: DiagramLabel,
    pub parent_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub divider_y: Vec<f64>,
    pub direction: Option<DiagramDirection>,
    pub style: ResolvedDiagramStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedGraphDiagram {
    pub direction: DiagramDirection,
    pub requested_width: Option<f64>,
    pub hide_empty_descriptions: bool,
    pub title: Option<String>,
    pub accessibility_title: Option<String>,
    pub accessibility_description: Option<String>,
    pub links: Vec<GraphLink>,
    pub groups: Vec<LayoutedGraphGroup>,
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<LayoutedGraphNode>,
    pub edges: Vec<LayoutedGraphEdge>,
}

// SEQUENCE FAMILY
#[derive(Clone, Debug, PartialEq)]
pub enum SequenceParticipantKind {
    Participant,
    Actor,
    Boundary,
    Control,
    Entity,
    Database,
    Collections,
    Queue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SequenceParticipant {
    pub id: String,
    pub label: DiagramLabel,
    pub label_wrap: SequenceTextWrap,
    pub kind: SequenceParticipantKind,
    pub style: Option<DiagramStyle>,
    pub group_id: Option<String>,
    pub links: Vec<SequenceLink>,
    pub properties: Vec<SequenceProperty>,
    /// Host document element whose JSON supplies additional links and properties.
    pub details_reference: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SequenceLink {
    pub label: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SequenceProperty {
    pub name: String,
    /// Canonical JSON preserves Mermaid's arbitrary property value types.
    pub value_json: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SequenceParticipantGroup {
    pub id: String,
    pub label: Option<String>,
    pub label_wrap: SequenceTextWrap,
    pub fill: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SequenceLineStyle {
    Solid,
    Dotted,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SequenceArrowhead {
    Open,
    Filled,
    Cross,
    Point,
    FilledTop,
    FilledBottom,
    StickTop,
    StickBottom,
    ReverseFilledTop,
    ReverseFilledBottom,
    ReverseStickTop,
    ReverseStickBottom,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SequenceCentralConnection {
    None,
    Source,
    Destination,
    Both,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SequenceNotePlacement {
    LeftOf,
    RightOf,
    Over,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum SequenceTextWrap {
    #[default]
    Default,
    Wrap,
    NoWrap,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SequenceBlockKind {
    Loop,
    Rect,
    Opt,
    Alt,
    Par,
    ParOver,
    Critical,
    Break,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SequenceEvent {
    AutoNumber {
        visible: bool,
        start: Option<f64>,
        step: Option<f64>,
    },
    Message {
        from: String,
        to: String,
        label: String,
        wrap: SequenceTextWrap,
        line_style: SequenceLineStyle,
        arrowhead: SequenceArrowhead,
        bidirectional: bool,
        central_connection: SequenceCentralConnection,
        activate: bool,
        deactivate: bool,
    },
    Activation {
        participant: String,
        active: bool,
    },
    ParticipantCreated {
        participant: String,
    },
    ParticipantDestroyed {
        participant: String,
    },
    Note {
        participants: Vec<String>,
        placement: SequenceNotePlacement,
        text: String,
        wrap: SequenceTextWrap,
    },
    BlockStart {
        kind: SequenceBlockKind,
        label: String,
        wrap: SequenceTextWrap,
        fill: Option<String>,
    },
    BlockBranch {
        label: String,
        wrap: SequenceTextWrap,
    },
    BlockEnd {
        kind: SequenceBlockKind,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SequenceDiagram {
    pub title: Option<String>,
    pub accessibility_title: Option<String>,
    pub accessibility_description: Option<String>,
    pub auto_number: bool,
    pub auto_number_start: f64,
    pub auto_number_step: f64,
    pub participants: Vec<SequenceParticipant>,
    pub participant_groups: Vec<SequenceParticipantGroup>,
    pub events: Vec<SequenceEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutedSequenceItem {
    ParticipantGroup {
        id: String,
        label: Option<String>,
        label_height: f64,
        fill: Option<String>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    ParticipantBox {
        id: String,
        label: String,
        label_height: f64,
        mirrored: bool,
        kind: SequenceParticipantKind,
        links: Vec<SequenceLink>,
        properties: Vec<SequenceProperty>,
        details_reference: Option<String>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    Lifeline {
        participant: String,
        x: f64,
        y1: f64,
        y2: f64,
    },
    Message {
        from_x: f64,
        to_x: f64,
        y: f64,
        label: String,
        label_height: f64,
        line_style: SequenceLineStyle,
        arrowhead: SequenceArrowhead,
        bidirectional: bool,
        central_connection: SequenceCentralConnection,
        number: Option<f64>,
    },
    Activation {
        participant: String,
        x: f64,
        y1: f64,
        y2: f64,
    },
    Destruction {
        participant: String,
        x: f64,
        y: f64,
    },
    Note {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        text: String,
    },
    BlockFrame {
        kind: SequenceBlockKind,
        label: String,
        label_height: f64,
        fill: Option<String>,
        depth: usize,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    BlockDivider {
        label: String,
        label_height: f64,
        x: f64,
        y: f64,
        width: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedSequenceDiagram {
    pub width: f64,
    pub height: f64,
    pub title: Option<String>,
    pub accessibility_title: Option<String>,
    pub accessibility_description: Option<String>,
    pub items: Vec<LayoutedSequenceItem>,
}

// CHART FAMILY
#[derive(Clone, Debug, PartialEq)]
pub enum ChartKind {
    Xy,
    Pie,
    Sankey,
    Quadrant,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum ChartOrientation {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AxisKind {
    Categorical,
    Numeric,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Axis {
    pub kind: AxisKind,
    pub title: Option<String>,
    pub categories: Vec<String>,
    pub min: f64,
    pub max: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SeriesKind {
    Bar,
    Line,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartSeries {
    pub kind: SeriesKind,
    pub label: Option<String>,
    pub data: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PieSlice {
    pub label: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SankeyNode {
    pub id: String,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SankeyFlow {
    pub source: String,
    pub target: String,
    pub weight: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuadrantPoint {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub radius: Option<f64>,
    pub color: Option<String>,
    pub stroke_color: Option<String>,
    pub stroke_width: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct QuadrantConfig {
    pub chart_width: Option<f64>,
    pub chart_height: Option<f64>,
    pub x_axis_position: Option<String>,
    pub y_axis_position: Option<String>,
    pub point_radius: Option<f64>,
    pub quadrant_padding: Option<f64>,
    pub internal_border_width: Option<f64>,
    pub external_border_width: Option<f64>,
    pub title_font_size: Option<f64>,
    pub title_padding: Option<f64>,
    pub x_axis_label_font_size: Option<f64>,
    pub x_axis_label_padding: Option<f64>,
    pub y_axis_label_font_size: Option<f64>,
    pub y_axis_label_padding: Option<f64>,
    pub quadrant_label_font_size: Option<f64>,
    pub quadrant_text_top_padding: Option<f64>,
    pub point_label_font_size: Option<f64>,
    pub point_text_padding: Option<f64>,
    pub quadrant_fills: [Option<String>; 4],
    pub quadrant_text_fills: [Option<String>; 4],
    pub point_fill: Option<String>,
    pub point_text_fill: Option<String>,
    pub x_axis_text_fill: Option<String>,
    pub y_axis_text_fill: Option<String>,
    pub internal_border_stroke_fill: Option<String>,
    pub external_border_stroke_fill: Option<String>,
    pub title_fill: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartDiagram {
    pub title: Option<String>,
    pub accessibility_title: Option<String>,
    pub accessibility_description: Option<String>,
    pub kind: ChartKind,
    pub show_data: bool,
    pub x_axis: Option<Axis>,
    pub y_axis: Option<Axis>,
    pub series: Vec<ChartSeries>,
    pub slices: Vec<PieSlice>,
    pub sankey_nodes: Vec<SankeyNode>,
    pub flows: Vec<SankeyFlow>,
    pub quadrant_labels: [Option<String>; 4],
    pub quadrant_points: Vec<QuadrantPoint>,
    pub quadrant_config: QuadrantConfig,
    pub orientation: ChartOrientation,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegendEntry {
    pub color: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedLabel {
    pub x: f64,
    pub y: f64,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutedChartItem {
    AxisSpine {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        orientation: Orientation,
    },
    AxisTick {
        x: f64,
        y: f64,
        label: String,
        orientation: Orientation,
    },
    GridLine {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Bar {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: String,
    },
    LinePath {
        points: Vec<Point>,
        color: String,
    },
    PieArc {
        cx: f64,
        cy: f64,
        r: f64,
        start_angle: f64,
        end_angle: f64,
        color: String,
        label: String,
    },
    SankeyBand {
        from_x: f64,
        from_y: f64,
        to_x: f64,
        to_y: f64,
        width: f64,
        color: String,
    },
    SankeyNode {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: String,
        label: String,
    },
    QuadrantRegion {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: String,
        label: Option<String>,
        label_font_size: Option<f64>,
        label_top_padding: f64,
        label_color: String,
    },
    QuadrantBorder {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        internal_color: String,
        external_color: String,
        internal_width: f64,
        external_width: f64,
    },
    ScatterPoint {
        x: f64,
        y: f64,
        radius: f64,
        color: String,
        stroke_color: String,
        stroke_width: f64,
        label: String,
        label_font_size: Option<f64>,
        label_padding: f64,
        label_color: String,
    },
    DataLabel {
        x: f64,
        y: f64,
        text: String,
        font_size: Option<f64>,
        color: Option<String>,
    },
    Legend {
        x: f64,
        y: f64,
        entries: Vec<LegendEntry>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedChartDiagram {
    pub width: f64,
    pub height: f64,
    pub accessibility_title: Option<String>,
    pub accessibility_description: Option<String>,
    pub title_box: Option<LayoutedLabel>,
    pub items: Vec<LayoutedChartItem>,
}

// STRUCTURAL FAMILY
#[derive(Clone, Debug, PartialEq)]
pub enum StructuralKind {
    Class,
    Er,
    C4,
    Requirement,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum StructuralNodeKind {
    #[default]
    Class,
    Interface,
    Abstract,
    Enum,
    Entity,
    Requirement,
    Element,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CompartmentKind {
    Header,
    Fields,
    Methods,
    Values,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Compartment {
    pub kind: CompartmentKind,
    pub entries: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RequirementRisk {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum RequirementKind {
    #[default]
    Requirement,
    Functional,
    Interface,
    Performance,
    Physical,
    DesignConstraint,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RequirementVerifyMethod {
    Analysis,
    Inspection,
    Test,
    Demonstration,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct RequirementMetadata {
    pub kind: RequirementKind,
    pub external_id: Option<String>,
    pub text: Option<String>,
    pub risk: Option<RequirementRisk>,
    pub verify_method: Option<RequirementVerifyMethod>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct RequirementElementMetadata {
    pub element_type: Option<String>,
    pub document_reference: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StructuralNodeMetadata {
    Requirement(RequirementMetadata),
    RequirementElement(RequirementElementMetadata),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructuralNode {
    pub id: String,
    pub label: String,
    pub stereotype: Option<String>,
    pub node_kind: StructuralNodeKind,
    pub metadata: Option<StructuralNodeMetadata>,
    pub style: Option<DiagramStyle>,
    pub compartments: Vec<Compartment>,
    pub parent_group: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructuralGroup {
    pub id: String,
    pub label: String,
    pub stereotype: Option<String>,
    pub parent_group: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RelKind {
    Inheritance,
    Realization,
    Composition,
    Aggregation,
    Association,
    Dependency,
    Link,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructuralRelationship {
    pub from: String,
    pub to: String,
    pub kind: RelKind,
    pub from_mult: Option<String>,
    pub to_mult: Option<String>,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructuralDiagram {
    pub kind: StructuralKind,
    pub title: Option<String>,
    pub accessibility_title: Option<String>,
    pub accessibility_description: Option<String>,
    pub direction: Option<DiagramDirection>,
    pub nodes: Vec<StructuralNode>,
    pub groups: Vec<StructuralGroup>,
    pub relationships: Vec<StructuralRelationship>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedCompartment {
    pub y_offset: f64,
    pub height: f64,
    pub rows: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedStructuralNode {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub header: String,
    pub stereotype: Option<String>,
    pub style: ResolvedDiagramStyle,
    pub compartments: Vec<LayoutedCompartment>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedStructuralGroup {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub label: String,
    pub stereotype: Option<String>,
    pub parent_group: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedStructuralRelationship {
    pub from_id: String,
    pub to_id: String,
    pub kind: RelKind,
    pub points: Vec<Point>,
    pub from_mult: Option<String>,
    pub to_mult: Option<String>,
    pub label: Option<(Point, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedStructuralDiagram {
    pub width: f64,
    pub height: f64,
    pub accessibility_title: Option<String>,
    pub accessibility_description: Option<String>,
    pub groups: Vec<LayoutedStructuralGroup>,
    pub nodes: Vec<LayoutedStructuralNode>,
    pub relationships: Vec<LayoutedStructuralRelationship>,
}

// TEMPORAL FAMILY
#[derive(Clone, Debug, PartialEq)]
pub enum TemporalKind {
    Gantt,
    Git,
    Journey,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JourneyTask {
    pub label: String,
    pub score: u8,
    pub people: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JourneySection {
    pub label: String,
    pub tasks: Vec<JourneyTask>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct JourneyConfig {
    pub diagram_margin_x: Option<f64>,
    pub diagram_margin_y: Option<f64>,
    pub task_width: Option<f64>,
    pub task_height: Option<f64>,
    pub task_margin: Option<f64>,
    pub task_font_size: Option<f64>,
    pub task_font_family: Option<String>,
    pub title_font_size: Option<f64>,
    pub title_font_family: Option<String>,
    pub title_color: Option<String>,
    pub actor_colors: Vec<String>,
    pub section_fills: Vec<String>,
    pub section_colors: Vec<String>,
    pub left_margin: Option<f64>,
    pub max_label_width: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JourneyDiagram {
    pub accessibility_title: Option<String>,
    pub accessibility_description: Option<String>,
    pub config: JourneyConfig,
    pub sections: Vec<JourneySection>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TaskStart {
    Date(String),
    After(String),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum TaskStatus {
    #[default]
    Normal,
    Done,
    Active,
    Crit,
    Milestone,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GanttTask {
    pub id: String,
    pub label: String,
    pub start: TaskStart,
    pub duration_days: f64,
    pub status: TaskStatus,
    pub dependencies: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GanttSection {
    pub label: Option<String>,
    pub tasks: Vec<GanttTask>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GanttDiagram {
    pub date_format: String,
    pub sections: Vec<GanttSection>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GitBranch {
    pub name: String,
    pub order: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum GitCommitType {
    #[default]
    Normal,
    Reverse,
    Highlight,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GitCommitSymbol {
    Normal,
    Reverse,
    Highlight,
    Merge,
    CherryPick,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GitEvent {
    Commit {
        id: Option<String>,
        resolved_id: String,
        parents: Vec<String>,
        message: Option<String>,
        tags: Vec<String>,
        branch: String,
        type_: GitCommitType,
    },
    Checkout {
        branch: String,
    },
    Merge {
        from: String,
        id: Option<String>,
        resolved_id: String,
        parents: Vec<String>,
        tags: Vec<String>,
        type_: GitCommitType,
    },
    CherryPick {
        id: String,
        resolved_id: String,
        parents: Vec<String>,
        tags: Vec<String>,
        parent: Option<String>,
        branch: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GitDiagram {
    pub title: Option<String>,
    pub accessibility_title: Option<String>,
    pub accessibility_description: Option<String>,
    pub direction: DiagramDirection,
    pub branches: Vec<GitBranch>,
    pub events: Vec<GitEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TemporalBody {
    Gantt(GanttDiagram),
    Git(GitDiagram),
    Journey(Box<JourneyDiagram>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TemporalDiagram {
    pub kind: TemporalKind,
    pub title: Option<String>,
    pub body: TemporalBody,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutedTemporalItem {
    TemporalTitle {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        label: String,
    },
    JourneyTitle {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        label: String,
        font_size: Option<f64>,
        font_family: Option<String>,
        color: Option<String>,
    },
    JourneySection {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        label: String,
        fill: String,
        text_color: String,
    },
    TimeAxisSpine {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    TimeAxisTick {
        x: f64,
        y: f64,
        label: String,
    },
    SectionHeader {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        label: String,
    },
    TaskBar {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        status: TaskStatus,
        label: String,
    },
    MilestoneMarker {
        x: f64,
        y: f64,
        label: String,
    },
    TodayMarker {
        x: f64,
        y1: f64,
        y2: f64,
    },
    BranchLane {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        label_x: f64,
        label_y: f64,
        label_width: f64,
        label_height: f64,
        color: String,
        label: String,
    },
    CommitNode {
        x: f64,
        y: f64,
        id: String,
        message: Option<String>,
        tags: Vec<String>,
        symbol: GitCommitSymbol,
    },
    GitHistoryArc {
        from_x: f64,
        from_y: f64,
        to_x: f64,
        to_y: f64,
    },
    JourneyTask {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        score_y: f64,
        score: u8,
        label: String,
        people: Vec<String>,
        person_colors: Vec<String>,
        font_size: Option<f64>,
        font_family: Option<String>,
        fill: String,
        text_color: String,
    },
    JourneyActor {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: String,
        label: String,
    },
    JourneyActivityLine {
        x1: f64,
        y: f64,
        x2: f64,
    },
    JourneyTaskLine {
        x: f64,
        y1: f64,
        y2: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedTemporalDiagram {
    pub width: f64,
    pub height: f64,
    pub accessibility_title: Option<String>,
    pub accessibility_description: Option<String>,
    pub items: Vec<LayoutedTemporalItem>,
}

// GEOMETRIC FAMILY
#[derive(Clone, Debug, PartialEq, Default)]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GeoElement {
    Box {
        id: String,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        corner_radius: f64,
        label: Option<String>,
        fill: Option<String>,
        stroke: Option<String>,
    },
    Circle {
        id: String,
        cx: f64,
        cy: f64,
        r: f64,
        label: Option<String>,
        fill: Option<String>,
        stroke: Option<String>,
    },
    Line {
        id: String,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        arrow_end: bool,
        arrow_start: bool,
        stroke: Option<String>,
    },
    Arc {
        id: String,
        cx: f64,
        cy: f64,
        r: f64,
        start_deg: f64,
        end_deg: f64,
        stroke: Option<String>,
    },
    Text {
        id: String,
        x: f64,
        y: f64,
        text: String,
        align: TextAlign,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeometricDiagram {
    pub title: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub elements: Vec<GeoElement>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutedGeometricDiagram {
    pub width: f64,
    pub height: f64,
    pub elements: Vec<GeoElement>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.63.0");
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
    }
    #[test]
    fn resolve_style_partial_override() {
        let style = DiagramStyle {
            fill: Some("#ff0000".to_string()),
            ..Default::default()
        };
        let s = resolve_style(Some(&style));
        assert_eq!(s.fill, "#ff0000");
        assert_eq!(s.stroke, "#2563eb");
    }
    #[test]
    fn resolve_style_with_base_overrides_base() {
        let base = ResolvedDiagramStyle {
            fill: "none".into(),
            stroke: "#4b5563".into(),
            stroke_width: 2.0,
            text_color: "#374151".into(),
            font_size: 12.0,
            font_weight: 400,
            font_italic: false,
            font_family: "Helvetica".into(),
            corner_radius: 0.0,
        };
        let s = resolve_style_with_base(None, base);
        assert_eq!(s.fill, "none");
    }
    #[test]
    fn graph_diagram_builds_correctly() {
        let node = GraphNode {
            id: "A".into(),
            label: DiagramLabel::new("Node A"),
            shape: None,
            style: None,
        };
        let edge = GraphEdge {
            id: None,
            from: "A".into(),
            to: "B".into(),
            label: None,
            kind: EdgeKind::Directed,
            style: None,
        };
        let d = GraphDiagram {
            direction: DiagramDirection::Lr,
            requested_width: None,
            hide_empty_descriptions: false,
            title: Some("G".into()),
            accessibility_title: None,
            accessibility_description: None,
            links: Vec::new(),
            groups: Vec::new(),
            nodes: vec![node],
            edges: vec![edge],
        };
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.edges.len(), 1);
    }
    #[test]
    fn diagram_label_new() {
        assert_eq!(DiagramLabel::new("hello").text, "hello");
    }
    #[test]
    fn chart_diagram_xy_builds() {
        let d = ChartDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            kind: ChartKind::Xy,
            show_data: false,
            x_axis: None,
            y_axis: None,
            series: vec![ChartSeries {
                kind: SeriesKind::Bar,
                label: None,
                data: vec![40.0, 60.0],
            }],
            slices: vec![],
            sankey_nodes: vec![],
            flows: vec![],
            quadrant_labels: [None, None, None, None],
            quadrant_points: vec![],
            quadrant_config: QuadrantConfig::default(),
            orientation: ChartOrientation::Vertical,
        };
        assert_eq!(d.series[0].data.len(), 2);
    }
    #[test]
    fn structural_diagram_builds() {
        let node = StructuralNode {
            id: "A".into(),
            label: "A".into(),
            stereotype: None,
            node_kind: StructuralNodeKind::Class,
            metadata: None,
            style: None,
            compartments: vec![],
            parent_group: None,
        };
        let d = StructuralDiagram {
            kind: StructuralKind::Class,
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            direction: None,
            nodes: vec![node],
            groups: vec![],
            relationships: vec![],
        };
        assert_eq!(d.nodes[0].id, "A");
    }
    #[test]
    fn gantt_diagram_builds() {
        let task = GanttTask {
            id: "t1".into(),
            label: "D".into(),
            start: TaskStart::Date("2026-01-01".into()),
            duration_days: 5.0,
            status: TaskStatus::Done,
            dependencies: vec![],
        };
        let d = TemporalDiagram {
            kind: TemporalKind::Gantt,
            title: None,
            body: TemporalBody::Gantt(GanttDiagram {
                date_format: "YYYY-MM-DD".into(),
                sections: vec![GanttSection {
                    label: None,
                    tasks: vec![task],
                }],
            }),
        };
        if let TemporalBody::Gantt(ref g) = d.body {
            assert_eq!(g.sections[0].tasks[0].id, "t1");
        }
    }
    #[test]
    fn geometric_diagram_builds() {
        let d = GeometricDiagram {
            title: None,
            width: Some(400.0),
            height: Some(200.0),
            elements: vec![GeoElement::Box {
                id: "a".into(),
                x: 10.0,
                y: 10.0,
                w: 100.0,
                h: 50.0,
                corner_radius: 0.0,
                label: None,
                fill: None,
                stroke: None,
            }],
        };
        assert_eq!(d.width, Some(400.0));
    }
}
