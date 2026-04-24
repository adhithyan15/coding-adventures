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

pub const VERSION: &str = "0.1.0";

use std::collections::HashMap;

use diagram_ir::{
    DiagramShape, EdgeKind, LayoutedGraphDiagram, LayoutedGraphEdge, LayoutedGraphNode, Point,
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

    let text_opts = LayoutToPaintOptions {
        width: diagram.width,
        height: diagram.height,
        background: Color { r: 0, g: 0, b: 0, a: 0 }, // transparent root
        device_pixel_ratio: options.device_pixel_ratio,
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
        assert_eq!(VERSION, "0.1.0");
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
