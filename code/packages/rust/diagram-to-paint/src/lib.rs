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

pub const VERSION: &str = "0.2.0";

use std::collections::HashMap;

use diagram_ir::{
    DiagramShape, EdgeKind, GeoElement, LayoutedChartDiagram, LayoutedChartItem,
    LayoutedGeometricDiagram, LayoutedGraphDiagram, LayoutedGraphEdge, LayoutedGraphNode,
    LayoutedStructuralDiagram, LayoutedTemporalDiagram, LayoutedTemporalItem, Orientation,
    Point, RelKind, TaskStatus, TextAlign as GeoTextAlign,
};
use layout_ir::{Color, Content, FontSpec, PositionedNode, TextAlign, TextContent};
use layout_to_paint::{LayoutToPaintOptions, layout_to_paint};
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
    pub shaper:   &'a S,
    pub metrics:  &'a M,
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
                    PathCommand::MoveTo { x: cx,                   y: node.y },
                    PathCommand::LineTo { x: node.x + node.width,  y: cy },
                    PathCommand::LineTo { x: cx,                   y: node.y + node.height },
                    PathCommand::LineTo { x: node.x,               y: cy },
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
        DiagramShape::Rect => PaintInstruction::Rect(PaintRect {
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

    let end  = &edge.points[edge.points.len() - 1];
    let prev = &edge.points[edge.points.len() - 2];

    let dx = end.x - prev.x;
    let dy = end.y - prev.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-9 {
        return None;
    }

    let ux = dx / len;
    let uy = dy / len;
    let size   = 10.0;
    let half_w = size * 0.6;

    let base_x = end.x - ux * size;
    let base_y = end.y - uy * size;
    let px = -uy;
    let py =  ux;

    Some(PaintPath {
        base: PaintBase::default(),
        commands: vec![
            PathCommand::MoveTo { x: end.x, y: end.y },
            PathCommand::LineTo { x: base_x + px * half_w, y: base_y + py * half_w },
            PathCommand::LineTo { x: base_x - px * half_w, y: base_y - py * half_w },
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
    Color { r: 0, g: 0, b: 0, a: 255 } // opaque black fallback
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
            text_align: TextAlign::Center,
        })),
        children: Vec::new(),
        ext: HashMap::new(),
    }
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

    // ── 1. Edges (lines + arrowheads) — drawn behind nodes ───────────────────
    for edge in &diagram.edges {
        instructions.push(PaintInstruction::Path(line_path(
            &edge.points,
            &edge.style.stroke,
            edge.style.stroke_width,
        )));
        if let Some(tip) = arrowhead(edge) {
            instructions.push(PaintInstruction::Path(tip));
        }
    }

    // ── 2. Node shapes — drawn over edges so endpoints are hidden ─────────────
    for node in &diagram.nodes {
        instructions.push(node_shape_instruction(node));
    }

    // ── 3. Text — all labels routed through layout-to-paint ───────────────────
    //
    // Build one PositionedNode per text item, collect them as children of a
    // transparent synthetic root spanning the full canvas, then call
    // layout_to_paint once. Append the resulting PaintGlyphRun instructions.
    let label_font   = options.label_font.clone();
    let title_font   = options.title_font.clone();
    let label_size   = label_font.size;
    let title_size   = title_font.size;

    let mut text_children: Vec<PositionedNode> = Vec::new();

    // Title (if present) — centred at the top of the canvas.
    if let Some(title) = &diagram.title {
        text_children.push(text_node(
            title,
            0.0,
            8.0,
            diagram.width,
            title_size * 1.2,
            title_font,
            Color { r: 17, g: 24, b: 39, a: 255 }, // #111827
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
                    f
                },
                css_to_color(&edge.style.text_color),
            ));
        }
    }

    // Node labels — vertically centred inside each node bounding box.
    for node in &diagram.nodes {
        text_children.push(text_node(
            &node.label.text,
            node.x,
            node.y + (node.height - label_size) / 2.0,
            node.width,
            label_size * 1.2,
            {
                let mut f = label_font.clone();
                f.size = node.style.font_size;
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
        background: Color { r: 0, g: 0, b: 0, a: 0 }, // transparent root
        device_pixel_ratio: 1.0,
        shaper:   options.shaper,
        metrics:  options.metrics,
        resolver: options.resolver,
    };
    let text_scene = layout_to_paint(&text_root, &text_opts);
    instructions.extend(text_scene.instructions);

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
        metadata: None,
    }
}

// ============================================================================
// Tests
// ============================================================================

// ============================================================================
// Chart family (DG04)
// ============================================================================

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
            &tb.text, tb.x - diagram.width / 2.0, tb.y - ls, diagram.width, ls * 1.4,
            options.title_font.clone(), Color { r: 17, g: 24, b: 39, a: 255 },
        ));
    }

    for item in &diagram.items {
        match item {
            LayoutedChartItem::AxisSpine { x1, y1, x2, y2, .. } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x1, y: *y1 }, Point { x: *x2, y: *y2 }],
                    "#374151", 1.5,
                )));
            }
            LayoutedChartItem::GridLine { x1, y1, x2, y2 } => {
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo { x: *x1, y: *y1 },
                        PathCommand::LineTo { x: *x2, y: *y2 },
                    ],
                    fill: Some("none".into()), fill_rule: None,
                    stroke: Some("#e5e7eb".into()), stroke_width: Some(1.0),
                    stroke_cap: None, stroke_join: None,
                    stroke_dash: Some(vec![4.0, 4.0]), stroke_dash_offset: None,
                }));
            }
            LayoutedChartItem::Bar { x, y, width, height, color } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x, y: *y, width: *width, height: *height,
                    fill: Some(color.clone()), stroke: None, stroke_width: None,
                    corner_radius: Some(2.0), stroke_dash: None, stroke_dash_offset: None,
                }));
            }
            LayoutedChartItem::LinePath { points, color } => {
                if points.len() >= 2 {
                    instructions.push(PaintInstruction::Path(line_path(points, color, 2.0)));
                }
            }
            LayoutedChartItem::PieArc { cx, cy, r, start_angle, end_angle, color, label } => {
                let cmds = pie_slice_commands(*cx, *cy, *r, *start_angle, *end_angle);
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: cmds,
                    fill: Some(color.clone()), fill_rule: None,
                    stroke: Some("#ffffff".into()), stroke_width: Some(1.5),
                    stroke_cap: None, stroke_join: None,
                    stroke_dash: None, stroke_dash_offset: None,
                }));
                // Label at midpoint of arc
                let mid = (start_angle + end_angle) / 2.0;
                let lx = cx + (r * 0.65) * mid.cos();
                let ly = cy + (r * 0.65) * mid.sin();
                text_children.push(text_node(
                    label, lx - 40.0, ly - ls / 2.0, 80.0, ls * 1.2,
                    lf.clone(), Color { r: 255, g: 255, b: 255, a: 255 },
                ));
            }
            LayoutedChartItem::SankeyBand { from_x, from_y, to_x, width, color, .. } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *from_x, y: *from_y,
                    width: to_x - from_x, height: *width,
                    fill: Some(color.clone()), stroke: None, stroke_width: None,
                    corner_radius: None, stroke_dash: None, stroke_dash_offset: None,
                }));
            }
            LayoutedChartItem::DataLabel { x, y, text } => {
                text_children.push(text_node(
                    text, x - 40.0, y - ls / 2.0, 80.0, ls * 1.2,
                    lf.clone(), Color { r: 55, g: 65, b: 81, a: 255 },
                ));
            }
            LayoutedChartItem::AxisTick { x, y, label, orientation } => {
                let (tx, ty, tw) = match orientation {
                    Orientation::Horizontal => (x - 30.0, y - ls / 2.0, 60.0),
                    Orientation::Vertical   => (x - 30.0, y + 2.0,       60.0),
                };
                text_children.push(text_node(
                    label, tx, ty, tw, ls * 1.2,
                    lf.clone(), Color { r: 107, g: 114, b: 128, a: 255 },
                ));
            }
            LayoutedChartItem::Legend { x, y, entries } => {
                let mut ex = *x;
                for e in entries {
                    instructions.push(PaintInstruction::Rect(PaintRect {
                        base: PaintBase::default(),
                        x: ex, y: y - ls / 2.0, width: ls, height: ls,
                        fill: Some(e.color.clone()), stroke: None, stroke_width: None,
                        corner_radius: None, stroke_dash: None, stroke_dash_offset: None,
                    }));
                    text_children.push(text_node(
                        &e.label, ex + ls + 4.0, y - ls / 2.0, 80.0, ls * 1.2,
                        lf.clone(), Color { r: 55, g: 65, b: 81, a: 255 },
                    ));
                    ex += ls + 4.0 + 88.0;
                }
            }
        }
    }

    let text_root = PositionedNode {
        x: 0.0, y: 0.0, width: diagram.width, height: diagram.height,
        id: None, content: None, children: text_children, ext: HashMap::new(),
    };
    let text_opts = LayoutToPaintOptions {
        width: diagram.width, height: diagram.height,
        background: Color { r: 0, g: 0, b: 0, a: 0 },
        device_pixel_ratio: 1.0,
        shaper: options.shaper, metrics: options.metrics, resolver: options.resolver,
    };
    let text_scene = layout_to_paint(&text_root, &text_opts);
    instructions.extend(text_scene.instructions);

    let bg = options.background;
    PaintScene {
        width: diagram.width, height: diagram.height,
        background: format!("rgb({},{},{})", bg.r, bg.g, bg.b),
        instructions, id: None, metadata: None,
    }
}

/// Build `PathCommand`s for a filled pie slice (center → arc → close).
fn pie_slice_commands(
    cx: f64, cy: f64, r: f64, start: f64, end: f64,
) -> Vec<PathCommand> {
    let mut cmds = vec![
        PathCommand::MoveTo { x: cx, y: cy },
        PathCommand::LineTo { x: cx + r * start.cos(), y: cy + r * start.sin() },
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
            x:    cx + r * c1c,
            y:    cy + r * c1s,
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
    let lf = options.label_font.clone();
    let ls = lf.size;
    const HEADER_H: f64 = 40.0;

    // ── Relationships (drawn behind nodes) ───────────────────────────────────
    for rel in &diagram.relationships {
        instructions.push(PaintInstruction::Path(line_path(&rel.points, "#6b7280", 1.5)));
        // Arrowhead on the last segment
        if rel.points.len() >= 2 {
            let tip  = &rel.points[rel.points.len() - 1];
            let prev = &rel.points[rel.points.len() - 2];
            instructions.push(PaintInstruction::Path(
                structural_arrowhead(prev, tip, &rel.kind),
            ));
        }
        if let Some((ref pos, ref lbl)) = rel.label {
            text_children.push(text_node(
                lbl, pos.x - 40.0, pos.y - ls / 2.0, 80.0, ls * 1.2,
                lf.clone(), Color { r: 55, g: 65, b: 81, a: 255 },
            ));
        }
    }

    // ── Node boxes ───────────────────────────────────────────────────────────
    for node in &diagram.nodes {
        // Outer rect
        instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: node.x, y: node.y, width: node.width, height: node.height,
            fill: Some("#f9fafb".into()), stroke: Some("#374151".into()),
            stroke_width: Some(1.5), corner_radius: Some(4.0),
            stroke_dash: None, stroke_dash_offset: None,
        }));
        // Header divider
        instructions.push(PaintInstruction::Path(line_path(
            &[
                Point { x: node.x, y: node.y + HEADER_H },
                Point { x: node.x + node.width, y: node.y + HEADER_H },
            ],
            "#d1d5db", 1.0,
        )));
        // Header text (with optional stereotype)
        let header_label = if let Some(ref st) = node.stereotype {
            format!("«{}»\n{}", st, node.header)
        } else {
            node.header.clone()
        };
        text_children.push(text_node(
            &header_label,
            node.x, node.y + 8.0, node.width, HEADER_H - 8.0,
            options.title_font.clone(), Color { r: 17, g: 24, b: 39, a: 255 },
        ));
        // Compartments
        for comp in &node.compartments {
            let comp_y = node.y + comp.y_offset;
            // Compartment divider
            instructions.push(PaintInstruction::Path(line_path(
                &[
                    Point { x: node.x, y: comp_y },
                    Point { x: node.x + node.width, y: comp_y },
                ],
                "#e5e7eb", 1.0,
            )));
            // Row text
            for (i, row) in comp.rows.iter().enumerate() {
                text_children.push(text_node(
                    row,
                    node.x + 8.0,
                    comp_y + 8.0 + i as f64 * (ls + 4.0),
                    node.width - 16.0,
                    ls * 1.2,
                    lf.clone(),
                    Color { r: 55, g: 65, b: 81, a: 255 },
                ));
            }
        }
    }

    let text_root = PositionedNode {
        x: 0.0, y: 0.0, width: diagram.width, height: diagram.height,
        id: None, content: None, children: text_children, ext: HashMap::new(),
    };
    let text_opts = LayoutToPaintOptions {
        width: diagram.width, height: diagram.height,
        background: Color { r: 0, g: 0, b: 0, a: 0 },
        device_pixel_ratio: 1.0,
        shaper: options.shaper, metrics: options.metrics, resolver: options.resolver,
    };
    let text_scene = layout_to_paint(&text_root, &text_opts);
    instructions.extend(text_scene.instructions);

    let bg = options.background;
    PaintScene {
        width: diagram.width, height: diagram.height,
        background: format!("rgb({},{},{})", bg.r, bg.g, bg.b),
        instructions, id: None, metadata: None,
    }
}

fn structural_arrowhead(prev: &Point, tip: &Point, kind: &RelKind) -> PaintPath {
    let dx = tip.x - prev.x;
    let dy = tip.y - prev.y;
    let len = (dx * dx + dy * dy).sqrt().max(1e-9);
    let ux = dx / len;
    let uy = dy / len;
    let size  = 10.0;
    let hw    = size * 0.5;
    let bx    = tip.x - ux * size;
    let by    = tip.y - uy * size;
    let px = -uy;
    let py =  ux;
    let (fill, open) = match kind {
        RelKind::Inheritance | RelKind::Realization => ("#ffffff", true),
        RelKind::Composition => ("#374151", false),
        _ => ("#6b7280", false),
    };
    let commands = if open {
        vec![
            PathCommand::MoveTo { x: tip.x, y: tip.y },
            PathCommand::LineTo { x: bx + px * hw, y: by + py * hw },
            PathCommand::MoveTo { x: tip.x, y: tip.y },
            PathCommand::LineTo { x: bx - px * hw, y: by - py * hw },
        ]
    } else {
        vec![
            PathCommand::MoveTo { x: tip.x, y: tip.y },
            PathCommand::LineTo { x: bx + px * hw, y: by + py * hw },
            PathCommand::LineTo { x: bx - px * hw, y: by - py * hw },
            PathCommand::Close,
        ]
    };
    PaintPath {
        base: PaintBase::default(),
        commands,
        fill: if open { Some("none".into()) } else { Some(fill.into()) },
        fill_rule: None,
        stroke: Some("#374151".into()), stroke_width: Some(1.5),
        stroke_cap: None, stroke_join: None,
        stroke_dash: None, stroke_dash_offset: None,
    }
}

// ============================================================================
// Temporal family (DG04)
// ============================================================================

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
            LayoutedTemporalItem::TimeAxisSpine { x1, y1, x2, y2 } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x1, y: *y1 }, Point { x: *x2, y: *y2 }],
                    "#374151", 1.5,
                )));
            }
            LayoutedTemporalItem::TimeAxisTick { x, y, label } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x, y: *y - 4.0 }, Point { x: *x, y: *y }],
                    "#374151", 1.0,
                )));
                text_children.push(text_node(
                    label, x - 20.0, *y + 2.0, 40.0, ls * 1.2,
                    lf.clone(), Color { r: 107, g: 114, b: 128, a: 255 },
                ));
            }
            LayoutedTemporalItem::SectionHeader { x, y, width, height, label } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x, y: *y, width: *width, height: *height,
                    fill: Some("#f3f4f6".into()), stroke: None, stroke_width: None,
                    corner_radius: None, stroke_dash: None, stroke_dash_offset: None,
                }));
                text_children.push(text_node(
                    label, *x + 8.0, *y + (*height - ls) / 2.0, *width - 16.0, ls * 1.2,
                    options.title_font.clone(), Color { r: 17, g: 24, b: 39, a: 255 },
                ));
            }
            LayoutedTemporalItem::TaskBar { x, y, width, height, status, label } => {
                let color = task_status_color(status);
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x, y: *y, width: *width, height: *height,
                    fill: Some(color.into()), stroke: None, stroke_width: None,
                    corner_radius: Some(2.0), stroke_dash: None, stroke_dash_offset: None,
                }));
                text_children.push(text_node(
                    label, *x + 4.0, *y + (*height - ls) / 2.0,
                    (*width - 8.0).max(8.0), ls * 1.2,
                    lf.clone(), Color { r: 255, g: 255, b: 255, a: 255 },
                ));
            }
            LayoutedTemporalItem::MilestoneMarker { x, y, label } => {
                let s = 8.0;
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo { x: *x,     y: y - s },
                        PathCommand::LineTo { x: x + s,  y: *y },
                        PathCommand::LineTo { x: *x,     y: y + s },
                        PathCommand::LineTo { x: x - s,  y: *y },
                        PathCommand::Close,
                    ],
                    fill: Some("#111827".into()), fill_rule: None,
                    stroke: None, stroke_width: None,
                    stroke_cap: None, stroke_join: None,
                    stroke_dash: None, stroke_dash_offset: None,
                }));
                text_children.push(text_node(
                    label, x - 40.0, y + s + 2.0, 80.0, ls * 1.2,
                    lf.clone(), Color { r: 17, g: 24, b: 39, a: 255 },
                ));
            }
            LayoutedTemporalItem::TodayMarker { x, y1, y2 } => {
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo { x: *x, y: *y1 },
                        PathCommand::LineTo { x: *x, y: *y2 },
                    ],
                    fill: Some("none".into()), fill_rule: None,
                    stroke: Some("#ef4444".into()), stroke_width: Some(2.0),
                    stroke_cap: None, stroke_join: None,
                    stroke_dash: Some(vec![6.0, 3.0]), stroke_dash_offset: None,
                }));
            }
            LayoutedTemporalItem::BranchLane { y, color, label } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: 0.0, y: *y }, Point { x: diagram.width, y: *y }],
                    color, 1.0,
                )));
                text_children.push(text_node(
                    label, 4.0, y - ls / 2.0, 56.0, ls * 1.2,
                    lf.clone(), Color { r: 55, g: 65, b: 81, a: 255 },
                ));
            }
            LayoutedTemporalItem::CommitNode { x, y, id: _, message, tag } => {
                instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                    base: PaintBase::default(),
                    cx: *x, cy: *y, rx: 8.0, ry: 8.0,
                    fill: Some("#3b82f6".into()),
                    stroke: Some("#1d4ed8".into()), stroke_width: Some(2.0),
                    stroke_dash: None, stroke_dash_offset: None,
                }));
                if let Some(ref msg) = message {
                    text_children.push(text_node(
                        msg, x - 40.0, y - ls - 10.0, 80.0, ls * 1.2,
                        lf.clone(), Color { r: 55, g: 65, b: 81, a: 255 },
                    ));
                }
                if let Some(ref t) = tag {
                    text_children.push(text_node(
                        t, x - 24.0, y + 12.0, 48.0, ls * 1.2,
                        lf.clone(), Color { r: 34, g: 197, b: 94, a: 255 },
                    ));
                }
            }
            LayoutedTemporalItem::MergeArc { from_x, from_y, to_x, to_y } => {
                let cpx = (from_x + to_x) / 2.0;
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: vec![
                        PathCommand::MoveTo { x: *from_x, y: *from_y },
                        PathCommand::CubicTo {
                            cx1: cpx, cy1: *from_y,
                            cx2: cpx, cy2: *to_y,
                            x: *to_x, y: *to_y,
                        },
                    ],
                    fill: Some("none".into()), fill_rule: None,
                    stroke: Some("#6b7280".into()), stroke_width: Some(2.0),
                    stroke_cap: Some(StrokeCap::Round), stroke_join: Some(StrokeJoin::Round),
                    stroke_dash: None, stroke_dash_offset: None,
                }));
            }
        }
    }

    let text_root = PositionedNode {
        x: 0.0, y: 0.0, width: diagram.width, height: diagram.height,
        id: None, content: None, children: text_children, ext: HashMap::new(),
    };
    let text_opts = LayoutToPaintOptions {
        width: diagram.width, height: diagram.height,
        background: Color { r: 0, g: 0, b: 0, a: 0 },
        device_pixel_ratio: 1.0,
        shaper: options.shaper, metrics: options.metrics, resolver: options.resolver,
    };
    let text_scene = layout_to_paint(&text_root, &text_opts);
    instructions.extend(text_scene.instructions);

    let bg = options.background;
    PaintScene {
        width: diagram.width, height: diagram.height,
        background: format!("rgb({},{},{})", bg.r, bg.g, bg.b),
        instructions, id: None, metadata: None,
    }
}

fn task_status_color(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Normal    => "#3b82f6",
        TaskStatus::Done      => "#22c55e",
        TaskStatus::Active    => "#f59e0b",
        TaskStatus::Crit      => "#ef4444",
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
            GeoElement::Box { x, y, w, h, corner_radius, label, fill, stroke, .. } => {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: *x, y: *y, width: *w, height: *h,
                    fill: Some(fill.clone().unwrap_or_else(|| "#f9fafb".into())),
                    stroke: Some(stroke.clone().unwrap_or_else(|| "#374151".into())),
                    stroke_width: Some(1.5),
                    corner_radius: Some(*corner_radius),
                    stroke_dash: None, stroke_dash_offset: None,
                }));
                if let Some(ref lbl) = label {
                    text_children.push(text_node(
                        lbl, *x + 4.0, y + (h - ls) / 2.0, w - 8.0, ls * 1.2,
                        lf.clone(), Color { r: 17, g: 24, b: 39, a: 255 },
                    ));
                }
            }
            GeoElement::Circle { cx, cy, r, label, fill, stroke, .. } => {
                instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                    base: PaintBase::default(),
                    cx: *cx, cy: *cy, rx: *r, ry: *r,
                    fill: Some(fill.clone().unwrap_or_else(|| "#f9fafb".into())),
                    stroke: Some(stroke.clone().unwrap_or_else(|| "#374151".into())),
                    stroke_width: Some(1.5),
                    stroke_dash: None, stroke_dash_offset: None,
                }));
                if let Some(ref lbl) = label {
                    text_children.push(text_node(
                        lbl, cx - r * 0.7, cy - ls / 2.0, r * 1.4, ls * 1.2,
                        lf.clone(), Color { r: 17, g: 24, b: 39, a: 255 },
                    ));
                }
            }
            GeoElement::Line { x1, y1, x2, y2, arrow_end, arrow_start, stroke, .. } => {
                let stroke_color = stroke.as_deref().unwrap_or("#374151");
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x1, y: *y1 }, Point { x: *x2, y: *y2 }],
                    stroke_color, 1.5,
                )));
                if *arrow_end {
                    let prev = Point { x: *x1, y: *y1 };
                    let tip  = Point { x: *x2, y: *y2 };
                    instructions.push(PaintInstruction::Path(simple_arrowhead(&prev, &tip, stroke_color)));
                }
                if *arrow_start {
                    let prev = Point { x: *x2, y: *y2 };
                    let tip  = Point { x: *x1, y: *y1 };
                    instructions.push(PaintInstruction::Path(simple_arrowhead(&prev, &tip, stroke_color)));
                }
            }
            GeoElement::Arc { cx, cy, r, start_deg, end_deg, stroke, .. } => {
                let start = start_deg.to_radians();
                let end   = end_deg.to_radians();
                let n = (((end - start).abs() / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
                let step = (end - start) / n as f64;
                let mut cmds = vec![
                    PathCommand::MoveTo {
                        x: cx + r * start.cos(), y: cy + r * start.sin(),
                    }
                ];
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
                        x:    cx + r * c1c,
                        y:    cy + r * c1s,
                    });
                }
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands: cmds,
                    fill: Some("none".into()), fill_rule: None,
                    stroke: Some(stroke.clone().unwrap_or_else(|| "#374151".into())),
                    stroke_width: Some(1.5),
                    stroke_cap: Some(StrokeCap::Round), stroke_join: Some(StrokeJoin::Round),
                    stroke_dash: None, stroke_dash_offset: None,
                }));
            }
            GeoElement::Text { x, y, text, align, .. } => {
                use layout_ir::TextAlign as LTextAlign;
                let ta = match align {
                    GeoTextAlign::Left   => LTextAlign::Start,
                    GeoTextAlign::Center => LTextAlign::Center,
                    GeoTextAlign::Right  => LTextAlign::End,
                };
                let est_w = text.len() as f64 * 7.5 + 8.0;
                text_children.push(PositionedNode {
                    x: *x, y: y - ls, width: est_w, height: ls * 1.4,
                    id: None,
                    content: Some(layout_ir::Content::Text(layout_ir::TextContent {
                        value: text.clone(),
                        font: lf.clone(),
                        color: Color { r: 17, g: 24, b: 39, a: 255 },
                        max_lines: None,
                        text_align: ta,
                    })),
                    children: Vec::new(),
                    ext: HashMap::new(),
                });
            }
        }
    }

    let text_root = PositionedNode {
        x: 0.0, y: 0.0, width: diagram.width, height: diagram.height,
        id: None, content: None, children: text_children, ext: HashMap::new(),
    };
    let text_opts = LayoutToPaintOptions {
        width: diagram.width, height: diagram.height,
        background: Color { r: 0, g: 0, b: 0, a: 0 },
        device_pixel_ratio: 1.0,
        shaper: options.shaper, metrics: options.metrics, resolver: options.resolver,
    };
    let text_scene = layout_to_paint(&text_root, &text_opts);
    instructions.extend(text_scene.instructions);

    let bg = options.background;
    PaintScene {
        width: diagram.width, height: diagram.height,
        background: format!("rgb({},{},{})", bg.r, bg.g, bg.b),
        instructions, id: None, metadata: None,
    }
}

fn simple_arrowhead(prev: &Point, tip: &Point, stroke: &str) -> PaintPath {
    let dx = tip.x - prev.x;
    let dy = tip.y - prev.y;
    let len = (dx * dx + dy * dy).sqrt().max(1e-9);
    let ux = dx / len;
    let uy = dy / len;
    let size = 10.0;
    let hw   = size * 0.5;
    let bx   = tip.x - ux * size;
    let by   = tip.y - uy * size;
    let px   = -uy;
    let py   =  ux;
    PaintPath {
        base: PaintBase::default(),
        commands: vec![
            PathCommand::MoveTo { x: tip.x, y: tip.y },
            PathCommand::LineTo { x: bx + px * hw, y: by + py * hw },
            PathCommand::LineTo { x: bx - px * hw, y: by - py * hw },
            PathCommand::Close,
        ],
        fill: Some(stroke.into()), fill_rule: None,
        stroke: None, stroke_width: None,
        stroke_cap: None, stroke_join: None,
        stroke_dash: None, stroke_dash_offset: None,
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
        fn units_per_em(&self, _: &FakeHandle) -> u32 { 1000 }
        fn ascent(&self, _: &FakeHandle) -> i32 { 800 }
        fn descent(&self, _: &FakeHandle) -> i32 { 200 }
        fn line_gap(&self, _: &FakeHandle) -> i32 { 0 }
        fn x_height(&self, _: &FakeHandle) -> Option<i32> { Some(500) }
        fn cap_height(&self, _: &FakeHandle) -> Option<i32> { Some(700) }
        fn family_name(&self, _: &FakeHandle) -> String { "Fake".into() }
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
        fn font_ref(&self, _h: &FakeHandle) -> String { "fake:test".into() }
    }

    fn make_opts<'a>(
        shaper: &'a FakeShaper,
        metrics: &'a FakeMetrics,
        resolver: &'a FakeResolver,
    ) -> DiagramToPaintOptions<'a, FakeShaper, FakeMetrics, FakeResolver> {
        DiagramToPaintOptions {
            background: Color { r: 255, g: 255, b: 255, a: 255 },
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
            fill:          "none".to_string(),
            stroke:        "#4b5563".to_string(),
            stroke_width:  2.0,
            text_color:    "#374151".to_string(),
            font_size:     12.0,
            corner_radius: 0.0,
        }
    }

    fn simple_layout() -> LayoutedGraphDiagram {
        LayoutedGraphDiagram {
            direction: DiagramDirection::Lr,
            title: None,
            width: 400.0,
            height: 200.0,
            nodes: vec![
                LayoutedGraphNode {
                    id: "A".to_string(),
                    label: DiagramLabel::new("Start"),
                    shape: DiagramShape::RoundedRect,
                    x: 24.0, y: 24.0,
                    width: 96.0, height: 52.0,
                    style: default_style(),
                },
                LayoutedGraphNode {
                    id: "B".to_string(),
                    label: DiagramLabel::new("End"),
                    shape: DiagramShape::RoundedRect,
                    x: 216.0, y: 24.0,
                    width: 96.0, height: 52.0,
                    style: default_style(),
                },
            ],
            edges: vec![LayoutedGraphEdge {
                id: None,
                from_node_id: "A".to_string(),
                to_node_id: "B".to_string(),
                kind: EdgeKind::Directed,
                points: vec![
                    Point { x: 120.0, y: 50.0 },
                    Point { x: 216.0, y: 50.0 },
                ],
                label: None,
                label_position: None,
                style: edge_style(),
            }],
        }
    }

    #[test]
    fn version_exists() {
        assert_eq!(crate::VERSION, "0.2.0");
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
    fn two_nodes_produce_two_rects() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&simple_layout(), &opts);
        let rects = scene.instructions.iter()
            .filter(|i| matches!(i, PaintInstruction::Rect(_)))
            .count();
        assert_eq!(rects, 2, "two RoundedRect nodes → two PaintRect instructions");
    }

    #[test]
    fn node_labels_emit_glyph_runs() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&simple_layout(), &opts);
        let runs = scene.instructions.iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_)))
            .count();
        // "Start" (5 chars) and "End" (3 chars) each produce one PaintGlyphRun.
        assert!(runs >= 2, "expected at least 2 PaintGlyphRuns for node labels, got {}", runs);
    }

    #[test]
    fn directed_edge_produces_arrowhead_path() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&simple_layout(), &opts);
        let paths = scene.instructions.iter()
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
        let paths = scene.instructions.iter()
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
        let ellipses = scene.instructions.iter()
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
        let diamond_paths: Vec<_> = scene.instructions.iter()
            .filter_map(|i| if let PaintInstruction::Path(p) = i { Some(p) } else { None })
            .filter(|p| p.commands.len() == 5)
            .collect();
        assert!(!diamond_paths.is_empty(), "expected a diamond PaintPath with 5 commands");
    }

    #[test]
    fn title_produces_extra_glyph_run() {
        let mut layout = simple_layout();
        layout.title = Some("My Diagram".to_string());
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene_with    = diagram_to_paint(&layout, &opts);

        let layout_no = simple_layout();
        let opts2 = make_opts(&shaper, &metrics, &resolver);
        let scene_without = diagram_to_paint(&layout_no, &opts2);

        let runs_with    = scene_with.instructions.iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_))).count();
        let runs_without = scene_without.instructions.iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_))).count();
        assert!(runs_with > runs_without, "title should add at least one glyph run");
    }

    #[test]
    fn glyph_run_font_ref_is_shaper_provided() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&simple_layout(), &opts);
        let run = scene.instructions.iter()
            .find(|i| matches!(i, PaintInstruction::GlyphRun(_)));
        if let Some(PaintInstruction::GlyphRun(gr)) = run {
            // The FakeShaper always returns "fake:test" as font_ref.
            assert_eq!(gr.font_ref, "fake:test",
                "font_ref should come from the shaper, not a hardcoded string");
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
        let scene_no_label   = diagram_to_paint(&simple_layout(), &opts2);

        let runs_with = scene_with_label.instructions.iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_))).count();
        let runs_without = scene_no_label.instructions.iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_))).count();
        assert!(runs_with > runs_without, "edge label should produce at least one extra glyph run");
    }

    #[test]
    fn css_to_color_parses_hex() {
        assert_eq!(css_to_color("#4b5563"), Color { r: 0x4b, g: 0x55, b: 0x63, a: 255 });
        assert_eq!(css_to_color("#ffffff"), Color { r: 255, g: 255, b: 255, a: 255 });
        // Invalid/unsupported → opaque black
        assert_eq!(css_to_color("none"), Color { r: 0, g: 0, b: 0, a: 255 });
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

        let last_path_idx = scene.instructions.iter().rposition(|i| matches!(i, PaintInstruction::Path(_)));
        let first_rect_idx = scene.instructions.iter().position(|i| matches!(i, PaintInstruction::Rect(_)));
        if let (Some(lp), Some(fr)) = (last_path_idx, first_rect_idx) {
            assert!(lp < fr, "all edge paths should appear before node rects");
        }
    }
}
