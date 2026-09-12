//! # diagram-to-paint
//!
//! DG03 — Converts a [`LayoutedGraphDiagram`] into a [`PaintScene`] that can
//! be rendered by any paint backend (Metal, SVG, Canvas, Direct2D …).
//!
//! ```text
//! LayoutedGraphDiagram  (pixel-positioned graph)
//!   → diagram-to-paint
//!       ├─ node shapes    → PaintRect / PaintEllipse / PaintPath  (geometry)
//!       ├─ edge paths     → PaintPath                             (geometry)
//!       └─ all text       → PositionedNode tree
//!                               → layout-to-paint (UI04)
//!                                   → PaintGlyphRun              (real shaping)
//!   → PaintScene         (renderable paint instructions)
//!   → PaintVM backend    (Metal, SVG, Canvas, Direct2D …)
//! ```
//!
//! Text rendering is **delegated to `layout-to-paint`** via a bridge of
//! `PositionedNode` values. Real glyph IDs are emitted (not Unicode codepoints),
//! so every paint backend — including `paint-metal`'s CoreText overlay — produces
//! correct, readable text.
//!
//! ## Painter's-algorithm order (back to front)
//!
//! 1. All edge lines and arrowheads.
//! 2. All node shapes (filled over edges so endpoints are hidden).
//! 3. All text (node labels + edge labels + title) via `layout-to-paint`.

pub const VERSION: &str = "0.63.0";

use std::collections::HashMap;

use diagram_ir::{
    DiagramShape, EdgeKind, GeoElement, GitCommitSymbol, LayoutedChartDiagram, LayoutedChartItem,
    EventModelEntityKind, LayoutedEventModelDiagram, LayoutedEventModelItem,
    LayoutedCynefinDiagram, LayoutedInfoDiagram, LayoutedIshikawaDiagram, LayoutedSwimlaneDiagram, LayoutedRailroadDiagram,
    LayoutedTreeViewDiagram, LayoutedTreemapDiagram, LayoutedVennDiagram, LayoutedWardleyDiagram,
    LayoutedGeometricDiagram, LayoutedGraphDiagram, LayoutedGraphEdge, LayoutedGraphNode,
    LayoutedBoardDiagram, LayoutedPacketDiagram,
    LayoutedSequenceDiagram, LayoutedSequenceItem, LayoutedStructuralDiagram,
    LayoutedTemporalDiagram, LayoutedTemporalItem, Orientation, Point, RelKind, SequenceArrowhead,
    SequenceBlockKind, SequenceCentralConnection, SequenceLineStyle, SequenceParticipantKind,
    GanttTaskTags, SequenceProperty, SwimlaneEdgeKind, TextAlign as GeoTextAlign, TreeViewNodeKind,
    RailroadElementKind,
};
use layout_ir::{Color, Content, FontSpec, PositionedNode, TextAlign, TextContent};
use layout_to_paint::{layout_to_paint, LayoutToPaintOptions};
use paint_instructions::{
    PaintBase, PaintEllipse, PaintGroup, PaintInstruction, PaintPath, PaintRect, PaintScene,
    PathCommand, StrokeCap, StrokeJoin,
};
use text_interfaces::{FontMetrics, FontResolver, TextShaper};

// ============================================================================
// Options
// ============================================================================

/// Rendering options for `diagram_to_paint`. The `shaper`, `metrics`, and
/// `resolver` must share the same font binding (`Handle` associated type).
pub struct DiagramToPaintOptions<'a, S, M, R>
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    /// Canvas background colour (RGBA).
    pub background: Color,
    /// Device pixel ratio — all coordinates are in logical pixels; the shaper
    /// scales to physical pixels internally.
    pub device_pixel_ratio: f64,
    /// Font for node labels and edge labels (default: Helvetica 14 pt 400).
    pub label_font: FontSpec,
    /// Font for the diagram title (default: Helvetica 18 pt 700).
    pub title_font: FontSpec,
    pub shaper: &'a S,
    pub metrics: &'a M,
    pub resolver: &'a R,
}

/// Lower Mermaid Info's fixed version label into backend-neutral glyphs.
pub fn diagram_to_paint_info<S, M, R>(diagram: &LayoutedInfoDiagram, options: &DiagramToPaintOptions<'_, S, M, R>) -> PaintScene
where S: TextShaper, M: FontMetrics<Handle = S::Handle>, R: FontResolver<Handle = S::Handle> {
    let mut font = options.title_font.clone(); font.size = diagram.font_size;
    let positioned = PositionedNode { x: 0.0, y: 0.0, width: diagram.width, height: diagram.height, id: None, content: None,
        children: vec![text_node_no_wrap(&diagram.label, diagram.x, diagram.y, 200.0, 44.0, font,
            Color { r: 0, g: 0, b: 0, a: 255 })], ext: HashMap::new() };
    let mut scene = layout_to_paint(&positioned, &LayoutToPaintOptions { width: diagram.width, height: diagram.height,
        background: options.background, device_pixel_ratio: options.device_pixel_ratio,
        shaper: options.shaper, metrics: options.metrics, resolver: options.resolver });
    scene.background = format!("rgb({},{},{})", options.background.r, options.background.g, options.background.b);
    scene
}

/// Lower a layouted treemap into backend-neutral paint instructions.
pub fn diagram_to_paint_treemap<S, M, R>(
    diagram: &LayoutedTreemapDiagram,
    options: &DiagramToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    const FILLS: &[&str] = &["#dbeafe", "#dcfce7", "#fef3c7", "#fee2e2", "#e0e7ff"];
    let mut instructions = Vec::new();
    let mut text_children = Vec::new();
    let text_color = Color { r: 15, g: 23, b: 42, a: 255 };

    if let Some(title) = &diagram.title {
        text_children.push(text_node(title, 8.0, 6.0, diagram.width - 16.0, 30.0, options.title_font.clone(), text_color));
    }
    for node in &diagram.nodes {
        if node.width <= 0.0 || node.height <= 0.0 {
            continue;
        }
        let color_index = node.class_selector.as_ref().map_or(node.depth, |class| {
            class.bytes().fold(node.depth, |hash, byte| hash.wrapping_mul(31).wrapping_add(byte as usize))
        });
        instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: node.x,
            y: node.y,
            width: node.width,
            height: node.height,
            fill: Some(FILLS[color_index % FILLS.len()].into()),
            stroke: Some("#475569".into()),
            stroke_width: Some(1.0),
            corner_radius: Some(3.0),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
        if node.width >= 44.0 && node.height >= 22.0 {
            let label = if node.height >= 42.0 {
                format!("{}\n{}", node.label, format_treemap_value(node.value))
            } else {
                node.label.clone()
            };
            text_children.push(text_node(
                &label,
                node.x + 6.0,
                node.y + 4.0,
                (node.width - 12.0).max(0.0),
                node.height.min(42.0),
                options.label_font.clone(),
                text_color,
            ));
        }
    }
    let text_root = PositionedNode {
        x: 0.0,
        y: 0.0,
        width: diagram.width,
        height: diagram.height,
        id: None,
        content: None,
        children: text_children,
        ext: HashMap::new(),
    };
    let text_scene = layout_to_paint(&text_root, &LayoutToPaintOptions {
        width: diagram.width,
        height: diagram.height,
        background: Color { r: 0, g: 0, b: 0, a: 0 },
        device_pixel_ratio: 1.0,
        shaper: options.shaper,
        metrics: options.metrics,
        resolver: options.resolver,
    });
    instructions.extend(text_scene.instructions);

    let mut metadata = HashMap::new();
    if let Some(title) = &diagram.accessibility_title {
        metadata.insert("accessibility.title".into(), title.clone());
    }
    if let Some(description) = &diagram.accessibility_description {
        metadata.insert("accessibility.description".into(), description.clone());
    }
    PaintScene {
        width: diagram.width,
        height: diagram.height,
        background: format!("rgb({},{},{})", options.background.r, options.background.g, options.background.b),
        instructions,
        id: None,
        metadata: (!metadata.is_empty()).then_some(metadata),
    }
}

fn format_treemap_value(value: f64) -> String {
    if value.fract() == 0.0 { format!("{value:.0}") } else { format!("{value:.2}") }
}

/// Lower a layouted TreeView into backend-neutral connectors, markers, and glyphs.
pub fn diagram_to_paint_treeview<S, M, R>(diagram: &LayoutedTreeViewDiagram, options: &DiagramToPaintOptions<'_, S, M, R>) -> PaintScene
where S: TextShaper, M: FontMetrics<Handle = S::Handle>, R: FontResolver<Handle = S::Handle> {
    let mut instructions = Vec::new();
    let mut text_children = Vec::new();
    let positions = diagram.nodes.iter().map(|node| (node.id.as_str(), (node.x, node.y))).collect::<HashMap<_, _>>();
    for node in &diagram.nodes {
        if let Some((parent_x, parent_y)) = node.parent_id.as_deref().and_then(|id| positions.get(id)).copied() {
            let joint_x = node.x - 16.0;
            instructions.push(PaintInstruction::Path(line_path(&[
                Point { x: parent_x + 7.0, y: parent_y + 14.0 },
                Point { x: joint_x, y: parent_y + 14.0 },
                Point { x: joint_x, y: node.y + 14.0 },
                Point { x: node.x, y: node.y + 14.0 },
            ], "#64748b", 1.5)));
        }
    }
    if let Some(title) = &diagram.title {
        text_children.push(text_node(title, 10.0, 5.0, diagram.width - 20.0, 30.0, options.title_font.clone(), Color { r: 15, g: 23, b: 42, a: 255 }));
    }
    for node in &diagram.nodes {
        if node.class_selector.as_deref() == Some("highlight") {
            instructions.push(PaintInstruction::Rect(PaintRect { base: PaintBase::default(), x: node.x - 5.0, y: node.y,
                width: node.width, height: node.height, fill: Some("#fff7d6".into()), stroke: Some("#f59e0b".into()),
                stroke_width: Some(1.0), corner_radius: Some(4.0), stroke_dash: None, stroke_dash_offset: None }));
        }
        let show_icon = node.icon.as_deref().is_some_and(|icon| icon != "none");
        if show_icon {
            let (fill, radius) = match node.kind { TreeViewNodeKind::Directory => ("#fbbf24", 3.0), TreeViewNodeKind::File => ("#bfdbfe", 1.0) };
            instructions.push(PaintInstruction::Rect(PaintRect { base: PaintBase::default(), x: node.x, y: node.y + 6.0,
                width: 16.0, height: 16.0, fill: Some(fill.into()), stroke: Some("#475569".into()), stroke_width: Some(1.0),
                corner_radius: Some(radius), stroke_dash: None, stroke_dash_offset: None }));
        }
        let text_x = node.x + if show_icon { 23.0 } else { 0.0 };
        let label_width = (node.label.chars().count() as f64 * options.label_font.size * 0.62 + 8.0).min(node.width * 0.6);
        text_children.push(text_node_no_wrap(&node.label, text_x, node.y + 2.0, label_width, 24.0,
            options.label_font.clone(), Color { r: 15, g: 23, b: 42, a: 255 }));
        if let Some(description) = &node.description {
            let description_width = (description.chars().count() as f64 * options.label_font.size * 0.58 + 8.0).min(node.width * 0.4);
            text_children.push(text_node_no_wrap(description, text_x + label_width + 10.0,
                node.y + 2.0, description_width, 24.0, options.label_font.clone(), Color { r: 71, g: 102, b: 85, a: 255 }));
        }
    }
    let text_scene = layout_to_paint(&PositionedNode { x: 0.0, y: 0.0, width: diagram.width, height: diagram.height,
        id: None, content: None, children: text_children, ext: HashMap::new() }, &LayoutToPaintOptions { width: diagram.width,
        height: diagram.height, background: Color { r: 0, g: 0, b: 0, a: 0 }, device_pixel_ratio: 1.0,
        shaper: options.shaper, metrics: options.metrics, resolver: options.resolver });
    instructions.extend(text_scene.instructions);
    let mut metadata = HashMap::new();
    if let Some(title) = &diagram.accessibility_title { metadata.insert("accessibility.title".into(), title.clone()); }
    if let Some(description) = &diagram.accessibility_description { metadata.insert("accessibility.description".into(), description.clone()); }
    PaintScene { width: diagram.width, height: diagram.height,
        background: format!("rgb({},{},{})", options.background.r, options.background.g, options.background.b),
        instructions, id: None, metadata: (!metadata.is_empty()).then_some(metadata) }
}

/// Lower Swimlane ownership bands, process nodes, and handoffs to portable paint.
pub fn diagram_to_paint_swimlane<S, M, R>(
    diagram: &LayoutedSwimlaneDiagram,
    options: &DiagramToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let mut instructions = Vec::new();
    let mut text_children = Vec::new();
    for (index, lane) in diagram.lanes.iter().enumerate() {
        let fill = if index % 2 == 0 { "#f8fafc" } else { "#eef6f8" };
        instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: lane.x,
            y: lane.y,
            width: lane.width,
            height: lane.height,
            fill: Some(fill.into()),
            stroke: Some("#78909c".into()),
            stroke_width: Some(1.5),
            corner_radius: Some(8.0),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
        text_children.push(text_node(
            &lane.label,
            lane.x + 8.0,
            lane.y + 7.0,
            lane.width - 16.0,
            26.0,
            options.title_font.clone(),
            Color {
                r: 38,
                g: 50,
                b: 56,
                a: 255,
            },
        ));
    }
    for edge in &diagram.edges {
        let mut path = line_path(
            &[edge.from.clone(), edge.to.clone()],
            "#455a64",
            if edge.kind == SwimlaneEdgeKind::Thick {
                3.5
            } else {
                1.8
            },
        );
        if edge.kind == SwimlaneEdgeKind::Dotted {
            path.stroke_dash = Some(vec![5.0, 5.0]);
        }
        instructions.push(PaintInstruction::Path(path));
        if edge.kind != SwimlaneEdgeKind::Undirected {
            instructions.push(PaintInstruction::Path(simple_arrowhead(
                &edge.from, &edge.to, "#455a64",
            )));
        }
        if let Some(label) = &edge.label {
            text_children.push(text_node(
                label,
                (edge.from.x + edge.to.x) / 2.0 - 52.0,
                (edge.from.y + edge.to.y) / 2.0 - 25.0,
                104.0,
                22.0,
                options.label_font.clone(),
                Color {
                    r: 55,
                    g: 71,
                    b: 79,
                    a: 255,
                },
            ));
        }
    }
    for node in &diagram.nodes {
        let fill = "#ffffff".to_string();
        let stroke = "#1565c0".to_string();
        let shape = match node.shape {
            DiagramShape::Ellipse => PaintInstruction::Ellipse(PaintEllipse {
                base: PaintBase::default(),
                cx: node.x + node.width / 2.0,
                cy: node.y + node.height / 2.0,
                rx: node.width / 2.0,
                ry: node.height / 2.0,
                fill: Some(fill),
                stroke: Some(stroke),
                stroke_width: Some(2.0),
                stroke_dash: None,
                stroke_dash_offset: None,
            }),
            DiagramShape::Diamond => {
                let cx = node.x + node.width / 2.0;
                let cy = node.y + node.height / 2.0;
                PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo { x: cx, y: node.y },
                        PathCommand::LineTo {
                            x: node.x + node.width,
                            y: cy,
                        },
                        PathCommand::LineTo {
                            x: cx,
                            y: node.y + node.height,
                        },
                        PathCommand::LineTo { x: node.x, y: cy },
                        PathCommand::Close,
                    ],
                    fill: Some(fill),
                    fill_rule: None,
                    stroke: Some(stroke),
                    stroke_width: Some(2.0),
                    stroke_cap: None,
                    stroke_join: Some(StrokeJoin::Round),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                })
            }
            _ => PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: node.x,
                y: node.y,
                width: node.width,
                height: node.height,
                fill: Some(fill),
                stroke: Some(stroke),
                stroke_width: Some(2.0),
                corner_radius: Some(if node.shape == DiagramShape::Rect {
                    2.0
                } else {
                    18.0
                }),
                stroke_dash: None,
                stroke_dash_offset: None,
            }),
        };
        instructions.push(shape);
        text_children.push(text_node(
            &node.label,
            node.x + 8.0,
            node.y + 10.0,
            node.width - 16.0,
            node.height - 16.0,
            options.label_font.clone(),
            Color {
                r: 13,
                g: 71,
                b: 161,
                a: 255,
            },
        ));
    }
    if let Some(title) = &diagram.title {
        text_children.push(text_node(
            title,
            10.0,
            5.0,
            diagram.width - 20.0,
            30.0,
            options.title_font.clone(),
            Color {
                r: 15,
                g: 23,
                b: 42,
                a: 255,
            },
        ));
    }
    let text_scene = layout_to_paint(
        &PositionedNode {
            x: 0.0,
            y: 0.0,
            width: diagram.width,
            height: diagram.height,
            id: None,
            content: None,
            children: text_children,
            ext: HashMap::new(),
        },
        &LayoutToPaintOptions {
            width: diagram.width,
            height: diagram.height,
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            device_pixel_ratio: 1.0,
            shaper: options.shaper,
            metrics: options.metrics,
            resolver: options.resolver,
        },
    );
    instructions.extend(text_scene.instructions);
    let mut metadata = HashMap::new();
    if let Some(title) = &diagram.accessibility_title {
        metadata.insert("accessibility.title".into(), title.clone());
    }
    if let Some(description) = &diagram.accessibility_description {
        metadata.insert("accessibility.description".into(), description.clone());
    }
    PaintScene {
        width: diagram.width,
        height: diagram.height,
        background: format!(
            "rgb({},{},{})",
            options.background.r, options.background.g, options.background.b
        ),
        instructions,
        id: None,
        metadata: (!metadata.is_empty()).then_some(metadata),
    }
}
/// Lower Railroad rules into backend-neutral paths, markers, boxes, and glyphs.
pub fn diagram_to_paint_railroad<S, M, R>(diagram: &LayoutedRailroadDiagram, options: &DiagramToPaintOptions<'_, S, M, R>) -> PaintScene
where S: TextShaper, M: FontMetrics<Handle = S::Handle>, R: FontResolver<Handle = S::Handle> {
    let mut instructions = Vec::new(); let mut text_children = Vec::new();
    if let Some(title) = &diagram.title { text_children.push(text_node(title, 10.0, 5.0, diagram.width - 20.0, 30.0,
        options.title_font.clone(), Color { r: 15, g: 23, b: 42, a: 255 })); }
    for rule in &diagram.rules {
        let center = rule.y + rule.height / 2.0;
        text_children.push(text_node_no_wrap(&rule.name, 10.0, center - 14.0, 92.0, 28.0,
            options.label_font.clone(), Color { r: 30, g: 64, b: 175, a: 255 }));
        instructions.push(PaintInstruction::Ellipse(PaintEllipse { base: PaintBase::default(), cx: 110.0, cy: center,
            rx: 5.0, ry: 5.0, fill: Some("#111827".into()), stroke: None, stroke_width: None, stroke_dash: None, stroke_dash_offset: None }));
        let end_x = rule.paths.iter().flat_map(|path| path.points.iter().map(|point| point.x)).fold(140.0, f64::max);
        instructions.push(PaintInstruction::Ellipse(PaintEllipse { base: PaintBase::default(), cx: end_x + 6.0, cy: center,
            rx: 5.0, ry: 5.0, fill: Some("#111827".into()), stroke: None, stroke_width: None, stroke_dash: None, stroke_dash_offset: None }));
        for path in &rule.paths { let mut paint = line_path(&path.points, "#334155", 1.8);
            if path.loopback { paint.stroke_dash = Some(vec![4.0, 3.0]); } instructions.push(PaintInstruction::Path(paint)); }
        for element in &rule.elements {
            let (fill, stroke, radius) = match element.kind { RailroadElementKind::Terminal => ("#fef3c7", "#92400e", 14.0),
                RailroadElementKind::NonTerminal => ("#ffffff", "#334155", 2.0), RailroadElementKind::Special => ("#f3e8ff", "#7e22ce", 7.0) };
            instructions.push(PaintInstruction::Rect(PaintRect { base: PaintBase::default(), x: element.x, y: element.y,
                width: element.width, height: element.height, fill: Some(fill.into()), stroke: Some(stroke.into()),
                stroke_width: Some(1.7), corner_radius: Some(radius), stroke_dash: None, stroke_dash_offset: None }));
            text_children.push(text_node_no_wrap(&element.label, element.x + 10.0, element.y + 7.0,
                element.width - 20.0, 24.0, options.label_font.clone(), Color { r: 15, g: 23, b: 42, a: 255 }));
        }
    }
    let text_scene = layout_to_paint(&PositionedNode { x: 0.0, y: 0.0, width: diagram.width, height: diagram.height,
        id: None, content: None, children: text_children, ext: HashMap::new() }, &LayoutToPaintOptions { width: diagram.width,
        height: diagram.height, background: Color { r: 0, g: 0, b: 0, a: 0 }, device_pixel_ratio: 1.0,
        shaper: options.shaper, metrics: options.metrics, resolver: options.resolver });
    instructions.extend(text_scene.instructions); let mut metadata = HashMap::new();
    if let Some(title) = &diagram.accessibility_title { metadata.insert("accessibility.title".into(), title.clone()); }
    if let Some(description) = &diagram.accessibility_description { metadata.insert("accessibility.description".into(), description.clone()); }
    PaintScene { width: diagram.width, height: diagram.height,
        background: format!("rgb({},{},{})", options.background.r, options.background.g, options.background.b),
        instructions, id: None, metadata: (!metadata.is_empty()).then_some(metadata) }
}

/// Lower Venn circle geometry into backend-neutral ellipses and glyph runs.
pub fn diagram_to_paint_venn<S, M, R>(diagram: &LayoutedVennDiagram, options: &DiagramToPaintOptions<'_, S, M, R>) -> PaintScene
where S: TextShaper, M: FontMetrics<Handle = S::Handle>, R: FontResolver<Handle = S::Handle> {
    const FILLS: &[&str] = &["#60a5fa", "#f59e0b", "#34d399", "#f472b6", "#a78bfa"];
    let mut instructions = Vec::new();
    for (index, circle) in diagram.circles.iter().enumerate() {
        let fill = circle.style.fill.clone().unwrap_or_else(|| FILLS[index % FILLS.len()].into());
        instructions.push(PaintInstruction::Ellipse(PaintEllipse {
            base: PaintBase::default(), cx: circle.cx, cy: circle.cy, rx: circle.radius, ry: circle.radius,
            fill: Some(with_opacity(&fill, circle.style.fill_opacity.unwrap_or(0.38))),
            stroke: Some(circle.style.stroke.clone().unwrap_or_else(|| "#334155".into())),
            stroke_width: Some(circle.style.stroke_width.unwrap_or(2.0)), stroke_dash: None, stroke_dash_offset: None,
        }));
    }
    let mut text_children = Vec::new();
    if let Some(title) = &diagram.title {
        text_children.push(text_node(title, 8.0, 5.0, diagram.width - 16.0, 28.0, options.title_font.clone(), Color { r: 15, g: 23, b: 42, a: 255 }));
    }
    for label in &diagram.labels {
        text_children.push(text_node(&label.text, label.x - 70.0, label.y - 12.0, 140.0, 26.0, options.label_font.clone(),
            label.style.text_color.as_deref().map(css_to_color).unwrap_or(Color { r: 15, g: 23, b: 42, a: 255 })));
    }
    let text_scene = layout_to_paint(&PositionedNode { x: 0.0, y: 0.0, width: diagram.width, height: diagram.height,
        id: None, content: None, children: text_children, ext: HashMap::new() }, &LayoutToPaintOptions {
        width: diagram.width, height: diagram.height, background: Color { r: 0, g: 0, b: 0, a: 0 }, device_pixel_ratio: 1.0,
        shaper: options.shaper, metrics: options.metrics, resolver: options.resolver,
    });
    instructions.extend(text_scene.instructions);
    PaintScene { width: diagram.width, height: diagram.height,
        background: format!("rgb({},{},{})", options.background.r, options.background.g, options.background.b),
        instructions, id: None, metadata: None }
}

/// Lower Ishikawa fishbone geometry into backend-neutral paths, a rect, and glyph runs.
pub fn diagram_to_paint_ishikawa<S, M, R>(diagram: &LayoutedIshikawaDiagram, options: &DiagramToPaintOptions<'_, S, M, R>) -> PaintScene
where S: TextShaper, M: FontMetrics<Handle = S::Handle>, R: FontResolver<Handle = S::Handle> {
    let stroke = "#334155".to_string();
    let mut instructions = vec![PaintInstruction::Path(line_path(
        &[diagram.spine_from.clone(), diagram.spine_to.clone()], &stroke, 3.0,
    ))];
    let mut text_children = Vec::new();
    for bone in &diagram.bones {
        instructions.push(PaintInstruction::Path(line_path(
            &[bone.from.clone(), bone.to.clone()], &stroke, if bone.depth == 1 { 2.5 } else { 1.5 },
        )));
        text_children.push(text_node(&bone.label, bone.label_position.x - 55.0, bone.label_position.y - 10.0,
            110.0, 24.0, options.label_font.clone(), Color { r: 30, g: 41, b: 59, a: 255 }));
    }
    instructions.push(PaintInstruction::Rect(PaintRect {
        base: PaintBase::default(), x: diagram.effect_x, y: diagram.effect_y, width: diagram.effect_width,
        height: diagram.effect_height, fill: Some("#fef3c7".into()), stroke: Some("#92400e".into()),
        stroke_width: Some(2.0), corner_radius: Some(8.0), stroke_dash: None, stroke_dash_offset: None,
    }));
    text_children.push(text_node(&diagram.effect, diagram.effect_x + 8.0, diagram.effect_y + 8.0,
        diagram.effect_width - 16.0, diagram.effect_height - 16.0, options.title_font.clone(),
        Color { r: 120, g: 53, b: 15, a: 255 }));
    let text_scene = layout_to_paint(&PositionedNode { x: 0.0, y: 0.0, width: diagram.width, height: diagram.height,
        id: None, content: None, children: text_children, ext: HashMap::new() }, &LayoutToPaintOptions {
        width: diagram.width, height: diagram.height, background: Color { r: 0, g: 0, b: 0, a: 0 }, device_pixel_ratio: 1.0,
        shaper: options.shaper, metrics: options.metrics, resolver: options.resolver,
    });
    instructions.extend(text_scene.instructions);
    PaintScene { width: diagram.width, height: diagram.height,
        background: format!("rgb({},{},{})", options.background.r, options.background.g, options.background.b),
        instructions, id: None, metadata: None }
}

/// Lower a Wardley strategic map into backend-neutral paths, ellipses, and glyph runs.
pub fn diagram_to_paint_wardley<S, M, R>(diagram: &LayoutedWardleyDiagram, options: &DiagramToPaintOptions<'_, S, M, R>) -> PaintScene
where S: TextShaper, M: FontMetrics<Handle = S::Handle>, R: FontResolver<Handle = S::Handle> {
    let left = 62.0; let right = diagram.width - 28.0; let top = if diagram.title.is_some() { 48.0 } else { 24.0 }; let bottom = diagram.height - 52.0;
    let mut instructions = vec![PaintInstruction::Path(line_path(&[Point { x: left, y: top }, Point { x: left, y: bottom }, Point { x: right, y: bottom }], "#475569", 2.0))];
    let mut text_children = Vec::new();
    if let Some(title) = &diagram.title {
        text_children.push(text_node(title, 10.0, 5.0, diagram.width - 20.0, 30.0, options.title_font.clone(), Color { r: 15, g: 23, b: 42, a: 255 }));
    }
    for (index, stage) in diagram.stages.iter().enumerate() {
        let x = left + (right - left) * index as f64 / (diagram.stages.len().saturating_sub(1).max(1)) as f64;
        if index > 0 { instructions.push(PaintInstruction::Path(line_path(&[Point { x, y: top }, Point { x, y: bottom }], "#cbd5e1", 1.0))); }
        text_children.push(text_node(stage, (x - 45.0).clamp(2.0, diagram.width - 92.0), bottom + 8.0, 90.0, 24.0, options.label_font.clone(), Color { r: 71, g: 85, b: 105, a: 255 }));
    }
    for link in &diagram.links { instructions.push(PaintInstruction::Path(line_path(&[link.from.clone(), link.to.clone()], "#64748b", 1.5))); }
    for evolution in &diagram.evolves { instructions.push(PaintInstruction::Path(line_path(&[evolution.from.clone(), evolution.to.clone()], "#dc2626", 2.0))); }
    for node in &diagram.nodes {
        let radius = if node.anchor { 8.0 } else { 6.0 };
        instructions.push(PaintInstruction::Ellipse(PaintEllipse { base: PaintBase::default(), cx: node.position.x, cy: node.position.y,
            rx: radius, ry: radius, fill: Some(if node.anchor { "#0f172a" } else { "#ffffff" }.into()), stroke: Some("#0f172a".into()),
            stroke_width: Some(2.0), stroke_dash: None, stroke_dash_offset: None }));
        text_children.push(text_node(&node.label, node.position.x + 9.0, node.position.y - 16.0, 150.0, 24.0,
            options.label_font.clone(), Color { r: 15, g: 23, b: 42, a: 255 }));
    }
    text_children.push(text_node("Visibility", 2.0, top, 56.0, 24.0, options.label_font.clone(), Color { r: 71, g: 85, b: 105, a: 255 }));
    let text_scene = layout_to_paint(&PositionedNode { x: 0.0, y: 0.0, width: diagram.width, height: diagram.height,
        id: None, content: None, children: text_children, ext: HashMap::new() }, &LayoutToPaintOptions {
        width: diagram.width, height: diagram.height, background: Color { r: 0, g: 0, b: 0, a: 0 }, device_pixel_ratio: 1.0,
        shaper: options.shaper, metrics: options.metrics, resolver: options.resolver,
    });
    instructions.extend(text_scene.instructions);
    PaintScene { width: diagram.width, height: diagram.height,
        background: format!("rgb({},{},{})", options.background.r, options.background.g, options.background.b), instructions, id: None, metadata: None }
}

/// Lower Cynefin domains and transitions into backend-neutral geometry and glyphs.
pub fn diagram_to_paint_cynefin<S, M, R>(diagram: &LayoutedCynefinDiagram, options: &DiagramToPaintOptions<'_, S, M, R>) -> PaintScene
where S: TextShaper, M: FontMetrics<Handle = S::Handle>, R: FontResolver<Handle = S::Handle> {
    let mut instructions = Vec::new(); let mut text_children = Vec::new();
    const COLORS: &[(&str, &str)] = &[("complex", "#dbeafe"), ("complicated", "#dcfce7"), ("clear", "#fef3c7"), ("chaotic", "#fee2e2")];
    for domain in diagram.domains.iter().filter(|domain| !domain.confusion) {
        let fill = COLORS.iter().find(|(name, _)| *name == domain.name).map_or("#f8fafc", |(_, color)| *color);
        instructions.push(PaintInstruction::Rect(PaintRect { base: PaintBase::default(), x: domain.x, y: domain.y,
            width: domain.width, height: domain.height, fill: Some(fill.into()), stroke: Some("#64748b".into()),
            stroke_width: Some(1.5), corner_radius: Some(20.0), stroke_dash: None, stroke_dash_offset: None }));
    }
    for transition in &diagram.transitions {
        instructions.push(PaintInstruction::Path(line_path(&[transition.from.clone(), transition.to.clone()], "#475569", 2.0)));
        if let Some(label) = &transition.label { text_children.push(text_node(label, (transition.from.x + transition.to.x) / 2.0 - 65.0,
            (transition.from.y + transition.to.y) / 2.0 - 30.0, 130.0, 24.0, options.label_font.clone(), Color { r: 51, g: 65, b: 85, a: 255 })); }
    }
    if let Some(domain) = diagram.domains.iter().find(|domain| domain.confusion) {
        instructions.push(PaintInstruction::Ellipse(PaintEllipse { base: PaintBase::default(), cx: domain.center.x, cy: domain.center.y,
            rx: domain.width / 2.0, ry: domain.height / 2.0, fill: Some("#e2e8f0".into()), stroke: Some("#475569".into()),
            stroke_width: Some(2.0), stroke_dash: None, stroke_dash_offset: None }));
    }
    if let Some(title) = &diagram.title { text_children.push(text_node(title, 10.0, 5.0, diagram.width - 20.0, 30.0,
        options.title_font.clone(), Color { r: 15, g: 23, b: 42, a: 255 })); }
    for domain in &diagram.domains {
        let label_y = if domain.confusion { domain.center.y - 34.0 } else { domain.y + 14.0 };
        text_children.push(text_node(&capitalize(&domain.name), domain.x + 12.0, label_y, domain.width - 24.0, 28.0,
            options.title_font.clone(), Color { r: 15, g: 23, b: 42, a: 255 }));
        for (index, item) in domain.items.iter().take(if domain.confusion { 3 } else { usize::MAX }).enumerate() {
            let y = if domain.confusion { domain.center.y - 2.0 + index as f64 * 22.0 } else { domain.y + 52.0 + index as f64 * 28.0 };
            text_children.push(text_node(item, domain.x + 18.0, y, domain.width - 36.0, 24.0, options.label_font.clone(), Color { r: 30, g: 41, b: 59, a: 255 }));
        }
    }
    let text_scene = layout_to_paint(&PositionedNode { x: 0.0, y: 0.0, width: diagram.width, height: diagram.height,
        id: None, content: None, children: text_children, ext: HashMap::new() }, &LayoutToPaintOptions { width: diagram.width,
        height: diagram.height, background: Color { r: 0, g: 0, b: 0, a: 0 }, device_pixel_ratio: 1.0,
        shaper: options.shaper, metrics: options.metrics, resolver: options.resolver });
    instructions.extend(text_scene.instructions);
    PaintScene { width: diagram.width, height: diagram.height,
        background: format!("rgb({},{},{})", options.background.r, options.background.g, options.background.b), instructions, id: None, metadata: None }
}

fn capitalize(value: &str) -> String {
    let mut characters = value.chars(); characters.next().map_or_else(String::new, |first| first.to_uppercase().collect::<String>() + characters.as_str())
}

fn with_opacity(color: &str, opacity: f64) -> String {
    let hex = color.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (u8::from_str_radix(&hex[0..2], 16), u8::from_str_radix(&hex[2..4], 16), u8::from_str_radix(&hex[4..6], 16)) {
            return format!("rgba({r},{g},{b},{})", opacity.clamp(0.0, 1.0));
        }
    }
    color.to_string()
}

/// Lower a layouted Event Modeling diagram into backend-neutral paint instructions.
pub fn diagram_to_paint_event_model<S, M, R>(
    diagram: &LayoutedEventModelDiagram,
    options: &DiagramToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let mut instructions = Vec::new();
    let mut text_children = Vec::new();
    let label_color = Color { r: 30, g: 41, b: 59, a: 255 };

    if let Some(title) = &diagram.title {
        text_children.push(text_node(
            title,
            0.0,
            8.0,
            diagram.width,
            28.0,
            options.title_font.clone(),
            label_color,
        ));
    }

    for item in &diagram.items {
        match item {
            LayoutedEventModelItem::Lane { x, y, width, height, label, fill } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: Some(fill.clone()),
                    stroke: Some("#cbd5e1".into()),
                    stroke_width: Some(1.0),
                    corner_radius: Some(6.0),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                text_children.push(text_node(
                    label,
                    x + 8.0,
                    y + 8.0,
                    132.0,
                    height - 16.0,
                    options.label_font.clone(),
                    label_color,
                ));
            }
            LayoutedEventModelItem::Frame { x, y, width, height, label, kind } => {
                let fill = match kind {
                    EventModelEntityKind::Ui => "#dbeafe",
                    EventModelEntityKind::Processor => "#e0e7ff",
                    EventModelEntityKind::Command => "#fef3c7",
                    EventModelEntityKind::ReadModel => "#dcfce7",
                    EventModelEntityKind::Event => "#fee2e2",
                };
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: Some(fill.into()),
                    stroke: Some("#475569".into()),
                    stroke_width: Some(1.5),
                    corner_radius: Some(5.0),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                text_children.push(text_node(
                    label,
                    x + 6.0,
                    y + 6.0,
                    width - 12.0,
                    height - 12.0,
                    options.label_font.clone(),
                    label_color,
                ));
            }
            LayoutedEventModelItem::Relation { from, to } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[from.clone(), to.clone()],
                    "#64748b",
                    2.0,
                )));
            }
        }
    }

    let text_root = PositionedNode {
        x: 0.0,
        y: 0.0,
        width: diagram.width,
        height: diagram.height,
        id: None,
        content: None,
        children: text_children,
        ext: HashMap::new(),
    };
    let text_scene = layout_to_paint(&text_root, &LayoutToPaintOptions {
        width: diagram.width,
        height: diagram.height,
        background: Color { r: 0, g: 0, b: 0, a: 0 },
        device_pixel_ratio: 1.0,
        shaper: options.shaper,
        metrics: options.metrics,
        resolver: options.resolver,
    });
    instructions.extend(text_scene.instructions);

    let mut metadata = HashMap::new();
    if let Some(title) = &diagram.accessibility_title {
        metadata.insert("accessibility.title".into(), title.clone());
    }
    if let Some(description) = &diagram.accessibility_description {
        metadata.insert("accessibility.description".into(), description.clone());
    }
    let bg = &options.background;
    PaintScene {
        width: diagram.width,
        height: diagram.height,
        background: format!("rgb({},{},{})", bg.r, bg.g, bg.b),
        instructions,
        id: None,
        metadata: (!metadata.is_empty()).then_some(metadata),
    }
}

// ============================================================================
// Node shape rendering (geometry only, no text)
// ============================================================================

fn node_shape_instruction(node: &LayoutedGraphNode) -> PaintInstruction {
    match node.shape {
        DiagramShape::Ellipse => PaintInstruction::Ellipse(PaintEllipse {
            base: PaintBase::default(),
            cx: node.x + node.width / 2.0,
            cy: node.y + node.height / 2.0,
            rx: node.width / 2.0,
            ry: node.height / 2.0,
            fill: Some(node.style.fill.clone()),
            stroke: Some(node.style.stroke.clone()),
            stroke_width: Some(node.style.stroke_width),
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
        DiagramShape::Diamond => {
            let cx = node.x + node.width / 2.0;
            let cy = node.y + node.height / 2.0;
            PaintInstruction::Path(PaintPath {
                base: PaintBase::default(),
                commands: vec![
                    PathCommand::MoveTo { x: cx, y: node.y },
                    PathCommand::LineTo {
                        x: node.x + node.width,
                        y: cy,
                    },
                    PathCommand::LineTo {
                        x: cx,
                        y: node.y + node.height,
                    },
                    PathCommand::LineTo { x: node.x, y: cy },
                    PathCommand::Close,
                ],
                fill: Some(node.style.fill.clone()),
                fill_rule: None,
                stroke: Some(node.style.stroke.clone()),
                stroke_width: Some(node.style.stroke_width),
                stroke_cap: None,
                stroke_join: Some(StrokeJoin::Round),
                stroke_dash: None,
                stroke_dash_offset: None,
            })
        }
        DiagramShape::Hexagon => {
            let inset = node.width.min(node.height * 2.0) * 0.18;
            polygon_node_instruction(node, &[
                (node.x + inset, node.y),
                (node.x + node.width - inset, node.y),
                (node.x + node.width, node.y + node.height / 2.0),
                (node.x + node.width - inset, node.y + node.height),
                (node.x + inset, node.y + node.height),
                (node.x, node.y + node.height / 2.0),
            ])
        }
        DiagramShape::ParallelogramRight => {
            let inset = node.width.min(node.height * 2.0) * 0.16;
            polygon_node_instruction(node, &[
                (node.x + inset, node.y),
                (node.x + node.width, node.y),
                (node.x + node.width - inset, node.y + node.height),
                (node.x, node.y + node.height),
            ])
        }
        DiagramShape::ParallelogramLeft => {
            let inset = node.width.min(node.height * 2.0) * 0.16;
            polygon_node_instruction(node, &[
                (node.x, node.y),
                (node.x + node.width - inset, node.y),
                (node.x + node.width, node.y + node.height),
                (node.x + inset, node.y + node.height),
            ])
        }
        DiagramShape::Trapezoid => {
            let inset = node.width.min(node.height * 2.0) * 0.16;
            polygon_node_instruction(node, &[
                (node.x + inset, node.y),
                (node.x + node.width - inset, node.y),
                (node.x + node.width, node.y + node.height),
                (node.x, node.y + node.height),
            ])
        }
        DiagramShape::InvertedTrapezoid => {
            let inset = node.width.min(node.height * 2.0) * 0.16;
            polygon_node_instruction(node, &[
                (node.x, node.y),
                (node.x + node.width, node.y),
                (node.x + node.width - inset, node.y + node.height),
                (node.x + inset, node.y + node.height),
            ])
        }
        DiagramShape::Stadium => {
            let radius = (node.height / 2.0).min(node.width / 2.0);
            PaintInstruction::Path(PaintPath {
                base: PaintBase::default(),
                commands: vec![
                    PathCommand::MoveTo { x: node.x + radius, y: node.y },
                    PathCommand::LineTo { x: node.x + node.width - radius, y: node.y },
                    PathCommand::ArcTo { rx: radius, ry: radius, x_rotation: 0.0, large_arc: false, sweep: true, x: node.x + node.width, y: node.y + radius },
                    PathCommand::ArcTo { rx: radius, ry: radius, x_rotation: 0.0, large_arc: false, sweep: true, x: node.x + node.width - radius, y: node.y + node.height },
                    PathCommand::LineTo { x: node.x + radius, y: node.y + node.height },
                    PathCommand::ArcTo { rx: radius, ry: radius, x_rotation: 0.0, large_arc: false, sweep: true, x: node.x, y: node.y + radius },
                    PathCommand::ArcTo { rx: radius, ry: radius, x_rotation: 0.0, large_arc: false, sweep: true, x: node.x + radius, y: node.y },
                    PathCommand::Close,
                ],
                fill: Some(node.style.fill.clone()), fill_rule: None,
                stroke: Some(node.style.stroke.clone()), stroke_width: Some(node.style.stroke_width),
                stroke_cap: None, stroke_join: Some(StrokeJoin::Round),
                stroke_dash: None, stroke_dash_offset: None,
            })
        }
        DiagramShape::Subroutine => {
            let inset = 12.0_f64.min(node.width / 5.0);
            PaintInstruction::Group(PaintGroup {
                base: PaintBase::default(),
                children: vec![
                    node_rect_instruction(node),
                    node_open_path_instruction(node, vec![
                        PathCommand::MoveTo { x: node.x + inset, y: node.y },
                        PathCommand::LineTo { x: node.x + inset, y: node.y + node.height },
                    ]),
                    node_open_path_instruction(node, vec![
                        PathCommand::MoveTo { x: node.x + node.width - inset, y: node.y },
                        PathCommand::LineTo { x: node.x + node.width - inset, y: node.y + node.height },
                    ]),
                ],
                transform: None,
                opacity: None,
            })
        }
        DiagramShape::Cylinder => {
            let cap = 10.0_f64.min(node.height / 4.0);
            PaintInstruction::Group(PaintGroup {
                base: PaintBase::default(),
                children: vec![
                    PaintInstruction::Path(PaintPath {
                        base: PaintBase::default(),
                        commands: vec![
                            PathCommand::MoveTo { x: node.x, y: node.y + cap },
                            PathCommand::LineTo { x: node.x, y: node.y + node.height - cap },
                            PathCommand::ArcTo { rx: node.width / 2.0, ry: cap, x_rotation: 0.0, large_arc: false, sweep: false, x: node.x + node.width, y: node.y + node.height - cap },
                            PathCommand::LineTo { x: node.x + node.width, y: node.y + cap },
                            PathCommand::Close,
                        ],
                        fill: Some(node.style.fill.clone()), fill_rule: None,
                        stroke: Some(node.style.stroke.clone()), stroke_width: Some(node.style.stroke_width),
                        stroke_cap: None, stroke_join: Some(StrokeJoin::Round),
                        stroke_dash: None, stroke_dash_offset: None,
                    }),
                    PaintInstruction::Ellipse(PaintEllipse {
                        base: PaintBase::default(),
                        cx: node.x + node.width / 2.0, cy: node.y + cap,
                        rx: node.width / 2.0, ry: cap,
                        fill: Some(node.style.fill.clone()), stroke: Some(node.style.stroke.clone()),
                        stroke_width: Some(node.style.stroke_width), stroke_dash: None, stroke_dash_offset: None,
                    }),
                ],
                transform: None,
                opacity: None,
            })
        }
        DiagramShape::DoubleCircle => {
            let inset = 5.0_f64.min(node.width / 8.0).min(node.height / 8.0);
            PaintInstruction::Group(PaintGroup {
                base: PaintBase::default(),
                children: vec![
                    node_ellipse_instruction(node, 0.0, Some(node.style.fill.clone())),
                    node_ellipse_instruction(node, inset, None),
                ],
                transform: None,
                opacity: None,
            })
        }
        DiagramShape::Note => {
            let fold = 12.0_f64.min(node.width / 4.0).min(node.height / 4.0);
            PaintInstruction::Path(PaintPath {
                base: PaintBase::default(),
                commands: vec![
                    PathCommand::MoveTo {
                        x: node.x,
                        y: node.y,
                    },
                    PathCommand::LineTo {
                        x: node.x + node.width - fold,
                        y: node.y,
                    },
                    PathCommand::LineTo {
                        x: node.x + node.width,
                        y: node.y + fold,
                    },
                    PathCommand::LineTo {
                        x: node.x + node.width,
                        y: node.y + node.height,
                    },
                    PathCommand::LineTo {
                        x: node.x,
                        y: node.y + node.height,
                    },
                    PathCommand::Close,
                ],
                fill: Some(node.style.fill.clone()),
                fill_rule: None,
                stroke: Some(node.style.stroke.clone()),
                stroke_width: Some(node.style.stroke_width),
                stroke_cap: None,
                stroke_join: Some(StrokeJoin::Round),
                stroke_dash: None,
                stroke_dash_offset: None,
            })
        }
        DiagramShape::Rect | DiagramShape::Bar => PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: node.x,
            y: node.y,
            width: node.width,
            height: node.height,
            fill: Some(node.style.fill.clone()),
            stroke: Some(node.style.stroke.clone()),
            stroke_width: Some(node.style.stroke_width),
            corner_radius: Some(0.0),
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
        DiagramShape::RoundedRect => PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: node.x,
            y: node.y,
            width: node.width,
            height: node.height,
            fill: Some(node.style.fill.clone()),
            stroke: Some(node.style.stroke.clone()),
            stroke_width: Some(node.style.stroke_width),
            corner_radius: Some(node.style.corner_radius),
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
    }
}

fn polygon_node_instruction(node: &LayoutedGraphNode, points: &[(f64, f64)]) -> PaintInstruction {
    let mut commands = Vec::with_capacity(points.len() + 1);
    let (x, y) = points[0];
    commands.push(PathCommand::MoveTo { x, y });
    commands.extend(points[1..].iter().map(|&(x, y)| PathCommand::LineTo { x, y }));
    commands.push(PathCommand::Close);
    PaintInstruction::Path(PaintPath {
        base: PaintBase::default(),
        commands,
        fill: Some(node.style.fill.clone()),
        fill_rule: None,
        stroke: Some(node.style.stroke.clone()),
        stroke_width: Some(node.style.stroke_width),
        stroke_cap: None,
        stroke_join: Some(StrokeJoin::Round),
        stroke_dash: None,
        stroke_dash_offset: None,
    })
}

fn node_rect_instruction(node: &LayoutedGraphNode) -> PaintInstruction {
    PaintInstruction::Rect(PaintRect {
        base: PaintBase::default(), x: node.x, y: node.y, width: node.width, height: node.height,
        fill: Some(node.style.fill.clone()), stroke: Some(node.style.stroke.clone()),
        stroke_width: Some(node.style.stroke_width), corner_radius: Some(0.0),
        stroke_dash: None, stroke_dash_offset: None,
    })
}

fn node_open_path_instruction(node: &LayoutedGraphNode, commands: Vec<PathCommand>) -> PaintInstruction {
    PaintInstruction::Path(PaintPath {
        base: PaintBase::default(), commands, fill: None, fill_rule: None,
        stroke: Some(node.style.stroke.clone()), stroke_width: Some(node.style.stroke_width),
        stroke_cap: None, stroke_join: Some(StrokeJoin::Round),
        stroke_dash: None, stroke_dash_offset: None,
    })
}

fn node_ellipse_instruction(node: &LayoutedGraphNode, inset: f64, fill: Option<String>) -> PaintInstruction {
    PaintInstruction::Ellipse(PaintEllipse {
        base: PaintBase::default(),
        cx: node.x + node.width / 2.0, cy: node.y + node.height / 2.0,
        rx: node.width / 2.0 - inset, ry: node.height / 2.0 - inset,
        fill, stroke: Some(node.style.stroke.clone()), stroke_width: Some(node.style.stroke_width),
        stroke_dash: None, stroke_dash_offset: None,
    })
}

// ============================================================================
// Edge rendering (geometry only — labels go through text bridge below)
// ============================================================================

fn line_path(points: &[Point], stroke: &str, stroke_width: f64) -> PaintPath {
    let mut commands: Vec<PathCommand> = Vec::with_capacity(points.len());
    for (i, pt) in points.iter().enumerate() {
        if i == 0 {
            commands.push(PathCommand::MoveTo { x: pt.x, y: pt.y });
        } else {
            commands.push(PathCommand::LineTo { x: pt.x, y: pt.y });
        }
    }
    PaintPath {
        base: PaintBase::default(),
        commands,
        fill: Some("none".to_string()),
        fill_rule: None,
        stroke: Some(stroke.to_string()),
        stroke_width: Some(stroke_width),
        stroke_cap: Some(StrokeCap::Round),
        stroke_join: Some(StrokeJoin::Round),
        stroke_dash: None,
        stroke_dash_offset: None,
    }
}

/// Filled triangle arrowhead at the tip of a directed edge.
///
/// ```text
///          end
///         /|\
///        / | \
///       /  |  \
///  left    |   right
///       base_mid
/// ```
fn arrowhead(edge: &LayoutedGraphEdge) -> Option<PaintPath> {
    if edge.kind != EdgeKind::Directed || edge.points.len() < 2 {
        return None;
    }

    let end = &edge.points[edge.points.len() - 1];
    let prev = &edge.points[edge.points.len() - 2];

    let dx = end.x - prev.x;
    let dy = end.y - prev.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-9 {
        return None;
    }

    let ux = dx / len;
    let uy = dy / len;
    let size = 10.0;
    let half_w = size * 0.6;

    let base_x = end.x - ux * size;
    let base_y = end.y - uy * size;
    let px = -uy;
    let py = ux;

    Some(PaintPath {
        base: PaintBase::default(),
        commands: vec![
            PathCommand::MoveTo { x: end.x, y: end.y },
            PathCommand::LineTo {
                x: base_x + px * half_w,
                y: base_y + py * half_w,
            },
            PathCommand::LineTo {
                x: base_x - px * half_w,
                y: base_y - py * half_w,
            },
            PathCommand::Close,
        ],
        fill: Some(edge.style.stroke.clone()),
        fill_rule: None,
        stroke: Some(edge.style.stroke.clone()),
        stroke_width: Some(1.0),
        stroke_cap: None,
        stroke_join: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    })
}

// ============================================================================
// Text bridge — PositionedNode construction
// ============================================================================

/// Convert a diagram-ir color string (CSS hex or "none") to a layout-ir Color.
/// Falls back to opaque black when the string is not a supported hex format.
fn css_to_color(css: &str) -> Color {
    let s = css.trim_start_matches('#');
    if s.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&s[0..2], 16),
            u8::from_str_radix(&s[2..4], 16),
            u8::from_str_radix(&s[4..6], 16),
        ) {
            return Color { r, g, b, a: 255 };
        }
    }
    Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    } // opaque black fallback
}

fn text_node(
    value: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    font: FontSpec,
    color: Color,
) -> PositionedNode {
    PositionedNode {
        x,
        y,
        width,
        height,
        id: None,
        content: Some(Content::Text(TextContent {
            value: value.to_string(),
            font,
            color,
            decoration: None,
            max_lines: None,
            wrap: true,
            text_align: TextAlign::Center,
        })),
        children: Vec::new(),
        ext: HashMap::new(),
    }
}

fn text_node_no_wrap(
    value: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    font: FontSpec,
    color: Color,
) -> PositionedNode {
    let mut node = text_node(value, x, y, width, height, font, color);
    if let Some(Content::Text(text)) = &mut node.content {
        text.wrap = false;
    }
    node
}

// ============================================================================
// Public API
// ============================================================================

/// Lower a [`LayoutedGraphDiagram`] into a [`PaintScene`].
///
/// Node shapes and edge geometry are emitted directly as typed paint
/// instructions. All text (node labels, edge labels, title) is routed through
/// `layout-to-paint` so every paint backend receives real glyph IDs produced
/// by the TXT00 shaping pipeline.
pub fn diagram_to_paint<S, M, R>(
    diagram: &LayoutedGraphDiagram,
    options: &DiagramToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let mut instructions: Vec<PaintInstruction> = Vec::new();

    // ── 1. Composite groups — drawn behind edges and member nodes ────────────
    for group in &diagram.groups {
        instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: group.x,
            y: group.y,
            width: group.width,
            height: group.height,
            fill: Some(group.style.fill.clone()),
            stroke: Some(group.style.stroke.clone()),
            stroke_width: Some(group.style.stroke_width),
            corner_radius: Some(group.style.corner_radius),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
        for divider_y in &group.divider_y {
            instructions.push(PaintInstruction::Path(line_path(
                &[
                    Point {
                        x: group.x,
                        y: *divider_y,
                    },
                    Point {
                        x: group.x + group.width,
                        y: *divider_y,
                    },
                ],
                &group.style.stroke,
                group.style.stroke_width,
            )));
        }
    }

    // ── 2. Edges (lines + arrowheads) — drawn behind nodes ───────────────────
    for edge in &diagram.edges {
        let mut path = line_path(&edge.points, &edge.style.stroke, edge.style.stroke_width);
        if edge.kind == EdgeKind::NoteAssociation {
            path.stroke_dash = Some(vec![4.0, 4.0]);
        }
        instructions.push(PaintInstruction::Path(path));
        if let Some(tip) = arrowhead(edge) {
            instructions.push(PaintInstruction::Path(tip));
        }
    }

    // ── 3. Node shapes — drawn over edges so endpoints are hidden ─────────────
    for node in &diagram.nodes {
        if diagram.hide_empty_descriptions && node.label.text.is_empty() {
            continue;
        }
        instructions.push(node_shape_instruction(node));
    }

    // ── 4. Text — all labels routed through layout-to-paint ───────────────────
    //
    // Build one PositionedNode per text item, collect them as children of a
    // transparent synthetic root spanning the full canvas, then call
    // layout_to_paint once. Append the resulting PaintGlyphRun instructions.
    let label_font = options.label_font.clone();
    let title_font = options.title_font.clone();
    let label_size = label_font.size;
    let title_size = title_font.size;

    let mut text_children: Vec<PositionedNode> = Vec::new();

    for group in &diagram.groups {
        text_children.push(text_node_no_wrap(
            &group.label.text,
            group.x + 12.0,
            group.y + 8.0,
            group.width - 24.0,
            label_size * 1.2,
            {
                let mut f = label_font.clone();
                f.size = group.style.font_size;
                f.weight = group.style.font_weight;
                f.italic = group.style.font_italic;
                f.family.clone_from(&group.style.font_family);
                f
            },
            css_to_color(&group.style.text_color),
        ));
    }

    // Title (if present) — centred at the top of the canvas.
    if let Some(title) = &diagram.title {
        text_children.push(text_node(
            title,
            0.0,
            8.0,
            diagram.width,
            title_size * 1.2,
            title_font,
            Color {
                r: 17,
                g: 24,
                b: 39,
                a: 255,
            }, // #111827
        ));
    }

    // Edge labels.
    for edge in &diagram.edges {
        if let (Some(label), Some(pos)) = (&edge.label, &edge.label_position) {
            text_children.push(text_node(
                &label.text,
                pos.x - 60.0,
                pos.y - label_size,
                120.0,
                label_size * 1.2,
                {
                    let mut f = label_font.clone();
                    f.size = edge.style.font_size;
                    f.weight = edge.style.font_weight;
                    f.italic = edge.style.font_italic;
                    f.family.clone_from(&edge.style.font_family);
                    f
                },
                css_to_color(&edge.style.text_color),
            ));
        }
    }

    // Node labels — vertically centred inside each node bounding box.
    for node in &diagram.nodes {
        if diagram.hide_empty_descriptions && node.label.text.is_empty() {
            continue;
        }
        let line_count = node.label.text.lines().count().max(1) as f64;
        let text_height = line_count * node.style.font_size * 1.2;
        text_children.push(text_node_no_wrap(
            &node.label.text,
            node.x,
            node.y + (node.height - text_height) / 2.0,
            node.width,
            text_height,
            {
                let mut f = label_font.clone();
                f.size = node.style.font_size;
                f.weight = node.style.font_weight;
                f.italic = node.style.font_italic;
                f.family.clone_from(&node.style.font_family);
                f
            },
            css_to_color(&node.style.text_color),
        ));
    }

    // Synthetic transparent root spanning the full canvas.
    let text_root = PositionedNode {
        x: 0.0,
        y: 0.0,
        width: diagram.width,
        height: diagram.height,
        id: None,
        content: None,
        children: text_children,
        ext: HashMap::new(),
    };

    // Use DPR=1 for the text bridge. `diagram_to_paint` emits all geometry
    // (rects, paths) in logical pixels and the PaintScene dimensions are
    // logical. layout_to_paint with DPR>1 would emit glyph positions in
    // device pixels, causing a mismatch: paint-metal creates the CGBitmap at
    // scene.height logical pixels and flips y as (height - gy), so a device-
    // pixel y value would land off-canvas. Keeping everything in logical pixel
    // space is consistent. A future pass can scale the whole scene by DPR.
    let text_opts = LayoutToPaintOptions {
        width: diagram.width,
        height: diagram.height,
        background: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }, // transparent root
        device_pixel_ratio: 1.0,
        shaper: options.shaper,
        metrics: options.metrics,
        resolver: options.resolver,
    };
    let text_scene = layout_to_paint(&text_root, &text_opts);
    instructions.extend(text_scene.instructions);

    let mut metadata = HashMap::new();
    if let Some(title) = &diagram.accessibility_title {
        metadata.insert("accessibility.title".into(), title.clone());
    }
    if let Some(description) = &diagram.accessibility_description {
        metadata.insert("accessibility.description".into(), description.clone());
    }
    for link in &diagram.links {
        let prefix = format!("graph.node.{}.link", link.node_id);
        metadata.insert(format!("{prefix}.url"), link.url.clone());
        if let Some(tooltip) = &link.tooltip {
            metadata.insert(format!("{prefix}.tooltip"), tooltip.clone());
        }
        if let Some(node) = diagram.nodes.iter().find(|node| node.id == link.node_id) {
            metadata.insert(
                format!("{prefix}.bounds"),
                format!("{},{},{},{}", node.x, node.y, node.width, node.height),
            );
        }
    }

    let bg = options.background;
    PaintScene {
        width: diagram.width,
        height: diagram.height,
        background: if bg.a == 255 {
            format!("rgb({}, {}, {})", bg.r, bg.g, bg.b)
        } else {
            let a = bg.a as f64 / 255.0;
            format!("rgba({}, {}, {}, {:.4})", bg.r, bg.g, bg.b, a)
        },
        instructions,
        id: None,
        metadata: (!metadata.is_empty()).then_some(metadata),
    }
}

/// Lower packet field rectangles and shaped labels into backend-neutral paint.
pub fn diagram_to_paint_packet<S, M, R>(
    diagram: &LayoutedPacketDiagram,
    options: &DiagramToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let mut instructions = Vec::new();
    let mut text_children = Vec::new();

    if let Some(title) = &diagram.title {
        let mut title_font = options.title_font.clone();
        title_font.size = diagram.title_style.font_size;
        text_children.push(text_node(
            title,
            0.0,
            diagram.title_y - title_font.size * 0.6,
            diagram.width,
            title_font.size * 1.2,
            title_font,
            css_to_color(&diagram.title_style.text_color),
        ));
    }
    for field in &diagram.fields {
        instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: field.x,
            y: field.y,
            width: field.width,
            height: field.height,
            fill: Some(field.style.fill.clone()),
            stroke: Some(field.style.stroke.clone()),
            stroke_width: Some(field.style.stroke_width),
            corner_radius: Some(0.0),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
        let mut label_font = options.label_font.clone();
        label_font.size = field.style.font_size;
        text_children.push(text_node_no_wrap(
            &field.label.text,
            field.x,
            field.y + (field.height - label_font.size * 1.2) / 2.0,
            field.width.max(1.0),
            (label_font.size * 1.2).min(field.height),
            label_font,
            css_to_color(&field.style.text_color),
        ));
    }
    for label in &diagram.bit_labels {
        let align = match label.align {
            GeoTextAlign::Left => TextAlign::Start,
            GeoTextAlign::Center => TextAlign::Center,
            GeoTextAlign::Right => TextAlign::End,
        };
        let mut bit_font = options.label_font.clone();
        bit_font.size = label.style.font_size;
        let mut text_node = text_node_no_wrap(
            &label.text,
            label.x,
            label.y,
            label.width.max(1.0),
            label.height,
            bit_font.clone(),
            css_to_color(&label.style.text_color),
        );
        if let Some(Content::Text(text)) = &mut text_node.content {
            text.text_align = align;
        }
        text_children.push(text_node);
    }

    let text_root = PositionedNode {
        x: 0.0,
        y: 0.0,
        width: diagram.width,
        height: diagram.height,
        id: None,
        content: None,
        children: text_children,
        ext: HashMap::new(),
    };
    let text_scene = layout_to_paint(
        &text_root,
        &LayoutToPaintOptions {
            width: diagram.width,
            height: diagram.height,
            background: Color { r: 0, g: 0, b: 0, a: 0 },
            device_pixel_ratio: 1.0,
            shaper: options.shaper,
            metrics: options.metrics,
            resolver: options.resolver,
        },
    );
    instructions.extend(text_scene.instructions);

    let mut metadata = HashMap::new();
    if let Some(title) = &diagram.accessibility_title {
        metadata.insert("accessibility.title".into(), title.clone());
    }
    if let Some(description) = &diagram.accessibility_description {
        metadata.insert("accessibility.description".into(), description.clone());
    }
    let bg = options.background;
    PaintScene {
        width: diagram.width,
        height: diagram.height,
        background: if bg.a == 255 {
            format!("rgb({}, {}, {})", bg.r, bg.g, bg.b)
        } else {
            format!("rgba({}, {}, {}, {:.4})", bg.r, bg.g, bg.b, f64::from(bg.a) / 255.0)
        },
        instructions,
        id: None,
        metadata: (!metadata.is_empty()).then_some(metadata),
    }
}

/// Lower Kanban columns and cards into backend-neutral paint instructions.
pub fn diagram_to_paint_board<S, M, R>(
    board: &LayoutedBoardDiagram,
    options: &DiagramToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let mut instructions = Vec::new();
    let mut text_children = Vec::new();
    for column in &board.columns {
        instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(), x: column.x, y: column.y,
            width: column.width, height: column.height,
            fill: Some(column.style.fill.clone()), stroke: Some(column.style.stroke.clone()),
            stroke_width: Some(column.style.stroke_width),
            corner_radius: Some(column.style.corner_radius),
            stroke_dash: None, stroke_dash_offset: None,
        }));
        let mut heading_font = options.title_font.clone();
        heading_font.size = 16.0;
        text_children.push(text_node_no_wrap(
            &column.label.text, column.x + 12.0, column.y + 14.0,
            column.width - 24.0, 22.0, heading_font,
            css_to_color(&column.style.text_color),
        ));
        for card in &column.cards {
            instructions.push(PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(), x: card.x, y: card.y,
                width: card.width, height: card.height,
                fill: Some(card.style.fill.clone()), stroke: Some(card.style.stroke.clone()),
                stroke_width: Some(card.style.stroke_width),
                corner_radius: Some(card.style.corner_radius),
                stroke_dash: None, stroke_dash_offset: None,
            }));
            text_children.push(text_node(
                &card.label.text, card.x + 10.0, card.y + 18.0,
                card.width - 20.0, card.height - 24.0,
                options.label_font.clone(), css_to_color(&card.style.text_color),
            ));
        }
    }
    let root = PositionedNode {
        x: 0.0, y: 0.0, width: board.width, height: board.height,
        id: None, content: None, children: text_children, ext: HashMap::new(),
    };
    instructions.extend(layout_to_paint(&root, &LayoutToPaintOptions {
        width: board.width, height: board.height,
        background: Color { r: 0, g: 0, b: 0, a: 0 },
        device_pixel_ratio: 1.0, shaper: options.shaper,
        metrics: options.metrics, resolver: options.resolver,
    }).instructions);
    let bg = options.background;
    PaintScene {
        width: board.width, height: board.height,
        background: format!("rgba({}, {}, {}, {:.4})", bg.r, bg.g, bg.b, f64::from(bg.a) / 255.0),
        instructions, id: None, metadata: None,
    }
}

// ============================================================================
// Tests
// ============================================================================

// ============================================================================
// Chart family (DG04)
// ============================================================================

fn font_with_size(base: &FontSpec, size: Option<f64>) -> FontSpec {
    let mut font = base.clone();
    if let Some(size) = size {
        font.size = size;
    }
    font
}

/// Lower a [`LayoutedChartDiagram`] into a [`PaintScene`].
pub fn diagram_to_paint_chart<S, M, R>(
    diagram: &LayoutedChartDiagram,
    options: &DiagramToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let mut instructions: Vec<PaintInstruction> = Vec::new();
    let mut text_children: Vec<PositionedNode> = Vec::new();
    let mut rotated_text_children: Vec<(PositionedNode, f64, f64, f64)> = Vec::new();
    let lf = options.label_font.clone();
    let ls = lf.size;

    if let Some(ref tb) = diagram.title_box {
        text_children.push(text_node(
            &tb.text,
            tb.x - diagram.width / 2.0,
            tb.y - ls,
            diagram.width,
            ls * 1.4,
            options.title_font.clone(),
            Color {
                r: 17,
                g: 24,
                b: 39,
                a: 255,
            },
        ));
    }

    for item in &diagram.items {
        match item {
            LayoutedChartItem::AxisSpine {
                x1,
                y1,
                x2,
                y2,
                stroke_width,
                color,
                ..
            } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x1, y: *y1 }, Point { x: *x2, y: *y2 }],
                    color,
                    *stroke_width,
                )));
            }
            LayoutedChartItem::AxisTickMark {
                x1,
                y1,
                x2,
                y2,
                stroke_width,
                color,
            } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x1, y: *y1 }, Point { x: *x2, y: *y2 }],
                    color,
                    *stroke_width,
                )));
            }
            LayoutedChartItem::GridLine { x1, y1, x2, y2 } => {
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo { x: *x1, y: *y1 },
                        PathCommand::LineTo { x: *x2, y: *y2 },
                    ],
                    fill: Some("none".into()),
                    fill_rule: None,
                    stroke: Some("#e5e7eb".into()),
                    stroke_width: Some(1.0),
                    stroke_cap: None,
                    stroke_join: None,
                    stroke_dash: Some(vec![4.0, 4.0]),
                    stroke_dash_offset: None,
                }));
            }
            LayoutedChartItem::Bar {
                x,
                y,
                width,
                height,
                color,
            } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: Some(color.clone()),
                    stroke: None,
                    stroke_width: None,
                    corner_radius: Some(2.0),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }
            LayoutedChartItem::LinePath { points, color } => {
                if points.len() >= 2 {
                    instructions.push(PaintInstruction::Path(line_path(points, color, 2.0)));
                }
            }
            LayoutedChartItem::PointLabel {
                x,
                y,
                width,
                height,
                text,
                font_size,
                color,
            } => {
                text_children.push(text_node(
                    text,
                    *x,
                    *y,
                    *width,
                    *height,
                    font_with_size(&lf, Some(*font_size)),
                    css_to_color(color),
                ));
            }
            LayoutedChartItem::BarLabel {
                x,
                y,
                width,
                height,
                text,
                font_size,
                color,
            } => {
                text_children.push(text_node(
                    text,
                    *x,
                    *y,
                    *width,
                    *height,
                    font_with_size(&lf, Some(*font_size)),
                    css_to_color(color),
                ));
            }
            LayoutedChartItem::PieArc {
                cx,
                cy,
                r,
                start_angle,
                end_angle,
                color,
                label,
            } => {
                let cmds = pie_slice_commands(*cx, *cy, *r, *start_angle, *end_angle);
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: cmds,
                    fill: Some(color.clone()),
                    fill_rule: None,
                    stroke: Some("#ffffff".into()),
                    stroke_width: Some(1.5),
                    stroke_cap: None,
                    stroke_join: None,
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                // Label at midpoint of arc
                let mid = (start_angle + end_angle) / 2.0;
                let lx = cx + (r * 0.65) * mid.cos();
                let ly = cy + (r * 0.65) * mid.sin();
                text_children.push(text_node(
                    label,
                    lx - 40.0,
                    ly - ls / 2.0,
                    80.0,
                    ls * 1.2,
                    lf.clone(),
                    Color {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 255,
                    },
                ));
            }
            LayoutedChartItem::SankeyBand {
                from_x,
                from_y,
                to_x,
                to_y,
                width,
                color,
            } => {
                let control_x = (from_x + to_x) / 2.0;
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo {
                            x: *from_x,
                            y: *from_y,
                        },
                        PathCommand::CubicTo {
                            cx1: control_x,
                            cy1: *from_y,
                            cx2: control_x,
                            cy2: *to_y,
                            x: *to_x,
                            y: *to_y,
                        },
                        PathCommand::LineTo {
                            x: *to_x,
                            y: to_y + width,
                        },
                        PathCommand::CubicTo {
                            cx1: control_x,
                            cy1: to_y + width,
                            cx2: control_x,
                            cy2: from_y + width,
                            x: *from_x,
                            y: from_y + width,
                        },
                        PathCommand::Close,
                    ],
                    fill: Some(color.clone()),
                    fill_rule: None,
                    stroke: None,
                    stroke_width: None,
                    stroke_cap: None,
                    stroke_join: None,
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }
            LayoutedChartItem::SankeyNode {
                x,
                y,
                width,
                height,
                color,
                label,
            } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: Some(color.clone()),
                    stroke: Some("#ffffff".into()),
                    stroke_width: Some(1.0),
                    corner_radius: Some(1.0),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                let label_x = if *x > diagram.width / 2.0 {
                    x - 124.0
                } else {
                    x + width + 4.0
                };
                text_children.push(text_node(
                    label,
                    label_x,
                    y + height / 2.0 - ls / 2.0,
                    120.0,
                    ls * 1.2,
                    lf.clone(),
                    Color {
                        r: 31,
                        g: 41,
                        b: 55,
                        a: 255,
                    },
                ));
            }
            LayoutedChartItem::QuadrantRegion {
                x,
                y,
                width,
                height,
                color,
                label,
                label_font_size,
                label_top_padding,
                label_color,
            } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: Some(color.clone()),
                    stroke: None,
                    stroke_width: None,
                    corner_radius: None,
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                if let Some(label) = label {
                    text_children.push(text_node(
                        label,
                        x + width / 2.0 - 60.0,
                        y + label_top_padding,
                        120.0,
                        label_font_size.unwrap_or(ls) * 1.2,
                        font_with_size(&lf, *label_font_size),
                        css_to_color(label_color),
                    ));
                }
            }
            LayoutedChartItem::QuadrantBorder {
                x,
                y,
                width,
                height,
                internal_color,
                external_color,
                internal_width,
                external_width,
            } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: Some("none".into()),
                    stroke: Some(external_color.clone()),
                    stroke_width: Some(*external_width),
                    corner_radius: None,
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                let center_x = x + width / 2.0;
                let center_y = y + height / 2.0;
                instructions.push(PaintInstruction::Path(line_path(
                    &[
                        Point { x: center_x, y: *y },
                        Point {
                            x: center_x,
                            y: y + height,
                        },
                    ],
                    internal_color,
                    *internal_width,
                )));
                instructions.push(PaintInstruction::Path(line_path(
                    &[
                        Point { x: *x, y: center_y },
                        Point {
                            x: x + width,
                            y: center_y,
                        },
                    ],
                    internal_color,
                    *internal_width,
                )));
            }
            LayoutedChartItem::ScatterPoint {
                x,
                y,
                radius,
                color,
                stroke_color,
                stroke_width,
                label,
                label_font_size,
                label_padding,
                label_color,
            } => {
                instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                    base: PaintBase::default(),
                    cx: *x,
                    cy: *y,
                    rx: *radius,
                    ry: *radius,
                    fill: Some(color.clone()),
                    stroke: Some(stroke_color.clone()),
                    stroke_width: Some(*stroke_width),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                text_children.push(text_node(
                    label,
                    x - 50.0,
                    y + radius + label_padding,
                    100.0,
                    label_font_size.unwrap_or(ls) * 1.2,
                    font_with_size(&lf, *label_font_size),
                    css_to_color(label_color),
                ));
            }
            LayoutedChartItem::DataLabel {
                x,
                y,
                text,
                font_size,
                color,
            } => {
                let width = diagram.width.min(240.0);
                let label_x = (x - width / 2.0).clamp(0.0, diagram.width - width);
                text_children.push(text_node(
                    text,
                    label_x,
                    y - ls / 2.0,
                    width,
                    font_size.unwrap_or(ls) * 1.2,
                    font_with_size(&lf, *font_size),
                    color.as_deref().map(css_to_color).unwrap_or(Color {
                        r: 55,
                        g: 65,
                        b: 81,
                        a: 255,
                    }),
                ));
            }
            LayoutedChartItem::AxisTick {
                x,
                y,
                label,
                orientation,
                font_size,
                rotation_degrees,
                color,
            } => {
                let (tx, ty, tw) = match orientation {
                    Orientation::Horizontal => (x - 30.0, y - ls / 2.0, 60.0),
                    Orientation::Vertical => (x - 30.0, y + 2.0, 60.0),
                };
                let node = text_node_no_wrap(
                    label,
                    tx,
                    ty,
                    tw,
                    font_size * 1.2,
                    font_with_size(&lf, Some(*font_size)),
                    css_to_color(color),
                );
                if rotation_degrees.abs() > f64::EPSILON {
                    rotated_text_children.push((node, *rotation_degrees, *x, *y));
                } else {
                    text_children.push(node);
                }
            }
            LayoutedChartItem::Legend {
                x,
                y,
                entries,
                font_size,
            } => {
                let legend_font_size = font_size.unwrap_or(ls);
                let mut ex = *x;
                for e in entries {
                    instructions.push(PaintInstruction::Rect(PaintRect {
                        base: PaintBase::default(),
                        x: ex,
                        y: y - legend_font_size / 2.0,
                        width: legend_font_size,
                        height: legend_font_size,
                        fill: Some(e.color.clone()),
                        stroke: None,
                        stroke_width: None,
                        corner_radius: None,
                        stroke_dash: None,
                        stroke_dash_offset: None,
                    }));
                    text_children.push(text_node(
                        &e.label,
                        ex + legend_font_size + 4.0,
                        y - legend_font_size / 2.0,
                        80.0,
                        legend_font_size * 1.2,
                        font_with_size(&lf, Some(legend_font_size)),
                        Color {
                            r: 55,
                            g: 65,
                            b: 81,
                            a: 255,
                        },
                    ));
                    ex += legend_font_size + 4.0 + 88.0;
                }
            }
        }
    }

    let text_root = PositionedNode {
        x: 0.0,
        y: 0.0,
        width: diagram.width,
        height: diagram.height,
        id: None,
        content: None,
        children: text_children,
        ext: HashMap::new(),
    };
    let text_opts = LayoutToPaintOptions {
        width: diagram.width,
        height: diagram.height,
        background: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
        device_pixel_ratio: 1.0,
        shaper: options.shaper,
        metrics: options.metrics,
        resolver: options.resolver,
    };
    let text_scene = layout_to_paint(&text_root, &text_opts);
    instructions.extend(text_scene.instructions);
    for (node, rotation_degrees, pivot_x, pivot_y) in rotated_text_children {
        let root = PositionedNode {
            x: 0.0,
            y: 0.0,
            width: diagram.width,
            height: diagram.height,
            id: None,
            content: None,
            children: vec![node],
            ext: HashMap::new(),
        };
        let scene = layout_to_paint(&root, &text_opts);
        let radians = rotation_degrees.to_radians();
        let cosine = radians.cos();
        let sine = radians.sin();
        instructions.push(PaintInstruction::Group(PaintGroup {
            base: PaintBase::default(),
            children: scene.instructions,
            transform: Some([
                cosine,
                sine,
                -sine,
                cosine,
                pivot_x - cosine * pivot_x + sine * pivot_y,
                pivot_y - sine * pivot_x - cosine * pivot_y,
            ]),
            opacity: None,
        }));
    }

    let bg = options.background;
    let mut metadata = HashMap::new();
    if let Some(title) = &diagram.accessibility_title {
        metadata.insert("accessibility.title".into(), title.clone());
    }
    if let Some(description) = &diagram.accessibility_description {
        metadata.insert("accessibility.description".into(), description.clone());
    }
    PaintScene {
        width: diagram.width,
        height: diagram.height,
        background: diagram
            .background_color
            .clone()
            .unwrap_or_else(|| format!("rgb({},{},{})", bg.r, bg.g, bg.b)),
        instructions,
        id: None,
        metadata: (!metadata.is_empty()).then_some(metadata),
    }
}

/// Build `PathCommand`s for a filled pie slice (center → arc → close).
fn pie_slice_commands(cx: f64, cy: f64, r: f64, start: f64, end: f64) -> Vec<PathCommand> {
    let mut cmds = vec![
        PathCommand::MoveTo { x: cx, y: cy },
        PathCommand::LineTo {
            x: cx + r * start.cos(),
            y: cy + r * start.sin(),
        },
    ];
    // Split arc into ≤ 90° segments.
    let total = end - start;
    let n = ((total.abs() / (std::f64::consts::FRAC_PI_2)).ceil() as usize).max(1);
    let step = total / n as f64;
    for i in 0..n {
        let a0 = start + i as f64 * step;
        let a1 = a0 + step;
        let k = (4.0 / 3.0) * ((a1 - a0) / 4.0).tan();
        let (c0s, c0c) = (a0.sin(), a0.cos());
        let (c1s, c1c) = (a1.sin(), a1.cos());
        cmds.push(PathCommand::CubicTo {
            cx1: cx + r * (c0c - k * c0s),
            cy1: cy + r * (c0s + k * c0c),
            cx2: cx + r * (c1c + k * c1s),
            cy2: cy + r * (c1s - k * c1c),
            x: cx + r * c1c,
            y: cy + r * c1s,
        });
    }
    cmds.push(PathCommand::Close);
    cmds
}

// ============================================================================
// Structural family (DG04)
// ============================================================================

/// Lower a [`LayoutedStructuralDiagram`] into a [`PaintScene`].
pub fn diagram_to_paint_structural<S, M, R>(
    diagram: &LayoutedStructuralDiagram,
    options: &DiagramToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let mut instructions: Vec<PaintInstruction> = Vec::new();
    let mut text_children: Vec<PositionedNode> = Vec::new();
    let mut scene_metadata = HashMap::new();
    if let Some(title) = &diagram.accessibility_title {
        scene_metadata.insert("accessibility.title".into(), title.clone());
    }
    if let Some(description) = &diagram.accessibility_description {
        scene_metadata.insert("accessibility.description".into(), description.clone());
    }
    let lf = options.label_font.clone();
    let ls = lf.size;

    // Groups are backend-neutral containers. Draw outer groups first so nested
    // groups, relationships, and nodes naturally layer above them.
    for group in &diagram.groups {
        instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: group.x,
            y: group.y,
            width: group.width,
            height: group.height,
            fill: Some("#f8fafc".into()),
            stroke: Some("#94a3b8".into()),
            stroke_width: Some(1.5),
            corner_radius: Some(8.0),
            stroke_dash: Some(vec![6.0, 4.0]),
            stroke_dash_offset: None,
        }));
        let label = match &group.stereotype {
            Some(stereotype) => format!("«{stereotype}» {}", group.label),
            None => group.label.clone(),
        };
        text_children.push(text_node(
            &label,
            group.x + 10.0,
            group.y + 6.0,
            group.width - 20.0,
            ls * 1.3,
            lf.clone(),
            Color {
                r: 71,
                g: 85,
                b: 105,
                a: 255,
            },
        ));
    }

    // ── Relationships (drawn behind nodes) ───────────────────────────────────
    for rel in &diagram.relationships {
        instructions.push(PaintInstruction::Path(line_path(
            &rel.points,
            "#6b7280",
            1.5,
        )));
        // Arrowhead on the last segment
        if rel.points.len() >= 2 {
            let tip = &rel.points[rel.points.len() - 1];
            let prev = &rel.points[rel.points.len() - 2];
            instructions.push(PaintInstruction::Path(structural_arrowhead(
                prev, tip, &rel.kind,
            )));
        }
        if let Some((ref pos, ref lbl)) = rel.label {
            text_children.push(text_node(
                lbl,
                pos.x - 40.0,
                pos.y - ls / 2.0,
                80.0,
                ls * 1.2,
                lf.clone(),
                Color {
                    r: 55,
                    g: 65,
                    b: 81,
                    a: 255,
                },
            ));
        }
        if rel.points.len() >= 2 {
            let start = &rel.points[0];
            let end = &rel.points[rel.points.len() - 1];
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let ux = dx / len;
            let uy = dy / len;
            if let Some(ref multiplicity) = rel.from_mult {
                text_children.push(text_node(
                    multiplicity,
                    start.x + ux * 18.0 - 20.0,
                    start.y + uy * 18.0 + 4.0,
                    40.0,
                    ls * 1.2,
                    lf.clone(),
                    Color {
                        r: 55,
                        g: 65,
                        b: 81,
                        a: 255,
                    },
                ));
            }
            if let Some(ref multiplicity) = rel.to_mult {
                text_children.push(text_node(
                    multiplicity,
                    end.x - ux * 18.0 - 20.0,
                    end.y - uy * 18.0 + 4.0,
                    40.0,
                    ls * 1.2,
                    lf.clone(),
                    Color {
                        r: 55,
                        g: 65,
                        b: 81,
                        a: 255,
                    },
                ));
            }
        }
    }

    // ── Node boxes ───────────────────────────────────────────────────────────
    for node in &diagram.nodes {
        let header_height = node
            .compartments
            .first()
            .map(|compartment| compartment.y_offset)
            .unwrap_or(node.height);
        let mut node_font = lf.clone();
        node_font.size = node.style.font_size;
        node_font.weight = node.style.font_weight;
        node_font.italic = node.style.font_italic;
        node_font.family.clone_from(&node.style.font_family);
        // Outer rect
        instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: node.x,
            y: node.y,
            width: node.width,
            height: node.height,
            fill: Some(node.style.fill.clone()),
            stroke: Some(node.style.stroke.clone()),
            stroke_width: Some(node.style.stroke_width),
            corner_radius: Some(node.style.corner_radius),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
        // Header divider
        instructions.push(PaintInstruction::Path(line_path(
            &[
                Point {
                    x: node.x,
                    y: node.y + header_height,
                },
                Point {
                    x: node.x + node.width,
                    y: node.y + header_height,
                },
            ],
            "#d1d5db",
            1.0,
        )));
        // Header text (with optional stereotype)
        let header_label = if let Some(ref st) = node.stereotype {
            format!("«{}»\n{}", st, node.header)
        } else {
            node.header.clone()
        };
        text_children.push(text_node(
            &header_label,
            node.x,
            node.y + 8.0,
            node.width,
            header_height - 8.0,
            node_font.clone(),
            css_to_color(&node.style.text_color),
        ));
        // Compartments
        for comp in &node.compartments {
            let comp_y = node.y + comp.y_offset;
            // Compartment divider
            instructions.push(PaintInstruction::Path(line_path(
                &[
                    Point {
                        x: node.x,
                        y: comp_y,
                    },
                    Point {
                        x: node.x + node.width,
                        y: comp_y,
                    },
                ],
                "#e5e7eb",
                1.0,
            )));
            // Row text
            for (i, row) in comp.rows.iter().enumerate() {
                text_children.push(text_node(
                    row,
                    node.x + 8.0,
                    comp_y + 8.0 + i as f64 * (node.style.font_size * 1.4),
                    node.width - 16.0,
                    node.style.font_size * 1.2,
                    node_font.clone(),
                    css_to_color(&node.style.text_color),
                ));
            }
        }
    }

    let text_root = PositionedNode {
        x: 0.0,
        y: 0.0,
        width: diagram.width,
        height: diagram.height,
        id: None,
        content: None,
        children: text_children,
        ext: HashMap::new(),
    };
    let text_opts = LayoutToPaintOptions {
        width: diagram.width,
        height: diagram.height,
        background: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
        device_pixel_ratio: 1.0,
        shaper: options.shaper,
        metrics: options.metrics,
        resolver: options.resolver,
    };
    let text_scene = layout_to_paint(&text_root, &text_opts);
    instructions.extend(text_scene.instructions);

    let bg = options.background;
    PaintScene {
        width: diagram.width,
        height: diagram.height,
        background: format!("rgb({},{},{})", bg.r, bg.g, bg.b),
        instructions,
        id: None,
        metadata: (!scene_metadata.is_empty()).then_some(scene_metadata),
    }
}

fn structural_arrowhead(prev: &Point, tip: &Point, kind: &RelKind) -> PaintPath {
    let dx = tip.x - prev.x;
    let dy = tip.y - prev.y;
    let len = (dx * dx + dy * dy).sqrt().max(1e-9);
    let ux = dx / len;
    let uy = dy / len;
    let size = 10.0;
    let hw = size * 0.5;
    let bx = tip.x - ux * size;
    let by = tip.y - uy * size;
    let px = -uy;
    let py = ux;
    let (fill, open) = match kind {
        RelKind::Inheritance | RelKind::Realization => ("#ffffff", true),
        RelKind::Composition => ("#374151", false),
        _ => ("#6b7280", false),
    };
    let commands = if open {
        vec![
            PathCommand::MoveTo { x: tip.x, y: tip.y },
            PathCommand::LineTo {
                x: bx + px * hw,
                y: by + py * hw,
            },
            PathCommand::MoveTo { x: tip.x, y: tip.y },
            PathCommand::LineTo {
                x: bx - px * hw,
                y: by - py * hw,
            },
        ]
    } else {
        vec![
            PathCommand::MoveTo { x: tip.x, y: tip.y },
            PathCommand::LineTo {
                x: bx + px * hw,
                y: by + py * hw,
            },
            PathCommand::LineTo {
                x: bx - px * hw,
                y: by - py * hw,
            },
            PathCommand::Close,
        ]
    };
    PaintPath {
        base: PaintBase::default(),
        commands,
        fill: if open {
            Some("none".into())
        } else {
            Some(fill.into())
        },
        fill_rule: None,
        stroke: Some("#374151".into()),
        stroke_width: Some(1.5),
        stroke_cap: None,
        stroke_join: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    }
}

// ============================================================================
// Sequence family (DG04)
// ============================================================================

/// Lower a layouted sequence diagram into backend-neutral PaintInstructions.
pub fn diagram_to_paint_sequence<S, M, R>(
    diagram: &LayoutedSequenceDiagram,
    options: &DiagramToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let mut instructions = Vec::new();
    let mut text_children = Vec::new();
    let mut central_markers = Vec::new();
    let mut scene_metadata = HashMap::new();
    if let Some(title) = &diagram.accessibility_title {
        scene_metadata.insert("accessibility.title".into(), title.clone());
    }
    if let Some(description) = &diagram.accessibility_description {
        scene_metadata.insert("accessibility.description".into(), description.clone());
    }
    let label_font = options.label_font.clone();
    let text_color = Color {
        r: 30,
        g: 41,
        b: 59,
        a: 255,
    };

    // Participant groups form the rear-most layer around their member lanes.
    for item in &diagram.items {
        if let LayoutedSequenceItem::ParticipantGroup {
            label,
            label_height,
            fill,
            x,
            y,
            width,
            height,
            ..
        } = item
        {
            instructions.push(PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: *x,
                y: *y,
                width: *width,
                height: *height,
                fill: fill.clone(),
                stroke: Some("#94a3b8".into()),
                stroke_width: Some(1.25),
                corner_radius: Some(6.0),
                stroke_dash: None,
                stroke_dash_offset: None,
            }));
            if let Some(label) = label {
                text_children.push(text_node_no_wrap(
                    label,
                    *x + 8.0,
                    *y + 6.0,
                    *width - 16.0,
                    (*label_height + 4.0).max(20.0),
                    label_font.clone(),
                    text_color,
                ));
            }
        }
    }

    // Block frames are backgrounds. Paint outer frames before nested frames
    // regardless of the order in which their closing events were laid out.
    let mut frames: Vec<&LayoutedSequenceItem> = diagram
        .items
        .iter()
        .filter(|item| matches!(item, LayoutedSequenceItem::BlockFrame { .. }))
        .collect();
    frames.sort_by_key(|item| match item {
        LayoutedSequenceItem::BlockFrame { depth, .. } => *depth,
        _ => unreachable!(),
    });
    for frame in frames {
        if let LayoutedSequenceItem::BlockFrame {
            kind,
            label,
            label_height,
            fill: frame_fill,
            x,
            y,
            width,
            height,
            ..
        } = frame
        {
            let (fill, stroke) = sequence_block_colors(kind);
            instructions.push(PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: *x,
                y: *y,
                width: *width,
                height: *height,
                fill: Some(frame_fill.as_deref().unwrap_or(fill).into()),
                stroke: (kind != &SequenceBlockKind::Rect).then(|| stroke.into()),
                stroke_width: Some(1.25),
                corner_radius: Some(4.0),
                stroke_dash: None,
                stroke_dash_offset: None,
            }));
            if kind != &SequenceBlockKind::Rect {
                let frame_label = if label.is_empty() {
                    sequence_block_name(kind).to_string()
                } else {
                    format!("{}  {label}", sequence_block_name(kind))
                };
                text_children.push(text_node_no_wrap(
                    &frame_label,
                    *x + 8.0,
                    *y + 6.0,
                    *width - 16.0,
                    *label_height,
                    label_font.clone(),
                    text_color,
                ));
            }
        }
    }

    for item in &diagram.items {
        if let LayoutedSequenceItem::BlockDivider {
            label,
            label_height,
            x,
            y,
            width,
        } = item
        {
            instructions.push(PaintInstruction::Path(PaintPath {
                base: PaintBase::default(),
                commands: vec![
                    PathCommand::MoveTo { x: *x, y: *y },
                    PathCommand::LineTo {
                        x: *x + *width,
                        y: *y,
                    },
                ],
                fill: None,
                fill_rule: None,
                stroke: Some("#64748b".into()),
                stroke_width: Some(1.0),
                stroke_cap: None,
                stroke_join: None,
                stroke_dash: Some(vec![4.0, 3.0]),
                stroke_dash_offset: None,
            }));
            text_children.push(text_node_no_wrap(
                label,
                *x + 8.0,
                *y + 5.0,
                *width - 16.0,
                *label_height,
                label_font.clone(),
                text_color,
            ));
        }
    }

    // Lifelines sit behind activation bars. Messages are painted afterward so
    // arrowheads remain visible where they meet an activation edge.
    for item in &diagram.items {
        if let LayoutedSequenceItem::Lifeline { x, y1, y2, .. } = item {
            instructions.push(PaintInstruction::Path(PaintPath {
                base: PaintBase::default(),
                commands: vec![
                    PathCommand::MoveTo { x: *x, y: *y1 },
                    PathCommand::LineTo { x: *x, y: *y2 },
                ],
                fill: None,
                fill_rule: None,
                stroke: Some("#94a3b8".into()),
                stroke_width: Some(1.25),
                stroke_cap: Some(StrokeCap::Round),
                stroke_join: None,
                stroke_dash: Some(vec![5.0, 5.0]),
                stroke_dash_offset: None,
            }));
        }
    }

    for item in &diagram.items {
        if let LayoutedSequenceItem::Activation { x, y1, y2, .. } = item {
            instructions.push(PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: *x,
                y: *y1,
                width: 12.0,
                height: (*y2 - *y1).max(4.0),
                fill: Some("#dbeafe".into()),
                stroke: Some("#2563eb".into()),
                stroke_width: Some(1.0),
                corner_radius: Some(1.0),
                stroke_dash: None,
                stroke_dash_offset: None,
            }));
        }
    }

    for item in &diagram.items {
        if let LayoutedSequenceItem::Message {
            from_x,
            to_x,
            y,
            label,
            label_height,
            line_style,
            arrowhead,
            bidirectional,
            central_connection,
            number,
        } = item
        {
            let (
                commands,
                source_previous,
                source_tip,
                destination_previous,
                destination_tip,
                label_x,
                label_width,
            ) = if (*from_x - *to_x).abs() < 0.1 {
                let loop_width = 46.0;
                (
                    vec![
                        PathCommand::MoveTo { x: *from_x, y: *y },
                        PathCommand::LineTo {
                            x: *from_x + loop_width,
                            y: *y,
                        },
                        PathCommand::LineTo {
                            x: *from_x + loop_width,
                            y: *y + 26.0,
                        },
                        PathCommand::LineTo {
                            x: *from_x,
                            y: *y + 26.0,
                        },
                    ],
                    Point {
                        x: *from_x + loop_width,
                        y: *y,
                    },
                    Point { x: *from_x, y: *y },
                    Point {
                        x: *from_x + loop_width,
                        y: *y + 26.0,
                    },
                    Point {
                        x: *from_x,
                        y: *y + 26.0,
                    },
                    *from_x + 8.0,
                    loop_width + 80.0,
                )
            } else {
                let left = from_x.min(*to_x);
                (
                    vec![
                        PathCommand::MoveTo { x: *from_x, y: *y },
                        PathCommand::LineTo { x: *to_x, y: *y },
                    ],
                    Point { x: *to_x, y: *y },
                    Point { x: *from_x, y: *y },
                    Point { x: *from_x, y: *y },
                    Point { x: *to_x, y: *y },
                    left,
                    (*to_x - *from_x).abs(),
                )
            };
            instructions.push(PaintInstruction::Path(PaintPath {
                base: PaintBase::default(),
                commands,
                fill: None,
                fill_rule: None,
                stroke: Some("#334155".into()),
                stroke_width: Some(1.5),
                stroke_cap: Some(StrokeCap::Round),
                stroke_join: Some(StrokeJoin::Round),
                stroke_dash: match line_style {
                    SequenceLineStyle::Solid => None,
                    SequenceLineStyle::Dotted => Some(vec![5.0, 4.0]),
                },
                stroke_dash_offset: None,
            }));
            let reverse = matches!(
                arrowhead,
                SequenceArrowhead::ReverseFilledTop
                    | SequenceArrowhead::ReverseFilledBottom
                    | SequenceArrowhead::ReverseStickTop
                    | SequenceArrowhead::ReverseStickBottom
            );
            if reverse {
                instructions.extend(sequence_arrowhead(&source_previous, &source_tip, arrowhead));
            } else {
                instructions.extend(sequence_arrowhead(
                    &destination_previous,
                    &destination_tip,
                    arrowhead,
                ));
            }
            if *bidirectional {
                instructions.extend(sequence_arrowhead(&source_previous, &source_tip, arrowhead));
            }
            for point in match central_connection {
                SequenceCentralConnection::None => vec![],
                SequenceCentralConnection::Source => vec![source_tip],
                SequenceCentralConnection::Destination => vec![destination_tip],
                SequenceCentralConnection::Both => vec![source_tip, destination_tip],
            } {
                central_markers.push(point);
            }
            let rendered_label = match number {
                Some(number) => format!("{}. {label}", format_sequence_number(*number)),
                None => label.clone(),
            };
            text_children.push(text_node_no_wrap(
                &rendered_label,
                label_x,
                *y - *label_height - 6.0,
                label_width.max(80.0),
                *label_height,
                label_font.clone(),
                text_color,
            ));
        }
    }

    for item in &diagram.items {
        match item {
            LayoutedSequenceItem::Note {
                x,
                y,
                width,
                height,
                text,
            } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: Some("#fef9c3".into()),
                    stroke: Some("#ca8a04".into()),
                    stroke_width: Some(1.25),
                    corner_radius: Some(3.0),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                text_children.push(text_node_no_wrap(
                    text,
                    *x + 8.0,
                    *y + 8.0,
                    *width - 16.0,
                    *height - 12.0,
                    label_font.clone(),
                    text_color,
                ));
            }
            LayoutedSequenceItem::ParticipantBox {
                id,
                label,
                label_height,
                mirrored,
                kind,
                links,
                properties,
                details_reference,
                x,
                y,
                width,
                height,
                ..
            } => {
                if !mirrored {
                    for link in links {
                        scene_metadata.insert(
                            format!("sequence.participant.{id}.link.{}", link.label),
                            link.url.clone(),
                        );
                    }
                    for property in properties {
                        scene_metadata.insert(
                            format!("sequence.participant.{id}.property.{}", property.name),
                            property.value_json.clone(),
                        );
                    }
                    if let Some(reference) = details_reference {
                        scene_metadata.insert(
                            format!("sequence.participant.{id}.details_reference"),
                            reference.clone(),
                        );
                    }
                }
                let specialized = !matches!(
                    kind,
                    SequenceParticipantKind::Participant | SequenceParticipantKind::Actor
                );
                if kind == &SequenceParticipantKind::Actor {
                    instructions.extend(sequence_actor_symbol(*x + *width / 2.0, *y + 19.0));
                } else {
                    instructions.push(PaintInstruction::Rect(PaintRect {
                        base: PaintBase::default(),
                        x: *x,
                        y: *y,
                        width: *width,
                        height: *height,
                        fill: Some(if kind == &SequenceParticipantKind::Participant {
                            "#eff6ff".into()
                        } else {
                            "#f0fdfa".into()
                        }),
                        stroke: Some(if kind == &SequenceParticipantKind::Participant {
                            "#2563eb".into()
                        } else {
                            "#0f766e".into()
                        }),
                        stroke_width: Some(1.5),
                        corner_radius: Some(5.0),
                        stroke_dash: None,
                        stroke_dash_offset: None,
                    }));
                }
                if specialized {
                    instructions.extend(sequence_participant_icon(
                        kind,
                        *x + 24.0,
                        *y + height / 2.0,
                    ));
                }
                let embedded_icon = sequence_embedded_icon_name(properties);
                if let Some(icon) = embedded_icon {
                    instructions.extend(sequence_embedded_icon(
                        icon,
                        *x + *width - 17.0,
                        *y + 16.0,
                    ));
                }
                text_children.push(text_node_no_wrap(
                    label,
                    *x + if specialized { 44.0 } else { 8.0 },
                    if kind == &SequenceParticipantKind::Actor {
                        *y + *height - *label_height - 4.0
                    } else {
                        *y + 11.0
                    },
                    *width
                        - if specialized { 50.0 } else { 16.0 }
                        - if embedded_icon.is_some() { 20.0 } else { 0.0 },
                    (*label_height + 4.0).min(*height - 12.0),
                    label_font.clone(),
                    text_color,
                ));
            }
            _ => {}
        }
    }

    for point in central_markers {
        instructions.push(PaintInstruction::Ellipse(PaintEllipse {
            base: PaintBase::default(),
            cx: point.x,
            cy: point.y,
            rx: 5.0,
            ry: 5.0,
            fill: Some("#ffffff".into()),
            stroke: Some("#334155".into()),
            stroke_width: Some(1.5),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
    }

    if let Some(title) = &diagram.title {
        text_children.push(text_node(
            title,
            20.0,
            10.0,
            diagram.width - 40.0,
            26.0,
            options.title_font.clone(),
            text_color,
        ));
    }

    let text_root = PositionedNode {
        x: 0.0,
        y: 0.0,
        width: diagram.width,
        height: diagram.height,
        id: None,
        content: None,
        children: text_children,
        ext: HashMap::new(),
    };
    let text_scene = layout_to_paint(
        &text_root,
        &LayoutToPaintOptions {
            width: diagram.width,
            height: diagram.height,
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            device_pixel_ratio: 1.0,
            shaper: options.shaper,
            metrics: options.metrics,
            resolver: options.resolver,
        },
    );
    instructions.extend(text_scene.instructions);

    let background = options.background;
    PaintScene {
        width: diagram.width,
        height: diagram.height,
        background: format!("rgb({},{},{})", background.r, background.g, background.b),
        instructions,
        id: None,
        metadata: (!scene_metadata.is_empty()).then_some(scene_metadata),
    }
}

fn format_sequence_number(number: f64) -> String {
    let formatted = format!("{number:.2}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn sequence_block_name(kind: &SequenceBlockKind) -> &'static str {
    match kind {
        SequenceBlockKind::Loop => "loop",
        SequenceBlockKind::Rect => "rect",
        SequenceBlockKind::Opt => "opt",
        SequenceBlockKind::Alt => "alt",
        SequenceBlockKind::Par => "par",
        SequenceBlockKind::ParOver => "par_over",
        SequenceBlockKind::Critical => "critical",
        SequenceBlockKind::Break => "break",
    }
}

fn sequence_actor_symbol(cx: f64, cy: f64) -> Vec<PaintInstruction> {
    vec![
        PaintInstruction::Ellipse(PaintEllipse {
            base: PaintBase::default(),
            cx,
            cy: cy - 10.0,
            rx: 5.0,
            ry: 5.0,
            fill: Some("#ffffff".into()),
            stroke: Some("#16a34a".into()),
            stroke_width: Some(1.5),
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
        PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: cx, y: cy - 5.0 },
                PathCommand::LineTo { x: cx, y: cy + 8.0 },
                PathCommand::MoveTo { x: cx - 9.0, y: cy },
                PathCommand::LineTo { x: cx + 9.0, y: cy },
                PathCommand::MoveTo { x: cx, y: cy + 8.0 },
                PathCommand::LineTo {
                    x: cx - 8.0,
                    y: cy + 17.0,
                },
                PathCommand::MoveTo { x: cx, y: cy + 8.0 },
                PathCommand::LineTo {
                    x: cx + 8.0,
                    y: cy + 17.0,
                },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#16a34a".into()),
            stroke_width: Some(1.5),
            stroke_cap: Some(StrokeCap::Round),
            stroke_join: Some(StrokeJoin::Round),
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
    ]
}

fn sequence_participant_icon(
    kind: &SequenceParticipantKind,
    cx: f64,
    cy: f64,
) -> Vec<PaintInstruction> {
    let ellipse = |rx: f64, ry: f64| {
        PaintInstruction::Ellipse(PaintEllipse {
            base: PaintBase::default(),
            cx,
            cy,
            rx,
            ry,
            fill: Some("#ffffff".into()),
            stroke: Some("#0f766e".into()),
            stroke_width: Some(1.5),
            stroke_dash: None,
            stroke_dash_offset: None,
        })
    };
    let path = |commands| {
        PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands,
            fill: None,
            fill_rule: None,
            stroke: Some("#0f766e".into()),
            stroke_width: Some(1.5),
            stroke_cap: Some(StrokeCap::Round),
            stroke_join: Some(StrokeJoin::Round),
            stroke_dash: None,
            stroke_dash_offset: None,
        })
    };
    match kind {
        SequenceParticipantKind::Boundary => vec![
            ellipse(9.0, 9.0),
            path(vec![
                PathCommand::MoveTo { x: cx + 9.0, y: cy },
                PathCommand::LineTo {
                    x: cx + 16.0,
                    y: cy,
                },
                PathCommand::MoveTo {
                    x: cx + 16.0,
                    y: cy - 13.0,
                },
                PathCommand::LineTo {
                    x: cx + 16.0,
                    y: cy + 13.0,
                },
            ]),
        ],
        SequenceParticipantKind::Control => vec![
            ellipse(11.0, 11.0),
            path(vec![
                PathCommand::MoveTo {
                    x: cx - 8.0,
                    y: cy - 10.0,
                },
                PathCommand::LineTo {
                    x: cx - 1.0,
                    y: cy - 15.0,
                },
                PathCommand::LineTo {
                    x: cx + 1.0,
                    y: cy - 8.0,
                },
            ]),
        ],
        SequenceParticipantKind::Entity => vec![
            ellipse(10.0, 10.0),
            path(vec![
                PathCommand::MoveTo {
                    x: cx - 12.0,
                    y: cy + 13.0,
                },
                PathCommand::LineTo {
                    x: cx + 12.0,
                    y: cy + 13.0,
                },
            ]),
        ],
        SequenceParticipantKind::Database => vec![ellipse(12.0, 15.0)],
        SequenceParticipantKind::Collections => vec![
            PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: cx - 9.0,
                y: cy - 13.0,
                width: 20.0,
                height: 22.0,
                fill: Some("#ffffff".into()),
                stroke: Some("#0f766e".into()),
                stroke_width: Some(1.25),
                corner_radius: Some(2.0),
                stroke_dash: None,
                stroke_dash_offset: None,
            }),
            PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: cx - 13.0,
                y: cy - 9.0,
                width: 20.0,
                height: 22.0,
                fill: None,
                stroke: Some("#0f766e".into()),
                stroke_width: Some(1.25),
                corner_radius: Some(2.0),
                stroke_dash: None,
                stroke_dash_offset: None,
            }),
        ],
        SequenceParticipantKind::Queue => vec![PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: cx - 14.0,
            y: cy - 9.0,
            width: 28.0,
            height: 18.0,
            fill: Some("#ffffff".into()),
            stroke: Some("#0f766e".into()),
            stroke_width: Some(1.5),
            corner_radius: Some(9.0),
            stroke_dash: None,
            stroke_dash_offset: None,
        })],
        SequenceParticipantKind::Participant | SequenceParticipantKind::Actor => vec![],
    }
}

fn sequence_embedded_icon_name(properties: &[SequenceProperty]) -> Option<&str> {
    properties
        .iter()
        .find(|property| property.name == "icon")
        .and_then(|property| {
            property
                .value_json
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
        })
        .and_then(|value| value.strip_prefix('@'))
        .filter(|value| matches!(*value, "clock" | "computer"))
}

fn sequence_embedded_icon(name: &str, cx: f64, cy: f64) -> Vec<PaintInstruction> {
    let path = |commands| {
        PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands,
            fill: None,
            fill_rule: None,
            stroke: Some("#475569".into()),
            stroke_width: Some(1.25),
            stroke_cap: Some(StrokeCap::Round),
            stroke_join: Some(StrokeJoin::Round),
            stroke_dash: None,
            stroke_dash_offset: None,
        })
    };
    match name {
        "clock" => vec![
            PaintInstruction::Ellipse(PaintEllipse {
                base: PaintBase::default(),
                cx,
                cy,
                rx: 7.0,
                ry: 7.0,
                fill: Some("#ffffff".into()),
                stroke: Some("#475569".into()),
                stroke_width: Some(1.25),
                stroke_dash: None,
                stroke_dash_offset: None,
            }),
            path(vec![
                PathCommand::MoveTo { x: cx, y: cy - 4.0 },
                PathCommand::LineTo { x: cx, y: cy },
                PathCommand::LineTo {
                    x: cx + 3.0,
                    y: cy + 2.0,
                },
            ]),
        ],
        "computer" => vec![
            PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: cx - 8.0,
                y: cy - 6.0,
                width: 16.0,
                height: 11.0,
                fill: Some("#ffffff".into()),
                stroke: Some("#475569".into()),
                stroke_width: Some(1.25),
                corner_radius: Some(1.0),
                stroke_dash: None,
                stroke_dash_offset: None,
            }),
            path(vec![
                PathCommand::MoveTo { x: cx, y: cy + 5.0 },
                PathCommand::LineTo { x: cx, y: cy + 8.0 },
                PathCommand::MoveTo {
                    x: cx - 5.0,
                    y: cy + 8.0,
                },
                PathCommand::LineTo {
                    x: cx + 5.0,
                    y: cy + 8.0,
                },
            ]),
        ],
        _ => vec![],
    }
}

fn sequence_block_colors(kind: &SequenceBlockKind) -> (&'static str, &'static str) {
    match kind {
        SequenceBlockKind::Rect => ("#fff7ed", "#ea580c"),
        SequenceBlockKind::Break => ("#fff1f2", "#e11d48"),
        SequenceBlockKind::Critical => ("#fefce8", "#ca8a04"),
        SequenceBlockKind::Par | SequenceBlockKind::ParOver => ("#f0fdfa", "#0f766e"),
        _ => ("transparent", "#64748b"),
    }
}

fn sequence_arrowhead(
    previous: &Point,
    tip: &Point,
    arrowhead: &SequenceArrowhead,
) -> Vec<PaintInstruction> {
    let dx = tip.x - previous.x;
    let dy = tip.y - previous.y;
    let length = (dx * dx + dy * dy).sqrt().max(1e-9);
    let ux = dx / length;
    let uy = dy / length;
    let px = -uy;
    let py = ux;
    let back_x = tip.x - ux * 10.0;
    let back_y = tip.y - uy * 10.0;
    let left = Point {
        x: back_x + px * 5.0,
        y: back_y + py * 5.0,
    };
    let right = Point {
        x: back_x - px * 5.0,
        y: back_y - py * 5.0,
    };
    let (top, bottom) = if left.y <= right.y {
        (&left, &right)
    } else {
        (&right, &left)
    };
    let commands = match arrowhead {
        SequenceArrowhead::Open => vec![
            PathCommand::MoveTo {
                x: left.x,
                y: left.y,
            },
            PathCommand::LineTo { x: tip.x, y: tip.y },
            PathCommand::LineTo {
                x: right.x,
                y: right.y,
            },
        ],
        SequenceArrowhead::Filled => vec![
            PathCommand::MoveTo { x: tip.x, y: tip.y },
            PathCommand::LineTo {
                x: left.x,
                y: left.y,
            },
            PathCommand::LineTo {
                x: right.x,
                y: right.y,
            },
            PathCommand::Close,
        ],
        SequenceArrowhead::Cross => vec![
            PathCommand::MoveTo {
                x: left.x,
                y: left.y,
            },
            PathCommand::LineTo {
                x: right.x,
                y: right.y,
            },
            PathCommand::MoveTo {
                x: back_x + px * 5.0,
                y: back_y + py * 5.0,
            },
            PathCommand::LineTo {
                x: tip.x - px * 5.0,
                y: tip.y - py * 5.0,
            },
        ],
        SequenceArrowhead::Point => vec![
            PathCommand::MoveTo {
                x: left.x,
                y: left.y,
            },
            PathCommand::LineTo { x: tip.x, y: tip.y },
            PathCommand::LineTo {
                x: right.x,
                y: right.y,
            },
            PathCommand::Close,
        ],
        SequenceArrowhead::FilledTop | SequenceArrowhead::ReverseFilledTop => vec![
            PathCommand::MoveTo { x: tip.x, y: tip.y },
            PathCommand::LineTo { x: top.x, y: top.y },
            PathCommand::LineTo {
                x: back_x,
                y: back_y,
            },
            PathCommand::Close,
        ],
        SequenceArrowhead::FilledBottom | SequenceArrowhead::ReverseFilledBottom => vec![
            PathCommand::MoveTo { x: tip.x, y: tip.y },
            PathCommand::LineTo {
                x: bottom.x,
                y: bottom.y,
            },
            PathCommand::LineTo {
                x: back_x,
                y: back_y,
            },
            PathCommand::Close,
        ],
        SequenceArrowhead::StickTop | SequenceArrowhead::ReverseStickTop => vec![
            PathCommand::MoveTo { x: top.x, y: top.y },
            PathCommand::LineTo { x: tip.x, y: tip.y },
        ],
        SequenceArrowhead::StickBottom | SequenceArrowhead::ReverseStickBottom => vec![
            PathCommand::MoveTo {
                x: bottom.x,
                y: bottom.y,
            },
            PathCommand::LineTo { x: tip.x, y: tip.y },
        ],
    };
    vec![PaintInstruction::Path(PaintPath {
        base: PaintBase::default(),
        commands,
        fill: match arrowhead {
            SequenceArrowhead::Filled
            | SequenceArrowhead::Point
            | SequenceArrowhead::FilledTop
            | SequenceArrowhead::FilledBottom
            | SequenceArrowhead::ReverseFilledTop
            | SequenceArrowhead::ReverseFilledBottom => Some("#334155".into()),
            _ => None,
        },
        fill_rule: None,
        stroke: Some("#334155".into()),
        stroke_width: Some(1.5),
        stroke_cap: Some(StrokeCap::Round),
        stroke_join: Some(StrokeJoin::Round),
        stroke_dash: None,
        stroke_dash_offset: None,
    })]
}

// ============================================================================
// Temporal family (DG04)
// ============================================================================

fn git_commit_symbol_instructions(
    x: f64,
    y: f64,
    symbol: &GitCommitSymbol,
) -> Vec<PaintInstruction> {
    let ellipse = |cx: f64, cy: f64, radius: f64, fill: Option<String>, stroke: String| {
        PaintInstruction::Ellipse(PaintEllipse {
            base: PaintBase::default(),
            cx,
            cy,
            rx: radius,
            ry: radius,
            fill,
            stroke: Some(stroke),
            stroke_width: Some(2.0),
            stroke_dash: None,
            stroke_dash_offset: None,
        })
    };
    let normal = || ellipse(x, y, 8.0, Some("#3b82f6".into()), "#1d4ed8".into());

    match symbol {
        GitCommitSymbol::Normal => vec![normal()],
        GitCommitSymbol::Reverse => vec![
            normal(),
            PaintInstruction::Path(PaintPath {
                base: PaintBase::default(),
                commands: vec![
                    PathCommand::MoveTo { x: x - 4.0, y: y - 4.0 },
                    PathCommand::LineTo { x: x + 4.0, y: y + 4.0 },
                    PathCommand::MoveTo { x: x - 4.0, y: y + 4.0 },
                    PathCommand::LineTo { x: x + 4.0, y: y - 4.0 },
                ],
                fill: None,
                fill_rule: None,
                stroke: Some("#ffffff".into()),
                stroke_width: Some(2.0),
                stroke_cap: Some(StrokeCap::Round),
                stroke_join: Some(StrokeJoin::Round),
                stroke_dash: None,
                stroke_dash_offset: None,
            }),
        ],
        GitCommitSymbol::Highlight => vec![
            PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: x - 10.0,
                y: y - 10.0,
                width: 20.0,
                height: 20.0,
                fill: Some("#1d4ed8".into()),
                stroke: Some("#1e3a8a".into()),
                stroke_width: Some(2.0),
                corner_radius: Some(1.0),
                stroke_dash: None,
                stroke_dash_offset: None,
            }),
            PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: x - 6.0,
                y: y - 6.0,
                width: 12.0,
                height: 12.0,
                fill: Some("#93c5fd".into()),
                stroke: None,
                stroke_width: None,
                corner_radius: Some(0.0),
                stroke_dash: None,
                stroke_dash_offset: None,
            }),
        ],
        GitCommitSymbol::Merge => vec![
            normal(),
            ellipse(x, y, 5.0, None, "#ffffff".into()),
        ],
        GitCommitSymbol::CherryPick => vec![
            normal(),
            ellipse(x - 3.0, y + 2.0, 2.0, Some("#ffffff".into()), "#ffffff".into()),
            ellipse(x + 3.0, y + 2.0, 2.0, Some("#ffffff".into()), "#ffffff".into()),
            PaintInstruction::Path(PaintPath {
                base: PaintBase::default(),
                commands: vec![
                    PathCommand::MoveTo { x: x - 3.0, y: y + 2.0 },
                    PathCommand::LineTo { x, y: y - 4.0 },
                    PathCommand::LineTo { x: x + 3.0, y: y + 2.0 },
                ],
                fill: None,
                fill_rule: None,
                stroke: Some("#ffffff".into()),
                stroke_width: Some(1.5),
                stroke_cap: Some(StrokeCap::Round),
                stroke_join: Some(StrokeJoin::Round),
                stroke_dash: None,
                stroke_dash_offset: None,
            }),
        ],
    }
}

/// Lower a [`LayoutedTemporalDiagram`] into a [`PaintScene`].
pub fn diagram_to_paint_temporal<S, M, R>(
    diagram: &LayoutedTemporalDiagram,
    options: &DiagramToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let mut instructions: Vec<PaintInstruction> = Vec::new();
    let mut text_children: Vec<PositionedNode> = Vec::new();
    let lf = options.label_font.clone();
    let ls = lf.size;

    for item in &diagram.items {
        match item {
            LayoutedTemporalItem::TemporalTitle {
                x,
                y,
                width,
                height,
                label,
            } => {
                text_children.push(text_node(
                    label,
                    *x + 8.0,
                    *y + 6.0,
                    *width - 16.0,
                    *height - 12.0,
                    options.title_font.clone(),
                    Color {
                        r: 17,
                        g: 24,
                        b: 39,
                        a: 255,
                    },
                ));
            }
            LayoutedTemporalItem::TimelineSpine { x1, y1, x2, y2 } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x1, y: *y1 }, Point { x: *x2, y: *y2 }],
                    "#475569", 3.0,
                )));
            }
            LayoutedTemporalItem::TimelineSection { x, y, width, height, label } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(), x: *x, y: *y, width: *width, height: *height,
                    fill: Some("#0f172a".into()), stroke: None, stroke_width: None,
                    corner_radius: Some(15.0), stroke_dash: None, stroke_dash_offset: None,
                }));
                text_children.push(text_node(
                    label, *x + 10.0, *y + 6.0, *width - 20.0, *height - 12.0, lf.clone(),
                    Color { r: 255, g: 255, b: 255, a: 255 },
                ));
            }
            LayoutedTemporalItem::TimelinePeriod {
                x, y, width, height, label, events, color_index,
            } => {
                const FILLS: [&str; 6] = [
                    "#dbeafe", "#dcfce7", "#fef3c7", "#fae8ff", "#ffe4e6", "#cffafe",
                ];
                const STROKES: [&str; 6] = [
                    "#2563eb", "#16a34a", "#d97706", "#a21caf", "#e11d48", "#0891b2",
                ];
                let palette_index = *color_index % FILLS.len();
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(), x: *x, y: *y, width: *width, height: *height,
                    fill: Some(FILLS[palette_index].into()),
                    stroke: Some(STROKES[palette_index].into()), stroke_width: Some(2.0),
                    corner_radius: Some(8.0), stroke_dash: None, stroke_dash_offset: None,
                }));
                text_children.push(text_node(
                    label, *x + 12.0, *y + 8.0, *width - 24.0, 22.0,
                    options.title_font.clone(), Color { r: 15, g: 23, b: 42, a: 255 },
                ));
                if !events.is_empty() {
                    let event_text = events.iter().map(|event| format!("- {event}"))
                        .collect::<Vec<_>>().join("\n");
                    text_children.push(text_node(
                        &event_text, *x + 12.0, *y + 32.0, *width - 24.0,
                        (*height - 38.0).max(12.0), lf.clone(),
                        Color { r: 51, g: 65, b: 85, a: 255 },
                    ));
                }
            }
            LayoutedTemporalItem::JourneyTitle {
                x,
                y,
                width,
                height,
                label,
                font_size,
                font_family,
                color,
            } => {
                let mut title_font = options.title_font.clone();
                if let Some(size) = font_size {
                    title_font.size = *size;
                }
                if let Some(family) = font_family {
                    title_font.family.clone_from(family);
                }
                text_children.push(text_node(
                    label,
                    *x + 8.0,
                    *y + 6.0,
                    *width - 16.0,
                    *height - 12.0,
                    title_font,
                    color.as_deref().map(css_to_color).unwrap_or(Color {
                        r: 17,
                        g: 24,
                        b: 39,
                        a: 255,
                    }),
                ));
            }
            LayoutedTemporalItem::JourneySection {
                x,
                y,
                width,
                height,
                label,
                fill,
                text_color,
            } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: Some(fill.clone()),
                    stroke: None,
                    stroke_width: None,
                    corner_radius: Some(3.0),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                text_children.push(text_node(
                    label,
                    *x + 8.0,
                    *y + 6.0,
                    *width - 16.0,
                    *height - 12.0,
                    options.title_font.clone(),
                    css_to_color(text_color),
                ));
            }
            LayoutedTemporalItem::TimeAxisSpine { x1, y1, x2, y2 } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x1, y: *y1 }, Point { x: *x2, y: *y2 }],
                    "#374151",
                    1.5,
                )));
            }
            LayoutedTemporalItem::TimeAxisTick { x, y, label, label_above } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x, y: *y - 4.0 }, Point { x: *x, y: *y }],
                    "#374151",
                    1.0,
                )));
                text_children.push(text_node(
                    label,
                    x - 20.0,
                    if *label_above { *y - ls * 1.2 - 2.0 } else { *y + 2.0 },
                    40.0,
                    ls * 1.2,
                    lf.clone(),
                    Color {
                        r: 107,
                        g: 114,
                        b: 128,
                        a: 255,
                    },
                ));
            }
            LayoutedTemporalItem::SectionHeader {
                x,
                y,
                width,
                height,
                label,
            } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: Some("#f3f4f6".into()),
                    stroke: None,
                    stroke_width: None,
                    corner_radius: None,
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                text_children.push(text_node(
                    label,
                    *x + 8.0,
                    *y + (*height - ls) / 2.0,
                    *width - 16.0,
                    *height,
                    options.title_font.clone(),
                    Color {
                        r: 17,
                        g: 24,
                        b: 39,
                        a: 255,
                    },
                ));
            }
            LayoutedTemporalItem::TaskBar {
                x,
                y,
                width,
                height,
                tags,
                label,
            } => {
                let (fill, stroke, stroke_width) = gantt_task_colors(tags);
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: Some(fill.into()),
                    stroke: stroke.map(str::to_string),
                    stroke_width,
                    corner_radius: Some(2.0),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                text_children.push(text_node(
                    label,
                    *x + 4.0,
                    *y + (*height - ls) / 2.0,
                    (*width - 8.0).max(8.0),
                    ls * 1.2,
                    lf.clone(),
                    Color {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 255,
                    },
                ));
            }
            LayoutedTemporalItem::MilestoneMarker { x, y, tags, label } => {
                let s = 8.0;
                let (fill, stroke, stroke_width) = gantt_task_colors(tags);
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo { x: *x, y: y - s },
                        PathCommand::LineTo { x: x + s, y: *y },
                        PathCommand::LineTo { x: *x, y: y + s },
                        PathCommand::LineTo { x: x - s, y: *y },
                        PathCommand::Close,
                    ],
                    fill: Some(fill.into()),
                    fill_rule: None,
                    stroke: stroke.map(str::to_string),
                    stroke_width,
                    stroke_cap: None,
                    stroke_join: None,
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                text_children.push(text_node(
                    label,
                    x - 40.0,
                    y + s + 2.0,
                    80.0,
                    ls * 1.2,
                    lf.clone(),
                    Color {
                        r: 17,
                        g: 24,
                        b: 39,
                        a: 255,
                    },
                ));
            }
            LayoutedTemporalItem::VerticalMarker { x, y1, y2, label } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x, y: *y1 }, Point { x: *x, y: *y2 }],
                    "#6b7280",
                    2.0,
                )));
                text_children.push(text_node(
                    label,
                    x - 60.0,
                    y2 + 2.0,
                    120.0,
                    ls * 1.2,
                    lf.clone(),
                    Color { r: 75, g: 85, b: 99, a: 255 },
                ));
            }
            LayoutedTemporalItem::TodayMarker {
                x,
                y1,
                y2,
                stroke,
                stroke_width,
                stroke_dash,
                opacity,
            } => {
                instructions.push(PaintInstruction::Group(PaintGroup {
                    base: PaintBase::default(),
                    children: vec![PaintInstruction::Path(PaintPath {
                        base: PaintBase::default(),
                        commands: vec![
                            PathCommand::MoveTo { x: *x, y: *y1 },
                            PathCommand::LineTo { x: *x, y: *y2 },
                        ],
                        fill: Some("none".into()),
                        fill_rule: None,
                        stroke: Some(stroke.clone()),
                        stroke_width: Some(*stroke_width),
                        stroke_cap: None,
                        stroke_join: None,
                        stroke_dash: stroke_dash.clone(),
                        stroke_dash_offset: None,
                    })],
                    transform: None,
                    opacity: Some(*opacity),
                }));
            }
            LayoutedTemporalItem::BranchLane {
                x1,
                y1,
                x2,
                y2,
                label_x,
                label_y,
                label_width,
                label_height,
                color,
                label,
            } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[
                        Point { x: *x1, y: *y1 },
                        Point { x: *x2, y: *y2 },
                    ],
                    color,
                    1.0,
                )));
                text_children.push(text_node(
                    label,
                    *label_x,
                    *label_y,
                    *label_width,
                    *label_height,
                    lf.clone(),
                    Color {
                        r: 55,
                        g: 65,
                        b: 81,
                        a: 255,
                    },
                ));
            }
            LayoutedTemporalItem::CommitNode {
                x,
                y,
                id: _,
                message,
                tags,
                symbol,
            } => {
                instructions.extend(git_commit_symbol_instructions(*x, *y, symbol));
                if let Some(ref msg) = message {
                    text_children.push(text_node(
                        msg,
                        x - 40.0,
                        y - ls - 10.0,
                        80.0,
                        ls * 1.2,
                        lf.clone(),
                        Color {
                            r: 55,
                            g: 65,
                            b: 81,
                            a: 255,
                        },
                    ));
                }
                if !tags.is_empty() {
                    text_children.push(text_node(
                        &tags.join(" · "),
                        x - 40.0,
                        y + 12.0,
                        80.0,
                        ls * 1.2,
                        lf.clone(),
                        Color {
                            r: 34,
                            g: 197,
                            b: 94,
                            a: 255,
                        },
                    ));
                }
            }
            LayoutedTemporalItem::GitHistoryArc {
                from_x,
                from_y,
                to_x,
                to_y,
            } => {
                let cpx = (from_x + to_x) / 2.0;
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo {
                            x: *from_x,
                            y: *from_y,
                        },
                        PathCommand::CubicTo {
                            cx1: cpx,
                            cy1: *from_y,
                            cx2: cpx,
                            cy2: *to_y,
                            x: *to_x,
                            y: *to_y,
                        },
                    ],
                    fill: Some("none".into()),
                    fill_rule: None,
                    stroke: Some("#6b7280".into()),
                    stroke_width: Some(2.0),
                    stroke_cap: Some(StrokeCap::Round),
                    stroke_join: Some(StrokeJoin::Round),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }
            LayoutedTemporalItem::JourneyActivityLine { x1, y, x2 } => {
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo { x: *x1, y: *y },
                        PathCommand::LineTo { x: *x2, y: *y },
                    ],
                    fill: Some("none".into()),
                    fill_rule: None,
                    stroke: Some("#0f172a".into()),
                    stroke_width: Some(4.0),
                    stroke_cap: Some(StrokeCap::Round),
                    stroke_join: Some(StrokeJoin::Round),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }
            LayoutedTemporalItem::JourneyTaskLine { x, y1, y2 } => {
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo { x: *x, y: *y1 },
                        PathCommand::LineTo { x: *x, y: *y2 },
                    ],
                    fill: Some("none".into()),
                    fill_rule: None,
                    stroke: Some("#64748b".into()),
                    stroke_width: Some(1.0),
                    stroke_cap: Some(StrokeCap::Round),
                    stroke_join: Some(StrokeJoin::Round),
                    stroke_dash: Some(vec![4.0, 2.0]),
                    stroke_dash_offset: None,
                }));
            }
            LayoutedTemporalItem::JourneyActor {
                x,
                y,
                width,
                height,
                color,
                label,
            } => {
                instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                    base: PaintBase::default(),
                    cx: *x,
                    cy: *y,
                    rx: 7.0,
                    ry: 7.0,
                    fill: Some(color.clone()),
                    stroke: Some("#000000".into()),
                    stroke_width: Some(1.0),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                text_children.push(text_node(
                    label,
                    x + 12.0,
                    y - height / 2.0,
                    *width,
                    *height,
                    lf.clone(),
                    Color {
                        r: 71,
                        g: 85,
                        b: 105,
                        a: 255,
                    },
                ));
            }
            LayoutedTemporalItem::JourneyTask {
                x,
                y,
                width,
                height,
                score_y,
                score,
                label,
                people: _,
                person_colors,
                font_size,
                font_family,
                fill,
                text_color,
            } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: Some(fill.clone()),
                    stroke: Some("#475569".into()),
                    stroke_width: Some(1.0),
                    corner_radius: Some(6.0),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                for (index, color) in person_colors.iter().enumerate() {
                    instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                        base: PaintBase::default(),
                        cx: x + 12.0 + index as f64 * 12.0,
                        cy: *y,
                        rx: 4.0,
                        ry: 4.0,
                        fill: Some(color.clone()),
                        stroke: Some("#000000".into()),
                        stroke_width: Some(0.75),
                        stroke_dash: None,
                        stroke_dash_offset: None,
                    }));
                }
                let face_x = x + width / 2.0;
                let face_y = *score_y;
                instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                    base: PaintBase::default(),
                    cx: face_x,
                    cy: face_y,
                    rx: 12.0,
                    ry: 12.0,
                    fill: Some("#ffffff".into()),
                    stroke: Some("#334155".into()),
                    stroke_width: Some(1.5),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                for eye_x in [face_x - 4.0, face_x + 4.0] {
                    instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                        base: PaintBase::default(),
                        cx: eye_x,
                        cy: face_y - 3.0,
                        rx: 1.25,
                        ry: 1.25,
                        fill: Some("#334155".into()),
                        stroke: None,
                        stroke_width: None,
                        stroke_dash: None,
                        stroke_dash_offset: None,
                    }));
                }
                let (mouth_start_y, mouth_control_y) = if *score > 3 {
                    (face_y + 2.0, face_y + 8.0)
                } else if *score < 3 {
                    (face_y + 7.0, face_y + 1.0)
                } else {
                    (face_y + 5.0, face_y + 5.0)
                };
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo {
                            x: face_x - 5.0,
                            y: mouth_start_y,
                        },
                        PathCommand::QuadTo {
                            cx: face_x,
                            cy: mouth_control_y,
                            x: face_x + 5.0,
                            y: mouth_start_y,
                        },
                    ],
                    fill: Some("none".into()),
                    fill_rule: None,
                    stroke: Some("#334155".into()),
                    stroke_width: Some(1.25),
                    stroke_cap: Some(StrokeCap::Round),
                    stroke_join: Some(StrokeJoin::Round),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                let mut task_font = lf.clone();
                if let Some(size) = font_size {
                    task_font.size = *size;
                }
                if let Some(family) = font_family {
                    task_font.family.clone_from(family);
                }
                text_children.push(text_node(
                    label,
                    x + 10.0,
                    y + 6.0,
                    width - 20.0,
                    height - 12.0,
                    task_font,
                    css_to_color(text_color),
                ));
            }
        }
    }

    let text_root = PositionedNode {
        x: 0.0,
        y: 0.0,
        width: diagram.width,
        height: diagram.height,
        id: None,
        content: None,
        children: text_children,
        ext: HashMap::new(),
    };
    let text_opts = LayoutToPaintOptions {
        width: diagram.width,
        height: diagram.height,
        background: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
        device_pixel_ratio: 1.0,
        shaper: options.shaper,
        metrics: options.metrics,
        resolver: options.resolver,
    };
    let text_scene = layout_to_paint(&text_root, &text_opts);
    instructions.extend(text_scene.instructions);

    let mut metadata = HashMap::new();
    if let Some(title) = &diagram.accessibility_title {
        metadata.insert("accessibility.title".to_string(), title.clone());
    }
    if let Some(description) = &diagram.accessibility_description {
        metadata.insert("accessibility.description".to_string(), description.clone());
    }
    for interaction in &diagram.interactions {
        let prefix = format!("gantt.task.{}", interaction.task_id);
        if let Some(link) = &interaction.link {
            metadata.insert(format!("{prefix}.link.url"), link.clone());
        }
        if let Some(callback) = &interaction.callback {
            metadata.insert(format!("{prefix}.callback.name"), callback.clone());
        }
        if let Some(args) = &interaction.callback_args {
            metadata.insert(format!("{prefix}.callback.args"), args.clone());
        }
        metadata.insert(
            format!("{prefix}.bounds"),
            format!(
                "{},{},{},{}",
                interaction.bounds.0,
                interaction.bounds.1,
                interaction.bounds.2,
                interaction.bounds.3
            ),
        );
    }
    let bg = options.background;
    PaintScene {
        width: diagram.width,
        height: diagram.height,
        background: format!("rgb({},{},{})", bg.r, bg.g, bg.b),
        instructions,
        id: None,
        metadata: (!metadata.is_empty()).then_some(metadata),
    }
}

fn gantt_task_colors(tags: &GanttTaskTags) -> (&'static str, Option<&'static str>, Option<f64>) {
    let fill = if tags.active {
        "#f59e0b"
    } else if tags.done {
        "#22c55e"
    } else if tags.critical {
        "#ef4444"
    } else {
        "#3b82f6"
    };
    if tags.critical {
        (fill, Some("#b91c1c"), Some(2.0))
    } else {
        (fill, None, None)
    }
}

// ============================================================================
// Geometric family (DG04)
// ============================================================================

/// Lower a [`LayoutedGeometricDiagram`] into a [`PaintScene`].
pub fn diagram_to_paint_geometric<S, M, R>(
    diagram: &LayoutedGeometricDiagram,
    options: &DiagramToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let mut instructions: Vec<PaintInstruction> = Vec::new();
    let mut text_children: Vec<PositionedNode> = Vec::new();
    let lf = options.label_font.clone();
    let ls = lf.size;

    for el in &diagram.elements {
        match el {
            GeoElement::Box {
                x,
                y,
                w,
                h,
                corner_radius,
                label,
                fill,
                stroke,
                ..
            } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x,
                    y: *y,
                    width: *w,
                    height: *h,
                    fill: Some(fill.clone().unwrap_or_else(|| "#f9fafb".into())),
                    stroke: Some(stroke.clone().unwrap_or_else(|| "#374151".into())),
                    stroke_width: Some(1.5),
                    corner_radius: Some(*corner_radius),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                if let Some(ref lbl) = label {
                    text_children.push(text_node(
                        lbl,
                        *x + 4.0,
                        y + (h - ls) / 2.0,
                        w - 8.0,
                        ls * 1.2,
                        lf.clone(),
                        Color {
                            r: 17,
                            g: 24,
                            b: 39,
                            a: 255,
                        },
                    ));
                }
            }
            GeoElement::Circle {
                cx,
                cy,
                r,
                label,
                fill,
                stroke,
                ..
            } => {
                instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                    base: PaintBase::default(),
                    cx: *cx,
                    cy: *cy,
                    rx: *r,
                    ry: *r,
                    fill: Some(fill.clone().unwrap_or_else(|| "#f9fafb".into())),
                    stroke: Some(stroke.clone().unwrap_or_else(|| "#374151".into())),
                    stroke_width: Some(1.5),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                if let Some(ref lbl) = label {
                    text_children.push(text_node(
                        lbl,
                        cx - r * 0.7,
                        cy - ls / 2.0,
                        r * 1.4,
                        ls * 1.2,
                        lf.clone(),
                        Color {
                            r: 17,
                            g: 24,
                            b: 39,
                            a: 255,
                        },
                    ));
                }
            }
            GeoElement::Line {
                x1,
                y1,
                x2,
                y2,
                arrow_end,
                arrow_start,
                stroke,
                ..
            } => {
                let stroke_color = stroke.as_deref().unwrap_or("#374151");
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x1, y: *y1 }, Point { x: *x2, y: *y2 }],
                    stroke_color,
                    1.5,
                )));
                if *arrow_end {
                    let prev = Point { x: *x1, y: *y1 };
                    let tip = Point { x: *x2, y: *y2 };
                    instructions.push(PaintInstruction::Path(simple_arrowhead(
                        &prev,
                        &tip,
                        stroke_color,
                    )));
                }
                if *arrow_start {
                    let prev = Point { x: *x2, y: *y2 };
                    let tip = Point { x: *x1, y: *y1 };
                    instructions.push(PaintInstruction::Path(simple_arrowhead(
                        &prev,
                        &tip,
                        stroke_color,
                    )));
                }
            }
            GeoElement::Arc {
                cx,
                cy,
                r,
                start_deg,
                end_deg,
                stroke,
                ..
            } => {
                let start = start_deg.to_radians();
                let end = end_deg.to_radians();
                let n =
                    (((end - start).abs() / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
                let step = (end - start) / n as f64;
                let mut cmds = vec![PathCommand::MoveTo {
                    x: cx + r * start.cos(),
                    y: cy + r * start.sin(),
                }];
                for i in 0..n {
                    let a0 = start + i as f64 * step;
                    let a1 = a0 + step;
                    let k = (4.0 / 3.0) * ((a1 - a0) / 4.0).tan();
                    let (c0s, c0c) = (a0.sin(), a0.cos());
                    let (c1s, c1c) = (a1.sin(), a1.cos());
                    cmds.push(PathCommand::CubicTo {
                        cx1: cx + r * (c0c - k * c0s),
                        cy1: cy + r * (c0s + k * c0c),
                        cx2: cx + r * (c1c + k * c1s),
                        cy2: cy + r * (c1s - k * c1c),
                        x: cx + r * c1c,
                        y: cy + r * c1s,
                    });
                }
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: cmds,
                    fill: Some("none".into()),
                    fill_rule: None,
                    stroke: Some(stroke.clone().unwrap_or_else(|| "#374151".into())),
                    stroke_width: Some(1.5),
                    stroke_cap: Some(StrokeCap::Round),
                    stroke_join: Some(StrokeJoin::Round),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }
            GeoElement::Text {
                x, y, text, align, ..
            } => {
                use layout_ir::TextAlign as LTextAlign;
                let ta = match align {
                    GeoTextAlign::Left => LTextAlign::Start,
                    GeoTextAlign::Center => LTextAlign::Center,
                    GeoTextAlign::Right => LTextAlign::End,
                };
                let est_w = text.len() as f64 * 7.5 + 8.0;
                text_children.push(PositionedNode {
                    x: *x,
                    y: y - ls,
                    width: est_w,
                    height: ls * 1.4,
                    id: None,
                    content: Some(layout_ir::Content::Text(layout_ir::TextContent {
                        value: text.clone(),
                        font: lf.clone(),
                        color: Color {
                            r: 17,
                            g: 24,
                            b: 39,
                            a: 255,
                        },
                        decoration: None,
                        max_lines: None,
                        wrap: true,
                        text_align: ta,
                    })),
                    children: Vec::new(),
                    ext: HashMap::new(),
                });
            }
        }
    }

    let text_root = PositionedNode {
        x: 0.0,
        y: 0.0,
        width: diagram.width,
        height: diagram.height,
        id: None,
        content: None,
        children: text_children,
        ext: HashMap::new(),
    };
    let text_opts = LayoutToPaintOptions {
        width: diagram.width,
        height: diagram.height,
        background: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
        device_pixel_ratio: 1.0,
        shaper: options.shaper,
        metrics: options.metrics,
        resolver: options.resolver,
    };
    let text_scene = layout_to_paint(&text_root, &text_opts);
    instructions.extend(text_scene.instructions);

    let bg = options.background;
    PaintScene {
        width: diagram.width,
        height: diagram.height,
        background: format!("rgb({},{},{})", bg.r, bg.g, bg.b),
        instructions,
        id: None,
        metadata: None,
    }
}

fn simple_arrowhead(prev: &Point, tip: &Point, stroke: &str) -> PaintPath {
    let dx = tip.x - prev.x;
    let dy = tip.y - prev.y;
    let len = (dx * dx + dy * dy).sqrt().max(1e-9);
    let ux = dx / len;
    let uy = dy / len;
    let size = 10.0;
    let hw = size * 0.5;
    let bx = tip.x - ux * size;
    let by = tip.y - uy * size;
    let px = -uy;
    let py = ux;
    PaintPath {
        base: PaintBase::default(),
        commands: vec![
            PathCommand::MoveTo { x: tip.x, y: tip.y },
            PathCommand::LineTo {
                x: bx + px * hw,
                y: by + py * hw,
            },
            PathCommand::LineTo {
                x: bx - px * hw,
                y: by - py * hw,
            },
            PathCommand::Close,
        ],
        fill: Some(stroke.into()),
        fill_rule: None,
        stroke: None,
        stroke_width: None,
        stroke_cap: None,
        stroke_join: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::{
        DiagramDirection, DiagramLabel, DiagramShape, EdgeKind, LayoutedGraphDiagram,
        LayoutedGraphEdge, LayoutedGraphNode, Point, ResolvedDiagramStyle,
    };
    use layout_ir::font_spec;
    use text_interfaces::{
        Direction, FontQuery, FontResolutionError, Glyph, ShapeOptions, ShapedRun, ShapedText,
        ShapingError,
    };

    // ── Minimal fake text backend ─────────────────────────────────────────

    #[derive(Clone)]
    struct FakeHandle;

    struct FakeResolver;
    impl FontResolver for FakeResolver {
        type Handle = FakeHandle;
        fn resolve(&self, _q: &FontQuery) -> Result<FakeHandle, FontResolutionError> {
            Ok(FakeHandle)
        }
    }

    struct FakeMetrics;
    impl FontMetrics for FakeMetrics {
        type Handle = FakeHandle;
        fn units_per_em(&self, _: &FakeHandle) -> u32 {
            1000
        }
        fn ascent(&self, _: &FakeHandle) -> i32 {
            800
        }
        fn descent(&self, _: &FakeHandle) -> i32 {
            200
        }
        fn line_gap(&self, _: &FakeHandle) -> i32 {
            0
        }
        fn x_height(&self, _: &FakeHandle) -> Option<i32> {
            Some(500)
        }
        fn cap_height(&self, _: &FakeHandle) -> Option<i32> {
            Some(700)
        }
        fn family_name(&self, _: &FakeHandle) -> String {
            "Fake".into()
        }
    }

    struct FakeShaper;
    impl TextShaper for FakeShaper {
        type Handle = FakeHandle;
        fn shape(
            &self,
            text: &str,
            _font: &FakeHandle,
            size: f32,
            opts: &ShapeOptions,
        ) -> Result<ShapedText, ShapingError> {
            if opts.direction != Direction::Ltr {
                return Err(ShapingError::UnsupportedDirection(opts.direction));
            }
            let advance = size / 2.0;
            let glyphs: Vec<Glyph> = text
                .chars()
                .enumerate()
                .map(|(i, c)| Glyph {
                    glyph_id: c as u32,
                    cluster: i as u32,
                    x_advance: advance,
                    y_advance: 0.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                })
                .collect();
            let total = glyphs.len() as f32 * advance;
            Ok(ShapedText::single(ShapedRun {
                glyphs,
                x_advance_total: total,
                font_ref: "fake:test".into(),
            }))
        }
        fn font_ref(&self, _h: &FakeHandle) -> String {
            "fake:test".into()
        }
    }

    fn make_opts<'a>(
        shaper: &'a FakeShaper,
        metrics: &'a FakeMetrics,
        resolver: &'a FakeResolver,
    ) -> DiagramToPaintOptions<'a, FakeShaper, FakeMetrics, FakeResolver> {
        DiagramToPaintOptions {
            background: Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            device_pixel_ratio: 1.0,
            label_font: font_spec("Helvetica", 14.0),
            title_font: FontSpec {
                family: "Helvetica".to_string(),
                size: 18.0,
                weight: 700,
                italic: false,
                line_height: 1.2,
            },
            shaper,
            metrics,
            resolver,
        }
    }

    fn default_style() -> ResolvedDiagramStyle {
        ResolvedDiagramStyle::default()
    }

    fn edge_style() -> ResolvedDiagramStyle {
        ResolvedDiagramStyle {
            fill: "none".to_string(),
            stroke: "#4b5563".to_string(),
            stroke_width: 2.0,
            text_color: "#374151".to_string(),
            font_size: 12.0,
            font_weight: 400,
            font_italic: false,
            font_family: "Helvetica".into(),
            corner_radius: 0.0,
        }
    }

    fn simple_layout() -> LayoutedGraphDiagram {
        LayoutedGraphDiagram {
            direction: DiagramDirection::Lr,
            requested_width: None,
            hide_empty_descriptions: false,
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            links: Vec::new(),
            groups: Vec::new(),
            width: 400.0,
            height: 200.0,
            nodes: vec![
                LayoutedGraphNode {
                    id: "A".to_string(),
                    label: DiagramLabel::new("Start"),
                    shape: DiagramShape::RoundedRect,
                    x: 24.0,
                    y: 24.0,
                    width: 96.0,
                    height: 52.0,
                    style: default_style(),
                },
                LayoutedGraphNode {
                    id: "B".to_string(),
                    label: DiagramLabel::new("End"),
                    shape: DiagramShape::RoundedRect,
                    x: 216.0,
                    y: 24.0,
                    width: 96.0,
                    height: 52.0,
                    style: default_style(),
                },
            ],
            edges: vec![LayoutedGraphEdge {
                id: None,
                from_node_id: "A".to_string(),
                to_node_id: "B".to_string(),
                kind: EdgeKind::Directed,
                points: vec![Point { x: 120.0, y: 50.0 }, Point { x: 216.0, y: 50.0 }],
                label: None,
                label_position: None,
                style: edge_style(),
            }],
        }
    }

    #[test]
    fn version_exists() {
        assert_eq!(crate::VERSION, "0.63.0");
    }

    #[test]
    fn git_commit_symbols_emit_distinct_backend_neutral_geometry() {
        let reverse = git_commit_symbol_instructions(20.0, 20.0, &GitCommitSymbol::Reverse);
        assert!(reverse.iter().any(|item| matches!(item, PaintInstruction::Path(_))));

        let highlight =
            git_commit_symbol_instructions(20.0, 20.0, &GitCommitSymbol::Highlight);
        assert_eq!(
            highlight.iter().filter(|item| matches!(item, PaintInstruction::Rect(_))).count(),
            2
        );

        let merge = git_commit_symbol_instructions(20.0, 20.0, &GitCommitSymbol::Merge);
        assert_eq!(
            merge.iter().filter(|item| matches!(item, PaintInstruction::Ellipse(_))).count(),
            2
        );

        let cherry_pick =
            git_commit_symbol_instructions(20.0, 20.0, &GitCommitSymbol::CherryPick);
        assert_eq!(
            cherry_pick.iter().filter(|item| matches!(item, PaintInstruction::Ellipse(_))).count(),
            3
        );
        assert!(cherry_pick.iter().any(|item| matches!(item, PaintInstruction::Path(_))));
    }

    #[test]
    fn sequence_messages_paint_above_activation_bars() {
        let diagram = LayoutedSequenceDiagram {
            width: 240.0,
            height: 140.0,
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            // Deliberately put the message first: layer order must not depend on
            // semantic item order.
            items: vec![
                LayoutedSequenceItem::Message {
                    from_x: 40.0,
                    to_x: 194.0,
                    y: 72.0,
                    label: "Request".into(),
                    label_height: 16.0,
                    line_style: SequenceLineStyle::Solid,
                    arrowhead: SequenceArrowhead::Filled,
                    bidirectional: false,
                    central_connection: SequenceCentralConnection::None,
                    number: None,
                },
                LayoutedSequenceItem::Activation {
                    participant: "Service".into(),
                    x: 194.0,
                    y1: 52.0,
                    y2: 112.0,
                },
            ],
        };
        let (shaper, metrics, resolver) = (FakeShaper, FakeMetrics, FakeResolver);
        let scene = diagram_to_paint_sequence(&diagram, &make_opts(&shaper, &metrics, &resolver));
        let activation_index = scene
            .instructions
            .iter()
            .position(|instruction| {
                matches!(instruction, PaintInstruction::Rect(rect) if rect.fill.as_deref() == Some("#dbeafe"))
            })
            .unwrap();
        let message_index = scene
            .instructions
            .iter()
            .position(|instruction| {
                matches!(instruction, PaintInstruction::Path(path) if path.stroke.as_deref() == Some("#334155"))
            })
            .unwrap();

        assert!(activation_index < message_index);
    }

    #[test]
    fn sequence_self_connection_markers_use_lifeline_endpoints() {
        let diagram = LayoutedSequenceDiagram {
            width: 180.0,
            height: 140.0,
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            items: vec![LayoutedSequenceItem::Message {
                from_x: 64.0,
                to_x: 64.0,
                y: 60.0,
                label: "self".into(),
                label_height: 16.0,
                line_style: SequenceLineStyle::Solid,
                arrowhead: SequenceArrowhead::Filled,
                bidirectional: false,
                central_connection: SequenceCentralConnection::Both,
                number: None,
            }],
        };
        let (shaper, metrics, resolver) = (FakeShaper, FakeMetrics, FakeResolver);
        let scene = diagram_to_paint_sequence(&diagram, &make_opts(&shaper, &metrics, &resolver));
        let markers: Vec<_> = scene
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                PaintInstruction::Ellipse(ellipse) if ellipse.rx == 5.0 && ellipse.ry == 5.0 => {
                    Some((ellipse.cx, ellipse.cy))
                }
                _ => None,
            })
            .collect();

        assert_eq!(markers, vec![(64.0, 60.0), (64.0, 86.0)]);
    }

    #[test]
    fn sequence_stereotypes_emit_distinct_icon_geometry() {
        let kinds = [
            SequenceParticipantKind::Boundary,
            SequenceParticipantKind::Control,
            SequenceParticipantKind::Entity,
            SequenceParticipantKind::Database,
            SequenceParticipantKind::Collections,
            SequenceParticipantKind::Queue,
        ];
        for kind in kinds {
            assert!(!sequence_participant_icon(&kind, 20.0, 20.0).is_empty());
        }
    }

    #[test]
    fn sequence_actor_emits_backend_neutral_stick_figure_geometry() {
        let instructions = sequence_actor_symbol(20.0, 24.0);
        assert!(matches!(instructions[0], PaintInstruction::Ellipse(_)));
        assert!(matches!(instructions[1], PaintInstruction::Path(_)));
        let PaintInstruction::Path(path) = &instructions[1] else {
            unreachable!();
        };
        assert_eq!(path.commands.len(), 8);
    }

    #[test]
    fn sequence_embedded_property_icons_emit_backend_neutral_geometry() {
        let properties = vec![SequenceProperty {
            name: "icon".into(),
            value_json: "\"@clock\"".into(),
        }];
        assert_eq!(sequence_embedded_icon_name(&properties), Some("clock"));

        let clock = sequence_embedded_icon("clock", 20.0, 20.0);
        assert!(matches!(
            clock.as_slice(),
            [PaintInstruction::Ellipse(_), PaintInstruction::Path(_)]
        ));

        let computer = sequence_embedded_icon("computer", 20.0, 20.0);
        assert!(matches!(
            computer.as_slice(),
            [PaintInstruction::Rect(_), PaintInstruction::Path(_)]
        ));
    }

    #[test]
    fn sequence_half_arrows_emit_half_head_geometry() {
        let start = Point { x: 0.0, y: 20.0 };
        let end = Point { x: 100.0, y: 20.0 };
        for arrowhead in [
            SequenceArrowhead::FilledTop,
            SequenceArrowhead::FilledBottom,
            SequenceArrowhead::StickTop,
            SequenceArrowhead::StickBottom,
            SequenceArrowhead::ReverseFilledTop,
            SequenceArrowhead::ReverseFilledBottom,
            SequenceArrowhead::ReverseStickTop,
            SequenceArrowhead::ReverseStickBottom,
        ] {
            let instructions = sequence_arrowhead(&start, &end, &arrowhead);
            assert!(matches!(
                instructions.as_slice(),
                [PaintInstruction::Path(_)]
            ));
        }
    }

    #[test]
    fn formats_decimal_sequence_numbers_without_trailing_zeroes() {
        assert_eq!(format_sequence_number(10.0), "10");
        assert_eq!(format_sequence_number(10.5), "10.5");
        assert_eq!(format_sequence_number(12.75), "12.75");
    }

    #[test]
    fn scene_dimensions_match_layout() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&simple_layout(), &opts);
        assert_eq!(scene.width, 400.0);
        assert_eq!(scene.height, 200.0);
    }

    #[test]
    fn scene_has_white_background() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&simple_layout(), &opts);
        assert_eq!(scene.background, "rgb(255, 255, 255)");
    }

    #[test]
    fn scene_is_not_empty() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&simple_layout(), &opts);
        assert!(!scene.instructions.is_empty());
    }

    #[test]
    fn journey_actors_and_scores_emit_backend_neutral_geometry() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedTemporalDiagram {
            width: 320.0,
            height: 96.0,
            accessibility_title: None,
            accessibility_description: None,
            interactions: Vec::new(),
            items: vec![
                LayoutedTemporalItem::JourneyTitle {
                    x: 0.0,
                    y: 0.0,
                    width: 320.0,
                    height: 36.0,
                    label: "Checkout".into(),
                    font_size: Some(22.0),
                    font_family: Some("Georgia".into()),
                    color: Some("#123456".into()),
                },
                LayoutedTemporalItem::JourneyActor {
                    x: 24.0,
                    y: 18.0,
                    width: 56.0,
                    height: 36.0,
                    color: "#8fbc8f".into(),
                    label: "Alice\nWonderland".into(),
                },
                LayoutedTemporalItem::JourneyActivityLine {
                    x1: 80.0,
                    y: 92.0,
                    x2: 240.0,
                },
                LayoutedTemporalItem::JourneyTaskLine {
                    x: 160.0,
                    y1: 80.0,
                    y2: 112.0,
                },
                LayoutedTemporalItem::JourneySection {
                    x: 0.0,
                    y: 28.0,
                    width: 320.0,
                    height: 32.0,
                    label: "Discovery".into(),
                    fill: "#112233".into(),
                    text_color: "#fefefe".into(),
                },
                LayoutedTemporalItem::JourneyTask {
                    x: 16.0,
                    y: 40.0,
                    width: 288.0,
                    height: 40.0,
                    score_y: 112.0,
                    score: 5,
                    label: "Find product".into(),
                    people: vec!["Alice".into()],
                    person_colors: vec!["#8fbc8f".into()],
                    font_size: Some(18.0),
                    font_family: Some("Avenir Next".into()),
                    fill: "#112233".into(),
                    text_color: "#fefefe".into(),
                },
            ],
        };
        let scene = diagram_to_paint_temporal(&layout, &opts);

        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Ellipse(ellipse) if ellipse.rx == 7.0
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Ellipse(ellipse) if ellipse.rx == 12.0
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Path(path) if path.commands.iter().any(|command| matches!(command, PathCommand::QuadTo { .. }))
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Path(path) if path.stroke_dash.as_deref() == Some(&[4.0, 2.0])
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::GlyphRun(run) if run.font_size == 22.0
        )));
    }

    #[test]
    fn today_marker_style_lowers_to_backend_neutral_path() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = diagram_ir::LayoutedTemporalDiagram {
            width: 320.0,
            height: 180.0,
            accessibility_title: None,
            accessibility_description: None,
            interactions: Vec::new(),
            items: vec![diagram_ir::LayoutedTemporalItem::TodayMarker {
                x: 160.0,
                y1: 20.0,
                y2: 160.0,
                stroke: "#00aa44".into(),
                stroke_width: 5.0,
                stroke_dash: None,
                opacity: 0.5,
            }],
        };
        let scene = diagram_to_paint_temporal(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Group(group)
                if group.opacity == Some(0.5)
                    && matches!(group.children.as_slice(), [PaintInstruction::Path(path)]
                        if path.stroke.as_deref() == Some("#00aa44")
                            && path.stroke_width == Some(5.0)
                            && path.stroke_dash.is_none())
        )));
    }

    #[test]
    fn two_nodes_produce_two_rects() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&simple_layout(), &opts);
        let rects = scene
            .instructions
            .iter()
            .filter(|i| matches!(i, PaintInstruction::Rect(_)))
            .count();
        assert_eq!(
            rects, 2,
            "two RoundedRect nodes → two PaintRect instructions"
        );
    }

    #[test]
    fn node_labels_emit_glyph_runs() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&simple_layout(), &opts);
        let runs = scene
            .instructions
            .iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_)))
            .count();
        // "Start" (5 chars) and "End" (3 chars) each produce one PaintGlyphRun.
        assert!(
            runs >= 2,
            "expected at least 2 PaintGlyphRuns for node labels, got {}",
            runs
        );
    }

    #[test]
    fn directed_edge_produces_arrowhead_path() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&simple_layout(), &opts);
        let paths = scene
            .instructions
            .iter()
            .filter(|i| matches!(i, PaintInstruction::Path(_)))
            .count();
        // 1 edge polyline + 1 arrowhead
        assert_eq!(paths, 2);
    }

    #[test]
    fn undirected_edge_has_no_arrowhead() {
        let mut layout = simple_layout();
        layout.edges[0].kind = EdgeKind::Undirected;
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&layout, &opts);
        let paths = scene
            .instructions
            .iter()
            .filter(|i| matches!(i, PaintInstruction::Path(_)))
            .count();
        assert_eq!(paths, 1, "undirected edge: only the polyline, no arrowhead");
    }

    #[test]
    fn ellipse_node_produces_ellipse_instruction() {
        let mut layout = simple_layout();
        layout.nodes[0].shape = DiagramShape::Ellipse;
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&layout, &opts);
        let ellipses = scene
            .instructions
            .iter()
            .filter(|i| matches!(i, PaintInstruction::Ellipse(_)))
            .count();
        assert_eq!(ellipses, 1);
    }

    #[test]
    fn diamond_node_produces_5_command_path() {
        let mut layout = simple_layout();
        layout.nodes[0].shape = DiagramShape::Diamond;
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&layout, &opts);
        let diamond_paths: Vec<_> = scene
            .instructions
            .iter()
            .filter_map(|i| {
                if let PaintInstruction::Path(p) = i {
                    Some(p)
                } else {
                    None
                }
            })
            .filter(|p| p.commands.len() == 5)
            .collect();
        assert!(
            !diamond_paths.is_empty(),
            "expected a diamond PaintPath with 5 commands"
        );
    }

    #[test]
    fn block_polygonal_and_stadium_shapes_lower_to_backend_neutral_paint() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);

        for (shape, command_count) in [
            (DiagramShape::Hexagon, 7),
            (DiagramShape::ParallelogramRight, 5),
            (DiagramShape::ParallelogramLeft, 5),
            (DiagramShape::Trapezoid, 5),
            (DiagramShape::InvertedTrapezoid, 5),
        ] {
            let mut layout = simple_layout();
            layout.nodes[0].shape = shape;
            let scene = diagram_to_paint(&layout, &opts);
            assert!(scene.instructions.iter().any(|instruction| {
                matches!(instruction, PaintInstruction::Path(path) if path.commands.len() == command_count)
            }));
        }

        let mut layout = simple_layout();
        layout.nodes[0].shape = DiagramShape::Stadium;
        let scene = diagram_to_paint(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| {
            matches!(instruction, PaintInstruction::Path(path)
                if path.commands.len() == 8
                    && path.commands.iter().filter(|command| matches!(command, PathCommand::ArcTo { .. })).count() == 4)
        }));
    }

    #[test]
    fn block_multi_outline_shapes_lower_to_backend_neutral_groups() {
        let mut layout = simple_layout();
        for (shape, expected_children) in [
            (DiagramShape::Subroutine, 3),
            (DiagramShape::Cylinder, 2),
            (DiagramShape::DoubleCircle, 2),
        ] {
            layout.nodes[0].shape = shape;
            assert!(matches!(
                node_shape_instruction(&layout.nodes[0]),
                PaintInstruction::Group(group) if group.children.len() == expected_children
            ));
        }
    }

    #[test]
    fn note_node_and_association_use_backend_neutral_paths() {
        let mut layout = simple_layout();
        layout.nodes[0].shape = DiagramShape::Note;
        layout.edges[0].kind = EdgeKind::NoteAssociation;
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&layout, &opts);

        assert!(scene.instructions.iter().any(|instruction| {
            matches!(instruction, PaintInstruction::Path(path) if path.commands.len() == 6)
        }));
        assert!(scene.instructions.iter().any(|instruction| {
            matches!(instruction, PaintInstruction::Path(path) if path.stroke_dash.is_some())
        }));
    }

    #[test]
    fn graph_accessibility_metadata_reaches_paint_scene() {
        let mut layout = simple_layout();
        layout.accessibility_title = Some("State lifecycle".into());
        layout.accessibility_description = Some("Ready transitions to running".into());
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&layout, &opts);
        let metadata = scene.metadata.unwrap();

        assert_eq!(metadata["accessibility.title"], "State lifecycle");
        assert_eq!(
            metadata["accessibility.description"],
            "Ready transitions to running"
        );
    }

    #[test]
    fn graph_node_links_reach_paint_scene_hit_test_metadata() {
        let mut layout = simple_layout();
        layout.links.push(diagram_ir::GraphLink {
            node_id: "A".into(),
            url: "https://example.com/ready".into(),
            tooltip: Some("Open ready state".into()),
        });
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&layout, &opts);
        let metadata = scene.metadata.unwrap();

        assert_eq!(
            metadata["graph.node.A.link.url"],
            "https://example.com/ready"
        );
        assert_eq!(metadata["graph.node.A.link.tooltip"], "Open ready state");
        assert!(metadata.contains_key("graph.node.A.link.bounds"));
    }

    #[test]
    fn graph_groups_lower_to_background_rectangles() {
        let mut layout = simple_layout();
        layout.groups.push(diagram_ir::LayoutedGraphGroup {
            id: "Processing".into(),
            label: DiagramLabel::new("Processing"),
            parent_id: None,
            x: 8.0,
            y: 8.0,
            width: 340.0,
            height: 100.0,
            divider_y: vec![58.0],
            direction: None,
            style: ResolvedDiagramStyle {
                fill: "#fef3c7".into(),
                stroke: "#b45309".into(),
                stroke_width: 3.0,
                text_color: "#78350f".into(),
                font_size: 14.0,
                font_weight: 400,
                font_italic: false,
                font_family: "Helvetica".into(),
                corner_radius: 8.0,
            },
        });
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&layout, &opts);

        assert!(scene.instructions.iter().any(|instruction| {
            matches!(instruction, PaintInstruction::Rect(rect)
                if rect.width == 340.0
                    && rect.fill.as_deref() == Some("#fef3c7")
                    && rect.stroke.as_deref() == Some("#b45309")
                    && rect.stroke_width == Some(3.0))
        }));
        assert!(scene.instructions.iter().any(|instruction| {
            matches!(instruction, PaintInstruction::Path(path)
                if path.stroke.as_deref() == Some("#b45309")
                    && path.stroke_width == Some(3.0))
        }));
    }

    #[test]
    fn hide_empty_descriptions_omits_unlabeled_state_geometry() {
        let mut layout = simple_layout();
        layout.hide_empty_descriptions = true;
        layout.nodes[0].label = DiagramLabel::new("");
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&layout, &opts);

        let rectangles = scene
            .instructions
            .iter()
            .filter(|instruction| matches!(instruction, PaintInstruction::Rect(_)))
            .count();
        assert_eq!(rectangles, 1);
        assert!(scene
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, PaintInstruction::Path(_))));
    }

    #[test]
    fn title_produces_extra_glyph_run() {
        let mut layout = simple_layout();
        layout.title = Some("My Diagram".to_string());
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene_with = diagram_to_paint(&layout, &opts);

        let layout_no = simple_layout();
        let opts2 = make_opts(&shaper, &metrics, &resolver);
        let scene_without = diagram_to_paint(&layout_no, &opts2);

        let runs_with = scene_with
            .instructions
            .iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_)))
            .count();
        let runs_without = scene_without
            .instructions
            .iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_)))
            .count();
        assert!(
            runs_with > runs_without,
            "title should add at least one glyph run"
        );
    }

    #[test]
    fn glyph_run_font_ref_is_shaper_provided() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&simple_layout(), &opts);
        let run = scene
            .instructions
            .iter()
            .find(|i| matches!(i, PaintInstruction::GlyphRun(_)));
        if let Some(PaintInstruction::GlyphRun(gr)) = run {
            // The FakeShaper always returns "fake:test" as font_ref.
            assert_eq!(
                gr.font_ref, "fake:test",
                "font_ref should come from the shaper, not a hardcoded string"
            );
        }
    }

    #[test]
    fn edge_label_produces_glyph_run() {
        let mut layout = simple_layout();
        layout.edges[0].label = Some(DiagramLabel::new("transfers"));
        layout.edges[0].label_position = Some(Point { x: 168.0, y: 42.0 });
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);

        let scene_with_label = diagram_to_paint(&layout, &opts);
        let opts2 = make_opts(&shaper, &metrics, &resolver);
        let scene_no_label = diagram_to_paint(&simple_layout(), &opts2);

        let runs_with = scene_with_label
            .instructions
            .iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_)))
            .count();
        let runs_without = scene_no_label
            .instructions
            .iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_)))
            .count();
        assert!(
            runs_with > runs_without,
            "edge label should produce at least one extra glyph run"
        );
    }

    #[test]
    fn css_to_color_parses_hex() {
        assert_eq!(
            css_to_color("#4b5563"),
            Color {
                r: 0x4b,
                g: 0x55,
                b: 0x63,
                a: 255
            }
        );
        assert_eq!(
            css_to_color("#ffffff"),
            Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255
            }
        );
        // Invalid/unsupported → opaque black
        assert_eq!(
            css_to_color("none"),
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255
            }
        );
    }

    #[test]
    fn chart_accessibility_metadata_reaches_paint_scene() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedChartDiagram {
            width: 400.0,
            height: 300.0,
            background_color: Some("#123456".into()),
            accessibility_title: Some("Portfolio matrix".into()),
            accessibility_description: Some("Native renderer priorities".into()),
            title_box: None,
            items: vec![],
        };

        let scene = diagram_to_paint_chart(&layout, &opts);
        assert_eq!(scene.background, "#123456");
        let metadata = scene.metadata.expect("chart accessibility metadata");
        assert_eq!(metadata["accessibility.title"], "Portfolio matrix");
        assert_eq!(
            metadata["accessibility.description"],
            "Native renderer priorities"
        );
    }

    #[test]
    fn treemap_lowers_to_backend_neutral_rectangles_and_metadata() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedTreemapDiagram {
            width: 320.0,
            height: 240.0,
            title: Some("Allocation".into()),
            accessibility_title: Some("Allocation treemap".into()),
            accessibility_description: None,
            nodes: vec![diagram_ir::LayoutedTreemapNode {
                id: "root".into(),
                label: "Root".into(),
                value: 10.0,
                depth: 0,
                x: 8.0,
                y: 48.0,
                width: 304.0,
                height: 184.0,
                class_selector: None,
            }],
        };

        let scene = diagram_to_paint_treemap(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::Rect(_))));
        assert_eq!(
            scene.metadata.as_ref().and_then(|metadata| metadata.get("accessibility.title")),
            Some(&"Allocation treemap".to_string())
        );
    }

    #[test]
    fn treeview_lowers_to_backend_neutral_paths_rects_and_glyphs() {
        let shaper = FakeShaper; let metrics = FakeMetrics; let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedTreeViewDiagram {
            width: 420.0, height: 100.0, title: None, accessibility_title: None, accessibility_description: None,
            nodes: vec![
                diagram_ir::LayoutedTreeViewNode { id: "root".into(), parent_id: None, depth: 0, label: "src".into(),
                    kind: TreeViewNodeKind::Directory, class_selector: Some("highlight".into()), icon: Some("folder".into()),
                    description: None, x: 26.0, y: 12.0, width: 376.0, height: 28.0 },
                diagram_ir::LayoutedTreeViewNode { id: "child".into(), parent_id: Some("root".into()), depth: 1, label: "main.rs".into(),
                    kind: TreeViewNodeKind::File, class_selector: None, icon: None, description: Some("entry".into()),
                    x: 68.0, y: 46.0, width: 334.0, height: 28.0 },
            ],
        };
        let scene = diagram_to_paint_treeview(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::Path(_))));
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::Rect(_))));
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::GlyphRun(_))));
    }

    #[test]
    fn venn_lowers_to_backend_neutral_ellipses_and_glyphs() {
        let shaper = FakeShaper; let metrics = FakeMetrics; let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedVennDiagram {
            width: 400.0, height: 300.0, title: Some("Overlap".into()),
            circles: vec![diagram_ir::LayoutedVennCircle {
                id: "A".into(), label: "Alpha".into(), cx: 200.0, cy: 160.0, radius: 90.0,
                style: diagram_ir::VennStyle { fill: Some("#ff0000".into()), fill_opacity: Some(0.5), ..Default::default() },
            }],
            labels: vec![diagram_ir::LayoutedVennLabel { text: "Alpha".into(), x: 200.0, y: 160.0, style: Default::default() }],
        };
        let scene = diagram_to_paint_venn(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::Ellipse(_))));
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::GlyphRun(_))));
    }

    #[test]
    fn ishikawa_lowers_to_backend_neutral_paths_rect_and_glyphs() {
        let shaper = FakeShaper; let metrics = FakeMetrics; let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedIshikawaDiagram { width: 600.0, height: 360.0, effect: "Delay".into(),
            effect_x: 430.0, effect_y: 150.0, effect_width: 150.0, effect_height: 60.0,
            spine_from: Point { x: 30.0, y: 180.0 }, spine_to: Point { x: 430.0, y: 180.0 },
            bones: vec![diagram_ir::LayoutedIshikawaBone { from: Point { x: 100.0, y: 80.0 },
                to: Point { x: 180.0, y: 180.0 }, label: "People".into(),
                label_position: Point { x: 100.0, y: 68.0 }, depth: 1 }], };
        let scene = diagram_to_paint_ishikawa(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::Path(_))));
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::Rect(_))));
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::GlyphRun(_))));
    }


    #[test]
    fn wardley_lowers_to_backend_neutral_paths_ellipses_and_glyphs() {
        let shaper = FakeShaper; let metrics = FakeMetrics; let resolver = FakeResolver; let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedWardleyDiagram { width: 500.0, height: 320.0, title: Some("Map".into()), stages: vec!["Genesis".into(), "Commodity".into()],
            nodes: vec![diagram_ir::LayoutedWardleyNode { id: "a".into(), label: "User".into(), position: Point { x: 200.0, y: 100.0 }, anchor: true }],
            links: vec![], evolves: vec![] };
        let scene = diagram_to_paint_wardley(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::Path(_))));
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::Ellipse(_))));
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::GlyphRun(_))));
    }


    #[test]
    fn cynefin_lowers_to_backend_neutral_rects_ellipse_and_glyphs() {
        let shaper = FakeShaper; let metrics = FakeMetrics; let resolver = FakeResolver; let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedCynefinDiagram { width: 400.0, height: 300.0, title: None,
            domains: vec![diagram_ir::LayoutedCynefinDomain { name: "complex".into(), items: vec!["Probe".into()], x: 10.0, y: 10.0,
                width: 180.0, height: 130.0, center: Point { x: 100.0, y: 75.0 }, confusion: false },
                diagram_ir::LayoutedCynefinDomain { name: "confusion".into(), items: vec![], x: 150.0, y: 110.0,
                    width: 100.0, height: 80.0, center: Point { x: 200.0, y: 150.0 }, confusion: true }], transitions: vec![] };
        let scene = diagram_to_paint_cynefin(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::Rect(_))));
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::Ellipse(_))));
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::GlyphRun(_))));
    }

    #[test]
    fn chart_point_and_bar_labels_lower_to_backend_neutral_glyphs() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedChartDiagram {
            width: 400.0,
            height: 300.0,
            background_color: None,
            accessibility_title: None,
            accessibility_description: None,
            title_box: None,
            items: vec![
                LayoutedChartItem::PointLabel {
                    x: 100.0,
                    y: 80.0,
                    width: 60.0,
                    height: 14.4,
                    text: "Peak".into(),
                    font_size: 12.0,
                    color: "#ef4444".into(),
                },
                LayoutedChartItem::BarLabel {
                    x: 160.0,
                    y: 120.0,
                    width: 40.0,
                    height: 14.4,
                    text: "42".into(),
                    font_size: 12.0,
                    color: "#123456".into(),
                },
            ],
        };

        let scene = diagram_to_paint_chart(&layout, &opts);
        assert_eq!(
            scene
                .instructions
                .iter()
                .filter(|instruction| matches!(instruction, PaintInstruction::GlyphRun(_)))
                .count(),
            2
        );
    }

    #[test]
    fn chart_axis_styles_lower_to_backend_neutral_paint() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedChartDiagram {
            width: 400.0,
            height: 300.0,
            background_color: None,
            accessibility_title: None,
            accessibility_description: None,
            title_box: None,
            items: vec![
                LayoutedChartItem::AxisSpine {
                    x1: 20.0,
                    y1: 260.0,
                    x2: 380.0,
                    y2: 260.0,
                    orientation: Orientation::Horizontal,
                    stroke_width: 5.0,
                    color: "#123456".into(),
                },
                LayoutedChartItem::AxisTick {
                    x: 100.0,
                    y: 264.0,
                    label: "Q1".into(),
                    orientation: Orientation::Vertical,
                    font_size: 18.0,
                    rotation_degrees: 0.0,
                    color: "#234567".into(),
                },
                LayoutedChartItem::AxisTick {
                    x: 180.0,
                    y: 264.0,
                    label: "Rotated".into(),
                    orientation: Orientation::Vertical,
                    font_size: 16.0,
                    rotation_degrees: -45.0,
                    color: "#345678".into(),
                },
            ],
        };

        let scene = diagram_to_paint_chart(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Path(path)
                if path.stroke_width == Some(5.0) && path.stroke.as_deref() == Some("#123456")
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::GlyphRun(run)
                if run.font_size == 18.0
                    && run.fill.as_deref() == Some("rgb(35, 69, 103)")
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Group(group)
                if group.transform.is_some()
                    && group.children.iter().any(|child| matches!(
                        child,
                        PaintInstruction::GlyphRun(run)
                            if run.font_size == 16.0
                                && run.fill.as_deref() == Some("rgb(52, 86, 120)")
                    ))
        )));
    }

    #[test]
    fn quadrant_border_lowers_to_frame_and_divider_paths() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedChartDiagram {
            width: 400.0,
            height: 300.0,
            background_color: None,
            accessibility_title: None,
            accessibility_description: None,
            title_box: None,
            items: vec![LayoutedChartItem::QuadrantBorder {
                x: 20.0,
                y: 30.0,
                width: 300.0,
                height: 200.0,
                internal_color: "#123456".into(),
                external_color: "#654321".into(),
                internal_width: 3.0,
                external_width: 5.0,
            }],
        };

        let scene = diagram_to_paint_chart(&layout, &opts);
        let frame = scene
            .instructions
            .iter()
            .find_map(|instruction| match instruction {
                PaintInstruction::Rect(rect) => Some(rect),
                _ => None,
            })
            .expect("quadrant frame");
        assert_eq!(frame.stroke_width, Some(5.0));
        assert_eq!(frame.stroke.as_deref(), Some("#654321"));
        assert_eq!(
            scene
                .instructions
                .iter()
                .filter(|instruction| {
                    matches!(instruction, PaintInstruction::Path(path)
                if path.stroke_width == Some(3.0) && path.stroke.as_deref() == Some("#123456"))
                })
                .count(),
            2
        );
    }

    #[test]
    fn painter_order_edges_before_nodes() {
        // All Path (edges/arrowheads) instructions must come before all Rect
        // (node shape) instructions — painter's algorithm: edges behind nodes.
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&simple_layout(), &opts);

        let last_path_idx = scene
            .instructions
            .iter()
            .rposition(|i| matches!(i, PaintInstruction::Path(_)));
        let first_rect_idx = scene
            .instructions
            .iter()
            .position(|i| matches!(i, PaintInstruction::Rect(_)));
        if let (Some(lp), Some(fr)) = (last_path_idx, first_rect_idx) {
            assert!(lp < fr, "all edge paths should appear before node rects");
        }
    }
}
