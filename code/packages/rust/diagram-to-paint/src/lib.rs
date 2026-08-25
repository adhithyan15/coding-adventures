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

pub const VERSION: &str = "0.62.0";

use std::collections::HashMap;

use diagram_ir::{
    DiagramShape, EdgeKind, GeoElement, GitCommitSymbol, LayoutedChartDiagram, LayoutedChartItem,
    LayoutedGeometricDiagram, LayoutedGraphDiagram, LayoutedGraphEdge, LayoutedGraphNode,
    LayoutedSequenceDiagram, LayoutedSequenceItem, LayoutedStructuralDiagram,
    LayoutedTemporalDiagram, LayoutedTemporalItem, Orientation, Point, RelKind, SequenceArrowhead,
    SequenceBlockKind, SequenceCentralConnection, SequenceLineStyle, SequenceParticipantKind,
    SequenceProperty, TaskStatus, TextAlign as GeoTextAlign,
};
use layout_ir::{Color, Content, FontSpec, PositionedNode, TextAlign, TextContent};
use layout_to_paint::{layout_to_paint, LayoutToPaintOptions};
use paint_instructions::{
    PaintBase, PaintEllipse, PaintInstruction, PaintPath, PaintRect, PaintScene, PathCommand,
    StrokeCap, StrokeJoin,
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
                ..
            } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x1, y: *y1 }, Point { x: *x2, y: *y2 }],
                    "#374151",
                    *stroke_width,
                )));
            }
            LayoutedChartItem::AxisTickMark {
                x1,
                y1,
                x2,
                y2,
                stroke_width,
            } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x1, y: *y1 }, Point { x: *x2, y: *y2 }],
                    "#6b7280",
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
            } => {
                let (tx, ty, tw) = match orientation {
                    Orientation::Horizontal => (x - 30.0, y - ls / 2.0, 60.0),
                    Orientation::Vertical => (x - 30.0, y + 2.0, 60.0),
                };
                text_children.push(text_node(
                    label,
                    tx,
                    ty,
                    tw,
                    font_size * 1.2,
                    font_with_size(&lf, Some(*font_size)),
                    Color {
                        r: 107,
                        g: 114,
                        b: 128,
                        a: 255,
                    },
                ));
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
        background: format!("rgb({},{},{})", bg.r, bg.g, bg.b),
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
            LayoutedTemporalItem::TimeAxisTick { x, y, label } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x, y: *y - 4.0 }, Point { x: *x, y: *y }],
                    "#374151",
                    1.0,
                )));
                text_children.push(text_node(
                    label,
                    x - 20.0,
                    *y + 2.0,
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
                status,
                label,
            } => {
                let color = task_status_color(status);
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: Some(color.into()),
                    stroke: None,
                    stroke_width: None,
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
            LayoutedTemporalItem::MilestoneMarker { x, y, label } => {
                let s = 8.0;
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo { x: *x, y: y - s },
                        PathCommand::LineTo { x: x + s, y: *y },
                        PathCommand::LineTo { x: *x, y: y + s },
                        PathCommand::LineTo { x: x - s, y: *y },
                        PathCommand::Close,
                    ],
                    fill: Some("#111827".into()),
                    fill_rule: None,
                    stroke: None,
                    stroke_width: None,
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
            LayoutedTemporalItem::TodayMarker { x, y1, y2 } => {
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo { x: *x, y: *y1 },
                        PathCommand::LineTo { x: *x, y: *y2 },
                    ],
                    fill: Some("none".into()),
                    fill_rule: None,
                    stroke: Some("#ef4444".into()),
                    stroke_width: Some(2.0),
                    stroke_cap: None,
                    stroke_join: None,
                    stroke_dash: Some(vec![6.0, 3.0]),
                    stroke_dash_offset: None,
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

fn task_status_color(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Normal => "#3b82f6",
        TaskStatus::Done => "#22c55e",
        TaskStatus::Active => "#f59e0b",
        TaskStatus::Crit => "#ef4444",
        TaskStatus::Milestone => "#111827",
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
        assert_eq!(crate::VERSION, "0.62.0");
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
            accessibility_title: Some("Portfolio matrix".into()),
            accessibility_description: Some("Native renderer priorities".into()),
            title_box: None,
            items: vec![],
        };

        let scene = diagram_to_paint_chart(&layout, &opts);
        let metadata = scene.metadata.expect("chart accessibility metadata");
        assert_eq!(metadata["accessibility.title"], "Portfolio matrix");
        assert_eq!(
            metadata["accessibility.description"],
            "Native renderer priorities"
        );
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
                },
                LayoutedChartItem::AxisTick {
                    x: 100.0,
                    y: 264.0,
                    label: "Q1".into(),
                    orientation: Orientation::Vertical,
                    font_size: 18.0,
                },
            ],
        };

        let scene = diagram_to_paint_chart(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Path(path) if path.stroke_width == Some(5.0)
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::GlyphRun(run) if run.font_size == 18.0
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
