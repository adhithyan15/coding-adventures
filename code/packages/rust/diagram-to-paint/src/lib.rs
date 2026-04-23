//! # diagram-to-paint
//!
//! Lowers a [`LayoutedGraphDiagram`] into a [`PaintScene`] for rendering
//! by `paint-metal` and other paint backends.
//!
//! This crate is the final step of the DG01 diagram pipeline:
//!
//! ```text
//! LayoutedGraphDiagram (diagram-ir)
//!   → diagram-to-paint (this crate)
//!   → PaintScene (paint-instructions)
//!   → paint-metal (GPU render)
//! ```
//!
//! ## What each diagram element becomes
//!
//! | Element          | Paint instruction(s)                             |
//! |------------------|--------------------------------------------------|
//! | Rect node        | `PaintRect` (corner_radius = 0)                  |
//! | RoundedRect node | `PaintRect` (corner_radius from style)           |
//! | Ellipse node     | `PaintEllipse`                                   |
//! | Diamond node     | `PaintPath` (4-vertex diamond polygon)           |
//! | Node label       | `PaintGlyphRun` (coretext: font scheme)          |
//! | Edge line        | `PaintPath` (polyline stroke)                    |
//! | Arrowhead        | `PaintPath` (filled triangle)                    |
//! | Edge label       | `PaintGlyphRun`                                  |
//! | Diagram title    | `PaintGlyphRun`                                  |
//!
//! ## Font scheme
//!
//! All text uses `coretext:Helvetica@<size>` — the scheme recognised by
//! `paint-metal`'s CoreText glyph-run overlay. Glyph IDs are Unicode
//! codepoints (one per character); CoreText resolves them to CGGlyph via
//! `CTFontGetGlyphsForCharacters`.
//!
//! The approximate advance per character is `font_size × 0.55` — accurate
//! enough for monospaced-ish label centering in diagram nodes.

pub const VERSION: &str = "0.1.0";

use diagram_ir::{
    DiagramShape, EdgeKind, LayoutedGraphDiagram, LayoutedGraphEdge, LayoutedGraphNode, Point,
};
use paint_instructions::{
    GlyphPosition, PaintBase, PaintEllipse, PaintGlyphRun, PaintInstruction, PaintPath, PaintRect,
    PaintScene, PathCommand, StrokeCap, StrokeJoin,
};

// ============================================================================
// Options
// ============================================================================

/// Rendering options for `diagram_to_paint`.
#[derive(Clone, Debug)]
pub struct DiagramToPaintOptions {
    /// Canvas background colour (default `"#ffffff"`).
    pub background: Option<String>,
    /// PostScript font name for CoreText (default `"Helvetica"`).
    pub ps_font_name: Option<String>,
    /// Font size for the diagram title (default `18.0`).
    pub title_font_size: Option<f64>,
}

struct Opts {
    background:       String,
    ps_font_name:     String,
    title_font_size:  f64,
}

impl Opts {
    fn from(options: Option<&DiagramToPaintOptions>) -> Self {
        let o = options.cloned().unwrap_or(DiagramToPaintOptions {
            background: None,
            ps_font_name: None,
            title_font_size: None,
        });
        Opts {
            background:      o.background.unwrap_or_else(|| "#ffffff".to_string()),
            ps_font_name:    o.ps_font_name.unwrap_or_else(|| "Helvetica".to_string()),
            title_font_size: o.title_font_size.unwrap_or(18.0),
        }
    }
}

// ============================================================================
// Font reference helpers
// ============================================================================

/// Build a CoreText font reference string recognised by `paint-metal`.
///
/// Format: `coretext:<PostScript-name>@<size>`.
///
/// Example: `coretext:Helvetica@14` — Helvetica at 14pt.
fn coretext_font_ref(ps_name: &str, size: f64) -> String {
    format!("coretext:{}@{}", ps_name, size)
}

/// Approximate character advance in pixels for a given font size.
///
/// This is a heuristic for Helvetica / system sans-serif fonts. A real shaper
/// would measure individual glyph advances; 0.55 × font_size is close for
/// ASCII in common Latin-script fonts and keeps label centering reasonable.
fn approx_char_advance(font_size: f64) -> f64 {
    font_size * 0.55
}

// ============================================================================
// Text / glyph run helpers
// ============================================================================

/// Build a centred `PaintGlyphRun` for a label inside a bounding box.
///
/// `cx`, `cy` — visual centre of the bounding box (not the baseline).
/// The Y baseline is `cy + font_size × 0.35` — adjusts for cap-height so
/// text appears visually centred in the box.
fn centred_glyph_run(
    text: &str,
    cx: f64,
    cy: f64,
    font_size: f64,
    fill: &str,
    ps_name: &str,
) -> PaintGlyphRun {
    let advance = approx_char_advance(font_size);
    let total_width = text.len() as f64 * advance;
    let start_x = cx - total_width / 2.0;
    let baseline_y = cy + font_size * 0.35;

    let glyphs: Vec<GlyphPosition> = text
        .chars()
        .enumerate()
        .map(|(i, ch)| GlyphPosition {
            glyph_id: ch as u32,
            x: start_x + i as f64 * advance,
            y: baseline_y,
        })
        .collect();

    PaintGlyphRun {
        base: PaintBase::default(),
        glyphs,
        font_ref: coretext_font_ref(ps_name, font_size),
        font_size,
        fill: Some(fill.to_string()),
    }
}

// ============================================================================
// Node shape rendering
// ============================================================================

fn node_shape_instruction(node: &LayoutedGraphNode) -> PaintInstruction {
    match node.shape {
        DiagramShape::Ellipse => {
            PaintInstruction::Ellipse(PaintEllipse {
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
            })
        }
        DiagramShape::Diamond => {
            let cx = node.x + node.width / 2.0;
            let cy = node.y + node.height / 2.0;
            PaintInstruction::Path(PaintPath {
                base: PaintBase::default(),
                commands: vec![
                    PathCommand::MoveTo { x: cx,               y: node.y },
                    PathCommand::LineTo { x: node.x + node.width, y: cy },
                    PathCommand::LineTo { x: cx,               y: node.y + node.height },
                    PathCommand::LineTo { x: node.x,            y: cy },
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
        DiagramShape::Rect => {
            PaintInstruction::Rect(PaintRect {
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
            })
        }
        DiagramShape::RoundedRect => {
            PaintInstruction::Rect(PaintRect {
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
            })
        }
    }
}

// ============================================================================
// Edge rendering
// ============================================================================

/// Build a `PaintPath` polyline for an edge route.
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

/// Compute and return a filled triangle arrowhead `PaintPath`, or `None` for
/// undirected edges or degenerate routes.
///
/// The arrowhead is a small isosceles triangle at the `end` point of the edge,
/// pointing in the direction of the last segment.
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

    let ux = dx / len; // unit vector along edge
    let uy = dy / len;

    let size    = 10.0; // arrowhead length
    let half_w  = size * 0.6; // half-width of the arrowhead base

    // Base centre is `size` pixels behind the tip.
    let base_x = end.x - ux * size;
    let base_y = end.y - uy * size;

    // Perpendicular vector (rotated 90° CCW).
    let px = -uy;
    let py =  ux;

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

fn edge_instructions(
    edge: &LayoutedGraphEdge,
    opts: &Opts,
) -> Vec<PaintInstruction> {
    let mut instructions = Vec::new();

    // The polyline path.
    instructions.push(PaintInstruction::Path(line_path(
        &edge.points,
        &edge.style.stroke,
        edge.style.stroke_width,
    )));

    // Arrowhead for directed edges.
    if let Some(tip) = arrowhead(edge) {
        instructions.push(PaintInstruction::Path(tip));
    }

    // Edge label.
    if let (Some(label), Some(pos)) = (&edge.label, &edge.label_position) {
        instructions.push(PaintInstruction::GlyphRun(centred_glyph_run(
            &label.text,
            pos.x,
            pos.y,
            edge.style.font_size,
            &edge.style.text_color,
            &opts.ps_font_name,
        )));
    }

    instructions
}

// ============================================================================
// Public API
// ============================================================================

/// Lower a [`LayoutedGraphDiagram`] into a [`PaintScene`].
///
/// # Example
///
/// ```rust,ignore
/// use diagram_ir::{GraphDiagram, DiagramDirection, GraphNode, GraphEdge,
///                   DiagramLabel, EdgeKind};
/// use diagram_layout_graph::layout_graph_diagram;
/// use diagram_to_paint::diagram_to_paint;
///
/// // Build a minimal two-node graph.
/// let diagram = GraphDiagram {
///     direction: DiagramDirection::Lr,
///     title: None,
///     nodes: vec![
///         GraphNode { id: "A".into(), label: DiagramLabel::new("Start"),
///                     shape: None, style: None },
///         GraphNode { id: "B".into(), label: DiagramLabel::new("End"),
///                     shape: None, style: None },
///     ],
///     edges: vec![GraphEdge { id: None, from: "A".into(), to: "B".into(),
///         label: None, kind: EdgeKind::Directed, style: None }],
/// };
///
/// let layout = layout_graph_diagram(&diagram, None);
/// let scene  = diagram_to_paint(&layout, None);
///
/// assert!(scene.width > 0.0);
/// assert!(!scene.instructions.is_empty());
/// ```
pub fn diagram_to_paint(
    diagram: &LayoutedGraphDiagram,
    options: Option<&DiagramToPaintOptions>,
) -> PaintScene {
    let opts = Opts::from(options);
    let mut instructions: Vec<PaintInstruction> = Vec::new();

    // Optional title — centred at the top of the canvas.
    if let Some(title) = &diagram.title {
        instructions.push(PaintInstruction::GlyphRun(centred_glyph_run(
            title,
            diagram.width / 2.0,
            // Vertically centre in the title band above the nodes.
            opts.title_font_size + 8.0,
            opts.title_font_size,
            "#111827",
            &opts.ps_font_name,
        )));
    }

    // Edges first (drawn behind nodes).
    for edge in &diagram.edges {
        instructions.extend(edge_instructions(edge, &opts));
    }

    // Nodes: shape then label (label on top of shape).
    for node in &diagram.nodes {
        instructions.push(node_shape_instruction(node));
        instructions.push(PaintInstruction::GlyphRun(centred_glyph_run(
            &node.label.text,
            node.x + node.width / 2.0,
            node.y + node.height / 2.0,
            node.style.font_size,
            &node.style.text_color,
            &opts.ps_font_name,
        )));
    }

    PaintScene {
        width: diagram.width,
        height: diagram.height,
        background: opts.background,
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
        DiagramDirection, DiagramLabel, DiagramShape, EdgeKind,
        LayoutedGraphDiagram, LayoutedGraphEdge, LayoutedGraphNode, Point, ResolvedDiagramStyle,
    };

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
        let layout = simple_layout();
        let scene  = diagram_to_paint(&layout, None);
        assert_eq!(scene.width, 400.0);
        assert_eq!(scene.height, 200.0);
    }

    #[test]
    fn scene_has_background() {
        let scene = diagram_to_paint(&simple_layout(), None);
        assert_eq!(scene.background, "#ffffff");
    }

    #[test]
    fn scene_is_not_empty() {
        let scene = diagram_to_paint(&simple_layout(), None);
        assert!(!scene.instructions.is_empty());
    }

    #[test]
    fn two_nodes_produce_rect_and_glyph_instructions() {
        let scene = diagram_to_paint(&simple_layout(), None);
        let rects = scene.instructions.iter()
            .filter(|i| matches!(i, PaintInstruction::Rect(_)))
            .count();
        let runs = scene.instructions.iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_)))
            .count();
        assert_eq!(rects, 2, "two nodes → two rect instructions");
        // 2 node labels + 1 arrowhead glyph run (none, arrowhead is a path)
        assert_eq!(runs, 2, "two node labels → two glyph run instructions");
    }

    #[test]
    fn directed_edge_produces_arrowhead_path() {
        let scene = diagram_to_paint(&simple_layout(), None);
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
        let scene = diagram_to_paint(&layout, None);
        let paths = scene.instructions.iter()
            .filter(|i| matches!(i, PaintInstruction::Path(_)))
            .count();
        assert_eq!(paths, 1, "undirected edge should have only the polyline, no arrowhead");
    }

    #[test]
    fn ellipse_node_produces_ellipse_instruction() {
        let mut layout = simple_layout();
        layout.nodes[0].shape = DiagramShape::Ellipse;
        let scene = diagram_to_paint(&layout, None);
        let ellipses = scene.instructions.iter()
            .filter(|i| matches!(i, PaintInstruction::Ellipse(_)))
            .count();
        assert_eq!(ellipses, 1);
    }

    #[test]
    fn diamond_node_produces_path_instruction() {
        let mut layout = simple_layout();
        layout.nodes[0].shape = DiagramShape::Diamond;
        let scene = diagram_to_paint(&layout, None);
        // Diamond + edge line + arrowhead = 3 paths
        let paths = scene.instructions.iter()
            .filter(|i| matches!(i, PaintInstruction::Path(_)))
            .count();
        assert!(paths >= 1);
        // The first path should have 5 commands (4 diamond sides + Close).
        let first_path = scene.instructions.iter()
            .find(|i| matches!(i, PaintInstruction::Path(_)));
        if let Some(PaintInstruction::Path(_p)) = first_path {
            // Diamond is 4 lines + close = 5 commands.
            // But edges are emitted first so the first Path may be the edge.
            // Just confirm at least one path has 5 commands.
            let diamond_paths: Vec<_> = scene.instructions.iter()
                .filter_map(|i| if let PaintInstruction::Path(p) = i { Some(p) } else { None })
                .filter(|p| p.commands.len() == 5)
                .collect();
            assert!(!diamond_paths.is_empty(), "expected a diamond path with 5 commands");
        }
    }

    #[test]
    fn title_adds_glyph_run() {
        let mut layout = simple_layout();
        layout.title = Some("My Diagram".to_string());
        let scene = diagram_to_paint(&layout, None);
        let runs = scene.instructions.iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_)))
            .count();
        // 2 node labels + 1 title = 3
        assert_eq!(runs, 3);
    }

    #[test]
    fn font_ref_uses_coretext_scheme() {
        let scene = diagram_to_paint(&simple_layout(), None);
        let run = scene.instructions.iter()
            .find(|i| matches!(i, PaintInstruction::GlyphRun(_)));
        if let Some(PaintInstruction::GlyphRun(gr)) = run {
            assert!(gr.font_ref.starts_with("coretext:"), "font_ref should use coretext: scheme");
        }
    }

    #[test]
    fn custom_ps_font_name_used_in_font_ref() {
        let opts = DiagramToPaintOptions {
            background: None,
            ps_font_name: Some("Arial-BoldMT".to_string()),
            title_font_size: None,
        };
        let scene = diagram_to_paint(&simple_layout(), Some(&opts));
        let run = scene.instructions.iter()
            .find(|i| matches!(i, PaintInstruction::GlyphRun(_)));
        if let Some(PaintInstruction::GlyphRun(gr)) = run {
            assert!(gr.font_ref.contains("Arial-BoldMT"));
        }
    }

    #[test]
    fn edge_with_label_produces_three_glyph_runs() {
        let mut layout = simple_layout();
        layout.edges[0].label = Some(DiagramLabel::new("transfers"));
        layout.edges[0].label_position = Some(Point { x: 168.0, y: 42.0 });
        let scene = diagram_to_paint(&layout, None);
        let runs = scene.instructions.iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_)))
            .count();
        // 2 node labels + 1 edge label = 3
        assert_eq!(runs, 3);
    }

    #[test]
    fn glyph_positions_are_spaced() {
        let scene = diagram_to_paint(&simple_layout(), None);
        if let Some(PaintInstruction::GlyphRun(gr)) = scene.instructions.iter()
            .find(|i| matches!(i, PaintInstruction::GlyphRun(_)))
        {
            if gr.glyphs.len() >= 2 {
                let x0 = gr.glyphs[0].x;
                let x1 = gr.glyphs[1].x;
                assert!(x1 > x0, "glyph positions should increase left to right");
            }
        }
    }
}
