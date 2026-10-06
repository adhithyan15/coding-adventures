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

pub const VERSION: &str = "0.75.0";

use std::collections::HashMap;

use diagram_ir::{
    ChartTextAnchor, ChartTextBaseline, DiagramLabel, DiagramShape, EdgeKind, GeoElement, GitCommitSymbol,
    LayoutedChartDiagram, LayoutedChartItem,
    EdgeMarker, EventModelEntityKind, LayoutedEventModelDiagram, LayoutedEventModelItem,
    LayoutedCynefinDiagram, LayoutedInfoDiagram, LayoutedIshikawaDiagram, LayoutedSwimlaneDiagram, LayoutedRailroadDiagram,
    LayoutedTreeViewDiagram, LayoutedTreemapDiagram, LayoutedVennDiagram, LayoutedWardleyDiagram,
    LayoutedGeometricDiagram, LayoutedGraphDiagram, LayoutedGraphEdge, LayoutedGraphNode,
    LayoutedBoardCard, LayoutedBoardDiagram, LayoutedPacketDiagram,
    LayoutedSequenceDiagram, LayoutedSequenceItem, LayoutedStructuralDiagram,
    LayoutedTemporalDiagram, LayoutedTemporalItem, Orientation, Point, RelKind, SequenceArrowhead,
    SequenceBlockKind, SequenceCentralConnection, SequenceLineStyle, SequenceParticipantKind,
    GanttTaskTags, SequenceProperty, SwimlaneEdgeKind, TextAlign as GeoTextAlign, TreeViewNodeKind,
    RailroadElementKind, StructuralNodeKind,
};
use layout_ir::{
    Color, Content, ExtValue, FontSpec, FontStretch, PositionedNode, TextAlign, TextContent,
    TextDecoration, TextDecorationLines, TextDecorationStyle, TextUnderlinePosition,
};
use layout_effects::{EffectColor, EffectFilter, EffectStyle};
use layout_to_paint::{layout_to_paint, LayoutToPaintOptions};
use paint_instructions::{
    GlyphPosition, PaintBase, PaintEllipse, PaintGlyphRun, PaintGroup, PaintInstruction, PaintPath,
    PaintRect, PaintScene, PathCommand, StrokeCap, StrokeJoin,
};
use text_interfaces::{
    FontMetrics, FontQuery, FontResolver, FontStyle, FontWeight, ShapeOptions, TextShaper,
};

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
    let mut instructions = Vec::new();
    let mut text_children = Vec::new();
    let text_color = Color { r: 15, g: 23, b: 42, a: 255 };

    if let Some(title) = &diagram.title {
        let mut title_font = options.title_font.clone();
        title_font.size = diagram.config.title_font_size.unwrap_or(title_font.size);
        let title_color = diagram.config.title_color.as_deref().map(css_to_color).unwrap_or(text_color);
        text_children.push(text_node(title, 8.0, 6.0, diagram.width - 16.0, 30.0, title_font, title_color));
    }
    for node in &diagram.nodes {
        if node.width <= 0.0 || node.height <= 0.0 {
            continue;
        }
        if node.depth == 0 && node.has_children {
            continue;
        }
        let palette_index = node.palette_index;
        let configured_fill = if node.has_children { &diagram.config.section_fill_color } else { &diagram.config.leaf_fill_color };
        let configured_stroke = if node.has_children { &diagram.config.section_stroke_color } else { &diagram.config.leaf_stroke_color };
        let configured_stroke_width = if node.has_children { diagram.config.section_stroke_width } else { diagram.config.leaf_stroke_width };
        let current_color = treemap_current_color(node, diagram, text_color);
        let resolve_current_color = |value: String| {
            if value.eq_ignore_ascii_case("currentcolor") { current_color.clone() } else { value }
        };
        let fill = normalize_css_paint(resolve_current_color(node.style.as_ref().and_then(|style| style.node.fill.clone())
            .or_else(|| configured_fill.clone())
            .unwrap_or_else(|| palette_index.map_or_else(|| "transparent".into(), |index| diagram.config.theme.fills[index].clone()))));
        let stroke = normalize_css_paint(resolve_current_color(node.style.as_ref().and_then(|style| style.node.stroke.clone())
            .or_else(|| configured_stroke.clone())
            .unwrap_or_else(|| palette_index.map_or_else(|| "transparent".into(), |index| diagram.config.theme.strokes[index].clone()))));
        let opacity = node.style.as_ref().and_then(|style| style.opacity).unwrap_or(1.0);
        let fill = if node.style.as_ref().is_some_and(|style| style.opacity.is_some() || style.fill_opacity.is_some()) {
            with_opacity(&fill, node.style.as_ref().and_then(|style| style.fill_opacity).unwrap_or(1.0) * opacity)
        } else { fill };
        let stroke = if node.style.as_ref().is_some_and(|style| style.opacity.is_some() || style.stroke_opacity.is_some()) {
            with_opacity(&stroke, node.style.as_ref().and_then(|style| style.stroke_opacity).unwrap_or(1.0) * opacity)
        } else { stroke };
        instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: node.x,
            y: node.y,
            width: node.width,
            height: node.height,
            fill: Some(fill),
            stroke: Some(stroke),
            stroke_width: Some(node.style.as_ref().and_then(|style| style.node.stroke_width)
                .or(configured_stroke_width).unwrap_or(diagram.config.border_width)),
            corner_radius: Some(treemap_corner_radius(node)),
            stroke_dash: node.style.as_ref().and_then(|style| style.node.stroke_dash.clone()),
            stroke_dash_offset: node.style.as_ref().and_then(|style| style.stroke_dash_offset),
        }));
        if node.width >= 44.0 && node.height >= 22.0 {
            let mut label_font = options.label_font.clone();
            label_font.size = node.style.as_ref().and_then(|style| style.node.font_size).unwrap_or(diagram.config.label_font_size);
            apply_treemap_font_size(&mut label_font, node.style.as_ref());
            if let Some(style) = &node.style {
                label_font.weight = style.node.font_weight.unwrap_or(label_font.weight);
                label_font.italic = style.node.font_italic.unwrap_or(label_font.italic);
                if let Some(family) = &style.node.font_family { label_font.family.clone_from(family); }
            }
            apply_treemap_line_height(&mut label_font, node.style.as_ref());
            let palette_text_color = || palette_index
                    .map(|index| css_to_color(&diagram.config.theme.labels[index]))
                    .unwrap_or(text_color);
            let label_text_color = node.style.as_ref().and_then(|style| style.node.text_color.as_deref())
                .or(diagram.config.label_color.as_deref()).map(css_to_color).unwrap_or_else(palette_text_color);
            let label_text_color = color_with_opacity(label_text_color, opacity);
            let label_height = label_font.size * label_font.line_height;
            text_children.push(treemap_text_node(
                &node.label,
                node.x + 6.0,
                node.y + 4.0,
                (node.width * if node.has_children { 0.68 } else { 1.0 } - 12.0).max(0.0),
                label_height,
                label_font,
                label_text_color,
                node.style.as_ref(),
            ));
            if diagram.config.show_values && node.height >= 42.0 {
                let mut value_font = options.label_font.clone();
                value_font.size = node.style.as_ref().and_then(|style| style.node.font_size).unwrap_or(diagram.config.value_font_size);
                apply_treemap_font_size(&mut value_font, node.style.as_ref());
                if let Some(style) = &node.style {
                    value_font.weight = style.node.font_weight.unwrap_or(value_font.weight);
                    value_font.italic = style.node.font_italic.unwrap_or(value_font.italic);
                    if let Some(family) = &style.node.font_family { value_font.family.clone_from(family); }
                }
                apply_treemap_line_height(&mut value_font, node.style.as_ref());
                let (value_x, value_y, value_width) = if node.has_children {
                    (node.x + node.width * 0.68, node.y + 4.0, node.width * 0.32 - 6.0)
                } else {
                    (node.x + 6.0, node.y + 8.0 + diagram.config.label_font_size * 1.2, node.width - 12.0)
                };
                let value_text_color = node.style.as_ref().and_then(|style| style.node.text_color.as_deref())
                    .or(diagram.config.value_color.as_deref()).map(css_to_color).unwrap_or_else(palette_text_color);
                let value_text_color = color_with_opacity(value_text_color, opacity);
                let value_height = value_font.size * value_font.line_height;
                text_children.push(treemap_text_node(
                    &format_treemap_value(node.value, &diagram.config.value_format),
                    value_x,
                    value_y,
                    value_width.max(0.0),
                    value_height,
                    value_font,
                    value_text_color,
                    node.style.as_ref(),
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
    metadata.insert("treemap.useMaxWidth".into(), diagram.config.use_max_width.to_string());
    metadata.insert("treemap.valueFormat".into(), diagram.config.value_format.clone());
    PaintScene {
        width: diagram.width,
        height: diagram.height,
        background: format!("rgb({},{},{})", options.background.r, options.background.g, options.background.b),
        instructions,
        id: None,
        metadata: (!metadata.is_empty()).then_some(metadata),
    }
}

fn format_treemap_value(value: f64, format: &str) -> String {
    let spec = parse_treemap_number_format(format).unwrap_or_else(|| parse_treemap_number_format(",").unwrap());
    let magnitude = value.abs();
    let mut rendered = match spec.kind {
        Some('b') => format!("{:b}", magnitude.round() as i128),
        Some('d') => format!("{magnitude:.0}"),
        Some('e' | 'E') => {
            let precision = spec.precision.unwrap_or(6);
            let result = format!("{magnitude:.precision$e}");
            if spec.kind == Some('E') { result.to_ascii_uppercase() } else { result }
        }
        Some('f' | 'F') => format!("{magnitude:.precision$}", precision = spec.precision.unwrap_or(6)),
        Some('g' | 'G' | 'r') => format_significant(magnitude, spec.precision.unwrap_or(6)),
        Some('o') => format!("{:o}", magnitude.round() as i128),
        Some('p') => format_significant(magnitude * 100.0, spec.precision.unwrap_or(6)) + "%",
        Some('%') => format!("{:.precision$}%", magnitude * 100.0, precision = spec.precision.unwrap_or(6)),
        Some('s') => format_si(magnitude, spec.precision.unwrap_or(6)),
        Some('x') => format!("{:x}", magnitude.round() as i128),
        Some('X') => format!("{:X}", magnitude.round() as i128),
        _ if spec.precision.is_some() => format!("{magnitude:.precision$}", precision = spec.precision.unwrap_or(0)),
        _ if magnitude.fract() == 0.0 => format!("{magnitude:.0}"),
        _ => magnitude.to_string(),
    };
    if spec.trim { rendered = trim_decimal_zeroes(rendered); }
    if spec.grouped { rendered = group_decimal_thousands(&rendered); }
    let mut prefix = match (value.is_sign_negative(), spec.sign) {
        (true, '(') => "(".into(),
        (true, _) => "-".into(),
        (false, '+') => "+".into(),
        (false, ' ') => " ".into(),
        _ => String::new(),
    };
    match spec.symbol {
        Some('$') => prefix.push('$'),
        Some('#') if matches!(spec.kind, Some('b')) => prefix.push_str("0b"),
        Some('#') if matches!(spec.kind, Some('o')) => prefix.push_str("0o"),
        Some('#') if matches!(spec.kind, Some('x')) => prefix.push_str("0x"),
        Some('#') if matches!(spec.kind, Some('X')) => prefix.push_str("0X"),
        _ => {}
    }
    let suffix = if value.is_sign_negative() && spec.sign == '(' { ")" } else { "" };
    align_treemap_number(prefix, rendered, suffix, &spec)
}

#[allow(clippy::too_many_arguments)]
fn treemap_text_node(
    value: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    font: FontSpec,
    color: Color,
    style: Option<&diagram_ir::TreemapStyle>,
) -> PositionedNode {
    let mut font = font;
    font.stretch = match style.and_then(|style| style.font_stretch) {
        Some(diagram_ir::TreemapFontStretch::UltraCondensed) => FontStretch::UltraCondensed,
        Some(diagram_ir::TreemapFontStretch::ExtraCondensed) => FontStretch::ExtraCondensed,
        Some(diagram_ir::TreemapFontStretch::Condensed) => FontStretch::Condensed,
        Some(diagram_ir::TreemapFontStretch::SemiCondensed) => FontStretch::SemiCondensed,
        Some(diagram_ir::TreemapFontStretch::SemiExpanded) => FontStretch::SemiExpanded,
        Some(diagram_ir::TreemapFontStretch::Expanded) => FontStretch::Expanded,
        Some(diagram_ir::TreemapFontStretch::ExtraExpanded) => FontStretch::ExtraExpanded,
        Some(diagram_ir::TreemapFontStretch::UltraExpanded) => FontStretch::UltraExpanded,
        Some(diagram_ir::TreemapFontStretch::Percentage(value)) => font_stretch_from_percentage(value),
        Some(diagram_ir::TreemapFontStretch::Normal) | None => FontStretch::Normal,
    };
    let text_indent = match style.and_then(|style| style.text_indent) {
        Some(diagram_ir::TreemapTextIndent::Pixels(value)) => value,
        Some(diagram_ir::TreemapTextIndent::Factor(value)) => width * value,
        None => 0.0,
    };
    let decoration_thickness = style.and_then(|style| style.text_decoration_thickness).and_then(|thickness| match thickness {
        diagram_ir::TreemapTextDecorationThickness::Pixels(value) => Some(value),
        diagram_ir::TreemapTextDecorationThickness::Factor(value) => Some(font.size * value),
        diagram_ir::TreemapTextDecorationThickness::Auto | diagram_ir::TreemapTextDecorationThickness::FromFont => None,
    });
    let underline_offset = style.and_then(|style| style.text_underline_offset).and_then(|offset| match offset {
        diagram_ir::TreemapTextUnderlineOffset::Pixels(value) => Some(value),
        diagram_ir::TreemapTextUnderlineOffset::Factor(value) => Some(font.size * value),
        diagram_ir::TreemapTextUnderlineOffset::Auto => None,
    });
    let underline_position = match style.and_then(|style| style.text_underline_position) {
        Some(diagram_ir::TreemapTextUnderlinePosition::FromFont) => TextUnderlinePosition::FromFont,
        Some(diagram_ir::TreemapTextUnderlinePosition::Under) => TextUnderlinePosition::Under,
        Some(diagram_ir::TreemapTextUnderlinePosition::Auto) | None => TextUnderlinePosition::Auto,
    };
    let white_space = style.and_then(|style| style.white_space);
    let whitespace_value = match white_space {
        None | Some(diagram_ir::TreemapWhiteSpace::Normal | diagram_ir::TreemapWhiteSpace::NoWrap) =>
            value.split_whitespace().collect::<Vec<_>>().join(" "),
        Some(diagram_ir::TreemapWhiteSpace::PreLine) => value.lines()
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>().join("\n"),
        Some(diagram_ir::TreemapWhiteSpace::PreserveSpaces) => value.chars()
            .map(|character| if matches!(character, '\t' | '\r' | '\n') { ' ' } else { character })
            .collect(),
        Some(diagram_ir::TreemapWhiteSpace::Pre | diagram_ir::TreemapWhiteSpace::PreWrap
            | diagram_ir::TreemapWhiteSpace::BreakSpaces) => {
            let tab = " ".repeat(style.and_then(|style| style.tab_size).unwrap_or(8) as usize);
            value.replace('\t', &tab)
        }
    };
    let value = match style.and_then(|style| style.text_transform) {
        Some(diagram_ir::TreemapTextTransform::Uppercase) => whitespace_value.to_uppercase(),
        Some(diagram_ir::TreemapTextTransform::Lowercase) => whitespace_value.to_lowercase(),
        Some(diagram_ir::TreemapTextTransform::Capitalize) => capitalize_words(&whitespace_value),
        Some(diagram_ir::TreemapTextTransform::FullWidth) => whitespace_value.chars().map(|character| match character {
            ' ' => '\u{3000}',
            '!'..='~' => char::from_u32(character as u32 + 0xfee0).expect("ASCII full-width mapping is valid"),
            _ => character,
        }).collect(),
        Some(diagram_ir::TreemapTextTransform::FullSizeKana) => full_size_kana(&whitespace_value),
        _ => whitespace_value,
    };
    let font_size = font.size;
    let mut node = text_node(&value, x, y, width, height, font, color);
    if let Some(Content::Text(content)) = &mut node.content {
        content.text_align = match style.and_then(|style| style.text_align) {
            Some(diagram_ir::TreemapTextAlign::Start) => TextAlign::Start,
            Some(diagram_ir::TreemapTextAlign::End) => TextAlign::End,
            Some(diagram_ir::TreemapTextAlign::Justify) => TextAlign::Start,
            _ => TextAlign::Center,
        };
        content.wrap = !matches!(style.and_then(|style| style.white_space),
            Some(diagram_ir::TreemapWhiteSpace::NoWrap | diagram_ir::TreemapWhiteSpace::Pre));
        content.decoration = style.and_then(|style| style.text_decoration).and_then(|decoration| {
            let mut lines = TextDecorationLines::NONE;
            if decoration.underline { lines = lines.union(TextDecorationLines::UNDERLINE); }
            if decoration.overline { lines = lines.union(TextDecorationLines::OVERLINE); }
            if decoration.line_through { lines = lines.union(TextDecorationLines::LINE_THROUGH); }
            let decoration_color = style.and_then(|style| style.text_decoration_color.as_ref()).map(|authored| match authored {
                diagram_ir::TreemapTextDecorationColor::CurrentColor => color,
                diagram_ir::TreemapTextDecorationColor::Color(value) => color_with_opacity(
                    css_to_color(value), style.and_then(|style| style.opacity).unwrap_or(1.0),
                ),
            });
            let decoration_style = match style.and_then(|style| style.text_decoration_style) {
                Some(diagram_ir::TreemapTextDecorationStyle::Double) => TextDecorationStyle::Double,
                Some(diagram_ir::TreemapTextDecorationStyle::Dotted) => TextDecorationStyle::Dotted,
                Some(diagram_ir::TreemapTextDecorationStyle::Dashed) => TextDecorationStyle::Dashed,
                Some(diagram_ir::TreemapTextDecorationStyle::Wavy) => TextDecorationStyle::Wavy,
                _ => TextDecorationStyle::Solid,
            };
            (lines != TextDecorationLines::NONE).then_some(TextDecoration {
                lines, style: decoration_style, color: decoration_color,
                thickness: decoration_thickness,
                underline_offset,
                underline_position,
            })
        });
    }
    if matches!(white_space, Some(diagram_ir::TreemapWhiteSpace::BreakSpaces)) {
        node.ext.insert("text.break-spaces".into(), ExtValue::Bool(true));
    }
    if matches!(white_space, Some(diagram_ir::TreemapWhiteSpace::PreserveSpaces)) {
        node.ext.insert("text.preserve-spaces".into(), ExtValue::Bool(true));
    }
    if text_indent != 0.0 {
        node.ext.insert("text.indent".into(), ExtValue::Float(text_indent));
    }
    if style.is_some_and(|style| style.text_indent_hanging) {
        node.ext.insert("text.indent-hanging".into(), ExtValue::Bool(true));
    }
    if style.is_some_and(|style| style.text_indent_each_line) {
        node.ext.insert("text.indent-each-line".into(), ExtValue::Bool(true));
    }
    if let Some(spacing) = style.and_then(|style| style.letter_spacing) {
        let spacing = match spacing {
            diagram_ir::TreemapLetterSpacing::Normal => 0.0,
            diagram_ir::TreemapLetterSpacing::Pixels(value) => value,
            diagram_ir::TreemapLetterSpacing::Factor(value) => value * font_size,
        };
        node.ext.insert("text.letter-spacing".into(), ExtValue::Float(spacing));
    }
    if let Some(spacing) = style.and_then(|style| style.word_spacing) {
        let spacing = match spacing {
            diagram_ir::TreemapWordSpacing::Normal => 0.0,
            diagram_ir::TreemapWordSpacing::Pixels(value) => value,
            diagram_ir::TreemapWordSpacing::Factor(value) => value * font_size,
        };
        node.ext.insert("text.word-spacing".into(), ExtValue::Float(spacing));
    }
    if let Some(align) = style.and_then(|style| style.text_align_last) {
        let align = match align {
            diagram_ir::TreemapTextAlignLast::Auto => "auto",
            diagram_ir::TreemapTextAlignLast::Start => "start",
            diagram_ir::TreemapTextAlignLast::Center => "center",
            diagram_ir::TreemapTextAlignLast::End => "end",
            diagram_ir::TreemapTextAlignLast::Justify => "justify",
        };
        node.ext.insert("text.align-last".into(), ExtValue::Str(align.into()));
    }
    if let Some(justify) = style.and_then(|style| style.text_justify) {
        let justify = match justify {
            diagram_ir::TreemapTextJustify::Auto => "auto",
            diagram_ir::TreemapTextJustify::None => "none",
            diagram_ir::TreemapTextJustify::InterWord => "inter-word",
            diagram_ir::TreemapTextJustify::InterCharacter => "inter-character",
        };
        node.ext.insert("text.justify-mode".into(), ExtValue::Str(justify.into()));
    }
    if let Some(wrap) = style.and_then(|style| style.overflow_wrap) {
        let wrap = match wrap {
            diagram_ir::TreemapOverflowWrap::Normal => "normal",
            diagram_ir::TreemapOverflowWrap::BreakWord => "break-word",
            diagram_ir::TreemapOverflowWrap::Anywhere => "anywhere",
        };
        node.ext.insert("text.overflow-wrap".into(), ExtValue::Str(wrap.into()));
    }
    if let Some(word_break) = style.and_then(|style| style.word_break) {
        let word_break = match word_break {
            diagram_ir::TreemapWordBreak::Normal => "normal",
            diagram_ir::TreemapWordBreak::BreakAll => "break-all",
            diagram_ir::TreemapWordBreak::KeepAll => "keep-all",
        };
        node.ext.insert("text.word-break".into(), ExtValue::Str(word_break.into()));
    }
    if let Some(line_break) = style.and_then(|style| style.line_break) {
        let line_break = match line_break {
            diagram_ir::TreemapLineBreak::Auto => "auto",
            diagram_ir::TreemapLineBreak::Loose => "loose",
            diagram_ir::TreemapLineBreak::Normal => "normal",
            diagram_ir::TreemapLineBreak::Strict => "strict",
            diagram_ir::TreemapLineBreak::Anywhere => "anywhere",
        };
        node.ext.insert("text.line-break".into(), ExtValue::Str(line_break.into()));
    }
    if let Some(hyphens) = style.and_then(|style| style.hyphens) {
        let hyphens = match hyphens {
            diagram_ir::TreemapHyphens::None => "none",
            diagram_ir::TreemapHyphens::Manual => "manual",
        };
        node.ext.insert("text.hyphens".into(), ExtValue::Str(hyphens.into()));
    }
    if let Some(character) = style.and_then(|style| style.hyphenate_character.as_ref()) {
        let character = match character {
            diagram_ir::TreemapHyphenateCharacter::Auto => "-",
            diagram_ir::TreemapHyphenateCharacter::Character(value) => value,
        };
        node.ext.insert("text.hyphenate-character".into(), ExtValue::Str(character.into()));
    }
    if let Some(text_overflow) = style.and_then(|style| style.text_overflow) {
        let text_overflow = match text_overflow {
            diagram_ir::TreemapTextOverflow::Clip => "clip",
            diagram_ir::TreemapTextOverflow::Ellipsis => "ellipsis",
        };
        node.ext.insert("text.overflow".into(), ExtValue::Str(text_overflow.into()));
    }
    if let Some(text_wrap_mode) = style.and_then(|style| style.text_wrap_mode) {
        let text_wrap_mode = match text_wrap_mode {
            diagram_ir::TreemapTextWrapMode::Wrap => "wrap",
            diagram_ir::TreemapTextWrapMode::NoWrap => "nowrap",
        };
        node.ext.insert("text.wrap-mode".into(), ExtValue::Str(text_wrap_mode.into()));
    }
    if let Some(text_wrap_style) = style.and_then(|style| style.text_wrap_style) {
        let text_wrap_style = match text_wrap_style {
            diagram_ir::TreemapTextWrapStyle::Auto => "auto",
            diagram_ir::TreemapTextWrapStyle::Balance => "balance",
            diagram_ir::TreemapTextWrapStyle::Pretty => "pretty",
            diagram_ir::TreemapTextWrapStyle::Stable => "stable",
        };
        node.ext.insert("text.wrap-style".into(), ExtValue::Str(text_wrap_style.into()));
    }
    if matches!(style.and_then(|style| style.text_align), Some(diagram_ir::TreemapTextAlign::Justify)) {
        node.ext.insert("text.justify".into(), ExtValue::Bool(true));
    }
    if let Some(direction) = style.and_then(|style| style.direction) {
        let value = match direction {
            diagram_ir::TreemapTextDirection::LeftToRight => "ltr",
            diagram_ir::TreemapTextDirection::RightToLeft => "rtl",
        };
        node.ext.insert("html".into(), ExtValue::Map(HashMap::from([
            ("dir".into(), ExtValue::Str(value.into())),
        ])));
    }
    if let Some(diagram_ir::TreemapTextShadow::Shadows(shadows)) =
        style.and_then(|style| style.text_shadow.as_ref())
    {
        let opacity = style.and_then(|style| style.opacity).unwrap_or(1.0);
        let effects = EffectStyle {
            filters: shadows.iter().map(|shadow| {
                let shadow_color = match &shadow.color {
                    diagram_ir::TreemapTextShadowColor::CurrentColor => color,
                    diagram_ir::TreemapTextShadowColor::Color(value) =>
                        color_with_opacity(css_to_color(value), opacity),
                };
                EffectFilter::DropShadow {
                    dx: shadow.offset_x,
                    dy: shadow.offset_y,
                    blur: shadow.blur_radius,
                    color: EffectColor {
                        r: shadow_color.r, g: shadow_color.g, b: shadow_color.b, a: shadow_color.a,
                    },
                }
            }).collect(),
            ..EffectStyle::default()
        };
        node.ext.insert("effects".into(), effects.to_ext());
    }
    node
}

fn font_stretch_from_percentage(value: f64) -> FontStretch {
    match value {
        value if value <= 56.25 => FontStretch::UltraCondensed,
        value if value <= 68.75 => FontStretch::ExtraCondensed,
        value if value <= 81.25 => FontStretch::Condensed,
        value if value <= 93.75 => FontStretch::SemiCondensed,
        value if value <= 106.25 => FontStretch::Normal,
        value if value <= 118.75 => FontStretch::SemiExpanded,
        value if value <= 137.5 => FontStretch::Expanded,
        value if value <= 175.0 => FontStretch::ExtraExpanded,
        _ => FontStretch::UltraExpanded,
    }
}

fn apply_treemap_line_height(font: &mut FontSpec, style: Option<&diagram_ir::TreemapStyle>) {
    font.line_height = match style.and_then(|style| style.line_height) {
        Some(diagram_ir::TreemapLineHeight::Factor(value)) => value,
        Some(diagram_ir::TreemapLineHeight::Pixels(value)) => value / font.size.max(1.0),
        None => font.line_height,
    };
}

fn apply_treemap_font_size(font: &mut FontSpec, style: Option<&diagram_ir::TreemapStyle>) {
    font.size = match style.and_then(|style| style.font_size) {
        Some(diagram_ir::TreemapFontSize::Pixels(value)) => value,
        Some(diagram_ir::TreemapFontSize::Factor(value)) => font.size * value,
        None => font.size,
    };
}

fn treemap_corner_radius(node: &diagram_ir::LayoutedTreemapNode) -> f64 {
    match node.style.as_ref().and_then(|style| style.border_radius) {
        Some(diagram_ir::TreemapBorderRadius::Pixels(value)) => value,
        Some(diagram_ir::TreemapBorderRadius::Factor(value)) => node.width.min(node.height) * value,
        None => node.style.as_ref().and_then(|style| style.node.corner_radius).unwrap_or(3.0),
    }
}

fn treemap_current_color(
    node: &diagram_ir::LayoutedTreemapNode,
    diagram: &diagram_ir::LayoutedTreemapDiagram,
    fallback: Color,
) -> String {
    node.style.as_ref().and_then(|style| style.node.text_color.as_deref())
        .filter(|value| !value.eq_ignore_ascii_case("currentcolor"))
        .or(diagram.config.label_color.as_deref())
        .filter(|value| !value.eq_ignore_ascii_case("currentcolor"))
        .map(str::to_string)
        .or_else(|| node.palette_index.map(|index| diagram.config.theme.labels[index].clone()))
        .unwrap_or_else(|| format!("rgb({}, {}, {})", fallback.r, fallback.g, fallback.b))
}

struct TreemapNumberFormat {
    fill: char, align: char, sign: char, symbol: Option<char>, width: Option<usize>, grouped: bool,
    precision: Option<usize>, trim: bool, kind: Option<char>,
}

fn parse_treemap_number_format(source: &str) -> Option<TreemapNumberFormat> {
    let characters = source.chars().collect::<Vec<_>>();
    let mut index = 0;
    let (fill, mut align) = if characters.get(1).is_some_and(|value| "<>=^".contains(*value)) {
        index = 2; (characters[0], characters[1])
    } else if characters.first().is_some_and(|value| "<>=^".contains(*value)) {
        index = 1; (' ', characters[0])
    } else { (' ', '>') };
    let sign = characters.get(index).copied().filter(|value| "+-( ".contains(*value)).map_or('-', |value| { index += 1; value });
    let symbol = characters.get(index).copied().filter(|value| matches!(value, '$' | '#'));
    if symbol.is_some() { index += 1; }
    let zero = characters.get(index) == Some(&'0');
    if zero { index += 1; align = '='; }
    let width_start = index;
    while characters.get(index).is_some_and(char::is_ascii_digit) { index += 1; }
    let width = (index > width_start).then(|| characters[width_start..index].iter().collect::<String>().parse().ok()).flatten();
    let grouped = characters.get(index) == Some(&',');
    if grouped { index += 1; }
    if grouped && characters.get(index) == Some(&'0') { index += 1; }
    let precision = if characters.get(index) == Some(&'.') {
        index += 1;
        let start = index;
        while characters.get(index).is_some_and(char::is_ascii_digit) { index += 1; }
        if start == index { return None; }
        let digits = characters[start..index].iter().collect::<String>();
        Some(if digits.chars().all(|digit| digit == '0') { digits.len() } else { digits.parse().ok()? })
    } else { None };
    let trim = characters.get(index) == Some(&'~');
    if trim { index += 1; }
    let kind = characters.get(index).copied().filter(|value| "bdeEfFgGoprs%xX".contains(*value));
    if kind.is_some() { index += 1; }
    (index == characters.len()).then_some(TreemapNumberFormat {
        fill: if zero { '0' } else { fill }, align, sign, symbol, width, grouped, precision, trim, kind,
    })
}

fn align_treemap_number(prefix: String, rendered: String, suffix: &str, spec: &TreemapNumberFormat) -> String {
    let content_width = prefix.chars().count() + rendered.chars().count() + suffix.chars().count();
    let padding = spec.width.unwrap_or(0).saturating_sub(content_width);
    let fill = spec.fill.to_string().repeat(padding);
    match spec.align {
        '<' => format!("{prefix}{rendered}{suffix}{fill}"),
        '^' => {
            let left = spec.fill.to_string().repeat(padding / 2);
            let right = spec.fill.to_string().repeat(padding - padding / 2);
            format!("{left}{prefix}{rendered}{suffix}{right}")
        }
        '=' => format!("{prefix}{fill}{rendered}{suffix}"),
        _ => format!("{fill}{prefix}{rendered}{suffix}"),
    }
}

fn format_significant(value: f64, precision: usize) -> String {
    if value == 0.0 { return "0".into(); }
    let exponent = value.abs().log10().floor() as i32;
    let decimals = (precision as i32 - exponent - 1).max(0) as usize;
    trim_decimal_zeroes(format!("{value:.decimals$}"))
}

fn format_si(value: f64, precision: usize) -> String {
    const PREFIXES: [&str; 17] = ["y", "z", "a", "f", "p", "n", "µ", "m", "", "k", "M", "G", "T", "P", "E", "Z", "Y"];
    if value == 0.0 { return "0".into(); }
    let group = ((value.abs().log10().floor() / 3.0).floor() as i32).clamp(-8, 8);
    let scaled = value / 1000_f64.powi(group);
    format!("{}{}", format_significant(scaled, precision), PREFIXES[(group + 8) as usize])
}

fn trim_decimal_zeroes(mut value: String) -> String {
    let exponent = value.find(['e', 'E']).map(|index| value.split_off(index));
    if value.contains('.') {
        while value.ends_with('0') { value.pop(); }
        if value.ends_with('.') { value.pop(); }
    }
    if let Some(exponent) = exponent { value.push_str(&exponent); }
    value
}

fn group_decimal_thousands(value: &str) -> String {
    let suffix_start = value.find(|character: char| !character.is_ascii_digit() && !matches!(character, '-' | '+' | '.'))
        .unwrap_or(value.len());
    let (number, suffix) = value.split_at(suffix_start);
    let (integer, fraction) = number.split_once('.').map_or((number, None), |(integer, fraction)| (integer, Some(fraction)));
    let sign_length = usize::from(integer.starts_with(['-', '+']));
    let (sign, digits) = integer.split_at(sign_length);
    let mut grouped = String::from(sign);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) { grouped.push(','); }
        grouped.push(digit);
    }
    if let Some(fraction) = fraction { grouped.push('.'); grouped.push_str(fraction); }
    grouped.push_str(suffix);
    grouped
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

fn capitalize_words(value: &str) -> String {
    let mut result = String::new();
    let mut at_word_start = true;
    for character in value.chars() {
        if character.is_whitespace() {
            result.push(character);
            at_word_start = true;
        } else if at_word_start {
            result.extend(character.to_uppercase());
            at_word_start = false;
        } else {
            result.push(character);
        }
    }
    result
}

fn full_size_kana(value: &str) -> String {
    value.chars().map(|character| match character {
        'ぁ' => 'あ', 'ぃ' => 'い', 'ぅ' => 'う', 'ぇ' => 'え', 'ぉ' => 'お',
        'っ' => 'つ', 'ゃ' => 'や', 'ゅ' => 'ゆ', 'ょ' => 'よ', 'ゎ' => 'わ',
        'ゕ' => 'か', 'ゖ' => 'け',
        'ァ' => 'ア', 'ィ' => 'イ', 'ゥ' => 'ウ', 'ェ' => 'エ', 'ォ' => 'オ',
        'ッ' => 'ツ', 'ャ' => 'ヤ', 'ュ' => 'ユ', 'ョ' => 'ヨ', 'ヮ' => 'ワ',
        'ヵ' => 'カ', 'ヶ' => 'ケ',
        'ㇰ' => 'ク', 'ㇱ' => 'シ', 'ㇲ' => 'ス', 'ㇳ' => 'ト', 'ㇴ' => 'ヌ',
        'ㇵ' => 'ハ', 'ㇶ' => 'ヒ', 'ㇷ' => 'フ', 'ㇸ' => 'ヘ', 'ㇹ' => 'ホ',
        'ㇺ' => 'ム', 'ㇻ' => 'ラ', 'ㇼ' => 'リ', 'ㇽ' => 'ル', 'ㇾ' => 'レ', 'ㇿ' => 'ロ',
        _ => character,
    }).collect()
}

fn with_opacity(color: &str, opacity: f64) -> String {
    let parsed = if color.eq_ignore_ascii_case("none") {
        Some(Color { r: 0, g: 0, b: 0, a: 0 })
    } else {
        parse_css_color(color)
    };
    if let Some(parsed) = parsed {
        let alpha = f64::from(parsed.a) / 255.0 * opacity.clamp(0.0, 1.0);
        return format!("rgba({},{},{},{})", parsed.r, parsed.g, parsed.b, css_alpha(alpha));
    }
    color.to_string()
}

fn css_alpha(alpha: f64) -> String {
    let value = format!("{:.6}", alpha.clamp(0.0, 1.0));
    value.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn color_with_opacity(mut color: Color, opacity: f64) -> Color {
    color.a = (f64::from(color.a) * opacity.clamp(0.0, 1.0)).round() as u8;
    color
}

fn event_model_kind_name(kind: &EventModelEntityKind) -> &'static str {
    match kind {
        EventModelEntityKind::Ui => "ui",
        EventModelEntityKind::Processor => "processor",
        EventModelEntityKind::Command => "command",
        EventModelEntityKind::ReadModel => "readmodel",
        EventModelEntityKind::Event => "event",
    }
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
            diagram.config.padding,
            diagram.config.padding + 8.0,
            diagram.width - 2.0 * diagram.config.padding,
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
            LayoutedEventModelItem::Frame { x, y, width, height, label, data_reference, data_label, kind: _, fill, stroke } => {
                let frame_metadata = data_reference.as_ref().map(|reference| {
                    HashMap::from([("eventModel.dataReference".into(), reference.clone())])
                });
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase { metadata: frame_metadata, ..PaintBase::default() },
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: Some(fill.clone()),
                    stroke: Some(stroke.clone()),
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
                    if data_label.is_some() { 24.0 } else { height - 12.0 },
                    options.label_font.clone(),
                    label_color,
                ));
                if let Some(data_label) = data_label {
                    text_children.push(text_node(
                        data_label,
                        x + 6.0,
                        y + 32.0,
                        width - 12.0,
                        height - 38.0,
                        options.label_font.clone(),
                        label_color,
                    ));
                }
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
    metadata.insert("eventModel.config.padding".into(), diagram.config.padding.to_string());
    metadata.insert("eventModel.config.rowHeight".into(), diagram.config.row_height.to_string());
    metadata.insert("eventModel.config.useMaxWidth".into(), diagram.config.use_max_width.to_string());
    for (index, entity) in diagram.entities.iter().enumerate() {
        metadata.insert(format!("eventModel.entity.{index}.id"), entity.id.clone());
        if let Some(namespace) = &entity.namespace {
            metadata.insert(format!("eventModel.entity.{index}.namespace"), namespace.clone());
        }
    }
    for (index, note) in diagram.notes.iter().enumerate() {
        metadata.insert(format!("eventModel.note.{index}.frame"), note.source_frame.clone());
        metadata.insert(format!("eventModel.note.{index}.data"), note.data.clone());
        if let Some(data_type) = &note.data_type {
            metadata.insert(format!("eventModel.note.{index}.type"), data_type.clone());
        }
    }
    for (index, gwt) in diagram.gwt.iter().enumerate() {
        metadata.insert(format!("eventModel.gwt.{index}.frame"), gwt.source_frame.clone());
        for (section, statements) in [
            ("given", &gwt.given),
            ("when", &gwt.when),
            ("then", &gwt.then),
        ] {
            for (statement_index, statement) in statements.iter().enumerate() {
                metadata.insert(
                    format!("eventModel.gwt.{index}.{section}.{statement_index}.kind"),
                    event_model_kind_name(&statement.kind).into(),
                );
                metadata.insert(
                    format!("eventModel.gwt.{index}.{section}.{statement_index}.entity"),
                    statement.entity_id.clone(),
                );
            }
        }
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
    let instruction = node_shape_geometry_instruction(node);
    if node.classes.is_empty() && node.icon.is_none() && node.label.markdown.is_none() {
        return instruction;
    }

    let mut metadata = HashMap::new();
    if !node.classes.is_empty() {
        metadata.insert("diagram.classes".into(), node.classes.join(" "));
    }
    if let Some(icon) = &node.icon {
        metadata.insert("diagram.icon".into(), icon.clone());
    }
    if let Some(markdown) = &node.label.markdown {
        metadata.insert("diagram.label.markdown".into(), markdown.clone());
    }
    PaintInstruction::Group(PaintGroup {
        base: PaintBase {
            id: Some(node.id.clone()),
            metadata: Some(metadata),
        },
        children: vec![instruction],
        transform: None,
        opacity: None,
    })
}

fn node_shape_geometry_instruction(node: &LayoutedGraphNode) -> PaintInstruction {
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
            stroke_dash: node.style.stroke_dash.clone(),
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
                stroke_dash: node.style.stroke_dash.clone(),
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
        DiagramShape::Cloud => {
            let x = node.x;
            let y = node.y;
            let w = node.width;
            let h = node.height;
            PaintInstruction::Path(PaintPath {
                base: PaintBase::default(),
                commands: vec![
                    PathCommand::MoveTo {
                        x: x + 0.18 * w,
                        y: y + 0.78 * h,
                    },
                    PathCommand::CubicTo {
                        cx1: x + 0.02 * w,
                        cy1: y + 0.78 * h,
                        cx2: x - 0.02 * w,
                        cy2: y + 0.52 * h,
                        x: x + 0.14 * w,
                        y: y + 0.46 * h,
                    },
                    PathCommand::CubicTo {
                        cx1: x + 0.10 * w,
                        cy1: y + 0.27 * h,
                        cx2: x + 0.31 * w,
                        cy2: y + 0.16 * h,
                        x: x + 0.43 * w,
                        y: y + 0.30 * h,
                    },
                    PathCommand::CubicTo {
                        cx1: x + 0.52 * w,
                        cy1: y + 0.06 * h,
                        cx2: x + 0.80 * w,
                        cy2: y + 0.10 * h,
                        x: x + 0.81 * w,
                        y: y + 0.35 * h,
                    },
                    PathCommand::CubicTo {
                        cx1: x + 1.00 * w,
                        cy1: y + 0.34 * h,
                        cx2: x + 1.04 * w,
                        cy2: y + 0.63 * h,
                        x: x + 0.87 * w,
                        y: y + 0.72 * h,
                    },
                    PathCommand::CubicTo {
                        cx1: x + 0.78 * w,
                        cy1: y + 0.91 * h,
                        cx2: x + 0.54 * w,
                        cy2: y + 0.90 * h,
                        x: x + 0.47 * w,
                        y: y + 0.78 * h,
                    },
                    PathCommand::CubicTo {
                        cx1: x + 0.38 * w,
                        cy1: y + 0.94 * h,
                        cx2: x + 0.20 * w,
                        cy2: y + 0.92 * h,
                        x: x + 0.18 * w,
                        y: y + 0.78 * h,
                    },
                    PathCommand::Close,
                ],
                fill: Some(node.style.fill.clone()),
                fill_rule: None,
                stroke: Some(node.style.stroke.clone()),
                stroke_width: Some(node.style.stroke_width),
                stroke_cap: None,
                stroke_join: Some(StrokeJoin::Round),
                stroke_dash: node.style.stroke_dash.clone(),
                stroke_dash_offset: None,
            })
        }
        DiagramShape::Bang => {
            let cx = node.x + node.width / 2.0;
            let cy = node.y + node.height / 2.0;
            let outer_x = node.width / 2.0;
            let outer_y = node.height / 2.0;
            let inner_x = outer_x * 0.68;
            let inner_y = outer_y * 0.68;
            let points = (0..16)
                .map(|index| {
                    let angle = -std::f64::consts::FRAC_PI_2
                        + index as f64 * std::f64::consts::PI / 8.0;
                    let (rx, ry) = if index % 2 == 0 {
                        (outer_x, outer_y)
                    } else {
                        (inner_x, inner_y)
                    };
                    (cx + rx * angle.cos(), cy + ry * angle.sin())
                })
                .collect::<Vec<_>>();
            polygon_node_instruction(node, &points)
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
                stroke_dash: node.style.stroke_dash.clone(), stroke_dash_offset: None,
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
                        stroke_dash: node.style.stroke_dash.clone(), stroke_dash_offset: None,
                    }),
                    PaintInstruction::Ellipse(PaintEllipse {
                        base: PaintBase::default(),
                        cx: node.x + node.width / 2.0, cy: node.y + cap,
                        rx: node.width / 2.0, ry: cap,
                        fill: Some(node.style.fill.clone()), stroke: Some(node.style.stroke.clone()),
                        stroke_width: Some(node.style.stroke_width), stroke_dash: node.style.stroke_dash.clone(), stroke_dash_offset: None,
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
        DiagramShape::Asymmetric => {
            let point = node.width.min(node.height * 2.0) * 0.16;
            polygon_node_instruction(node, &[
                (node.x, node.y),
                (node.x + point, node.y + node.height / 2.0),
                (node.x, node.y + node.height),
                (node.x + node.width, node.y + node.height),
                (node.x + node.width, node.y),
            ])
        }
        DiagramShape::BlockArrow(directions) => block_arrow_instruction(node, directions),
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
                stroke_dash: node.style.stroke_dash.clone(),
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
            stroke_dash: node.style.stroke_dash.clone(),
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
            stroke_dash: node.style.stroke_dash.clone(),
            stroke_dash_offset: None,
        }),
    }
}

fn block_arrow_instruction(
    node: &LayoutedGraphNode,
    directions: diagram_ir::BlockArrowDirections,
) -> PaintInstruction {
    let head = 12.0_f64.min(node.width / 4.0).min(node.height / 4.0);
    let left = if directions.left { head } else { 0.0 };
    let right = if directions.right { head } else { 0.0 };
    let up = if directions.up { head } else { 0.0 };
    let down = if directions.down { head } else { 0.0 };
    let cx = node.x + node.width / 2.0;
    let cy = node.y + node.height / 2.0;
    let mut points = vec![(node.x + left, node.y + up)];
    if directions.up { points.extend([(cx - head, node.y + up), (cx, node.y), (cx + head, node.y + up)]); }
    points.push((node.x + node.width - right, node.y + up));
    if directions.right {
        points.extend([(node.x + node.width - right, cy - head), (node.x + node.width, cy), (node.x + node.width - right, cy + head)]);
    }
    points.push((node.x + node.width - right, node.y + node.height - down));
    if directions.down {
        points.extend([(cx + head, node.y + node.height - down), (cx, node.y + node.height), (cx - head, node.y + node.height - down)]);
    }
    points.push((node.x + left, node.y + node.height - down));
    if directions.left { points.extend([(node.x + left, cy + head), (node.x, cy), (node.x + left, cy - head)]); }
    polygon_node_instruction(node, &points)
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
        stroke_dash: node.style.stroke_dash.clone(),
        stroke_dash_offset: None,
    })
}

fn node_rect_instruction(node: &LayoutedGraphNode) -> PaintInstruction {
    PaintInstruction::Rect(PaintRect {
        base: PaintBase::default(), x: node.x, y: node.y, width: node.width, height: node.height,
        fill: Some(node.style.fill.clone()), stroke: Some(node.style.stroke.clone()),
        stroke_width: Some(node.style.stroke_width), corner_radius: Some(0.0),
        stroke_dash: node.style.stroke_dash.clone(), stroke_dash_offset: None,
    })
}

fn node_open_path_instruction(node: &LayoutedGraphNode, commands: Vec<PathCommand>) -> PaintInstruction {
    PaintInstruction::Path(PaintPath {
        base: PaintBase::default(), commands, fill: None, fill_rule: None,
        stroke: Some(node.style.stroke.clone()), stroke_width: Some(node.style.stroke_width),
        stroke_cap: None, stroke_join: Some(StrokeJoin::Round),
        stroke_dash: node.style.stroke_dash.clone(), stroke_dash_offset: None,
    })
}

fn node_ellipse_instruction(node: &LayoutedGraphNode, inset: f64, fill: Option<String>) -> PaintInstruction {
    PaintInstruction::Ellipse(PaintEllipse {
        base: PaintBase::default(),
        cx: node.x + node.width / 2.0, cy: node.y + node.height / 2.0,
        rx: node.width / 2.0 - inset, ry: node.height / 2.0 - inset,
        fill, stroke: Some(node.style.stroke.clone()), stroke_width: Some(node.style.stroke_width),
        stroke_dash: node.style.stroke_dash.clone(), stroke_dash_offset: None,
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
fn arrowhead(edge: &LayoutedGraphEdge, at_start: bool) -> Option<PaintPath> {
    let marker = if at_start { edge.start_marker } else { edge.end_marker };
    if marker != EdgeMarker::Point || edge.points.len() < 2 {
        return None;
    }

    let (end, prev) = if at_start {
        (&edge.points[0], &edge.points[1])
    } else {
        (&edge.points[edge.points.len() - 1], &edge.points[edge.points.len() - 2])
    };

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

fn endpoint_marker(edge: &LayoutedGraphEdge, at_start: bool) -> Vec<PaintInstruction> {
    let marker = if at_start { edge.start_marker } else { edge.end_marker };
    if marker == EdgeMarker::Point {
        return arrowhead(edge, at_start).map(PaintInstruction::Path).into_iter().collect();
    }
    if marker == EdgeMarker::None || edge.points.len() < 2 {
        return Vec::new();
    }
    let tip = if at_start { &edge.points[0] } else { &edge.points[edge.points.len() - 1] };
    match marker {
        EdgeMarker::Circle => vec![PaintInstruction::Ellipse(PaintEllipse {
            base: PaintBase::default(),
            cx: tip.x,
            cy: tip.y,
            rx: 5.0,
            ry: 5.0,
            fill: Some("#ffffff".into()),
            stroke: Some(edge.style.stroke.clone()),
            stroke_width: Some(edge.style.stroke_width),
            stroke_dash: None,
            stroke_dash_offset: None,
        })],
        EdgeMarker::Cross => {
            let radius = 5.0;
            vec![
                PaintInstruction::Path(line_path(
                    &[Point { x: tip.x - radius, y: tip.y - radius }, Point { x: tip.x + radius, y: tip.y + radius }],
                    &edge.style.stroke,
                    edge.style.stroke_width,
                )),
                PaintInstruction::Path(line_path(
                    &[Point { x: tip.x - radius, y: tip.y + radius }, Point { x: tip.x + radius, y: tip.y - radius }],
                    &edge.style.stroke,
                    edge.style.stroke_width,
                )),
            ]
        }
        EdgeMarker::None | EdgeMarker::Point => Vec::new(),
    }
}

// ============================================================================
// Text bridge — PositionedNode construction
// ============================================================================

/// Convert a diagram-ir color string (CSS hex or "none") to a layout-ir Color.
/// Falls back to opaque black when the string is not a supported hex format.
fn css_to_color(css: &str) -> Color {
    parse_css_color(css).unwrap_or(Color { r: 0, g: 0, b: 0, a: 255 })
}

fn parse_css_color(css: &str) -> Option<Color> {
    if css.eq_ignore_ascii_case("transparent") {
        return Some(Color { r: 0, g: 0, b: 0, a: 0 });
    }
    if let Some(color) = parse_css_color_mix_function(css) {
        return Some(color);
    }
    if let Some(color) = parse_css_rgb_function(css) {
        return Some(color);
    }
    if let Some(color) = parse_css_hsl_function(css) {
        return Some(color);
    }
    if let Some(color) = parse_css_hwb_function(css) {
        return Some(color);
    }
    if let Some(color) = parse_css_lab_function(css) {
        return Some(color);
    }
    if let Some(color) = parse_css_lch_function(css) {
        return Some(color);
    }
    if let Some(color) = parse_css_oklab_function(css) {
        return Some(color);
    }
    if let Some(color) = parse_css_oklch_function(css) {
        return Some(color);
    }
    if let Some(color) = parse_css_color_function(css) {
        return Some(color);
    }
    if let Some(color) = parse_css_named_color(css) {
        return Some(color);
    }
    let value = css.strip_prefix('#')?;
    if !value.is_ascii() || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |source: &str| u8::from_str_radix(source, 16).ok();
    let nibble = |source: &str| byte(source).map(|value| value * 17);
    match value.len() {
        3 | 4 => Some(Color {
            r: nibble(&value[0..1])?,
            g: nibble(&value[1..2])?,
            b: nibble(&value[2..3])?,
            a: if value.len() == 4 { nibble(&value[3..4])? } else { 255 },
        }),
        6 | 8 => Some(Color {
            r: byte(&value[0..2])?,
            g: byte(&value[2..4])?,
            b: byte(&value[4..6])?,
            a: if value.len() == 8 { byte(&value[6..8])? } else { 255 },
        }),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum CssColorMixSpace {
    Srgb,
    SrgbLinear,
    DisplayP3,
    A98Rgb,
    ProPhotoRgb,
    Rec2020,
    XyzD65,
    XyzD50,
    Hsl,
    Hwb,
    Lab,
    Lch,
    Oklab,
    Oklch,
}

#[derive(Clone, Copy)]
enum CssHueInterpolation {
    Shorter,
    Longer,
    Increasing,
    Decreasing,
}

fn parse_css_color_mix_function(css: &str) -> Option<Color> {
    let source = css.trim().to_ascii_lowercase();
    let inner = source.strip_prefix("color-mix(")?.strip_suffix(')')?.trim();
    let parts = split_css_top_level_commas(inner)?;
    if parts.len() != 3 {
        return None;
    }
    let interpolation = parts[0].split_whitespace().collect::<Vec<_>>();
    let (space_name, hue_interpolation, explicit_hue_method) = match interpolation.as_slice() {
        ["in", space] => (*space, CssHueInterpolation::Shorter, false),
        ["in", space, method, "hue"] => {
            let method = match *method {
                "shorter" => CssHueInterpolation::Shorter,
                "longer" => CssHueInterpolation::Longer,
                "increasing" => CssHueInterpolation::Increasing,
                "decreasing" => CssHueInterpolation::Decreasing,
                _ => return None,
            };
            (*space, method, true)
        }
        _ => return None,
    };
    let space = match space_name {
        "srgb" => CssColorMixSpace::Srgb,
        "srgb-linear" => CssColorMixSpace::SrgbLinear,
        "display-p3" => CssColorMixSpace::DisplayP3,
        "a98-rgb" => CssColorMixSpace::A98Rgb,
        "prophoto-rgb" => CssColorMixSpace::ProPhotoRgb,
        "rec2020" => CssColorMixSpace::Rec2020,
        "xyz" | "xyz-d65" => CssColorMixSpace::XyzD65,
        "xyz-d50" => CssColorMixSpace::XyzD50,
        "hsl" => CssColorMixSpace::Hsl,
        "hwb" => CssColorMixSpace::Hwb,
        "lab" => CssColorMixSpace::Lab,
        "lch" => CssColorMixSpace::Lch,
        "oklab" => CssColorMixSpace::Oklab,
        "oklch" => CssColorMixSpace::Oklch,
        _ => return None,
    };
    let polar = matches!(space,
        CssColorMixSpace::Hsl | CssColorMixSpace::Hwb
            | CssColorMixSpace::Lch | CssColorMixSpace::Oklch);
    if explicit_hue_method && !polar {
        return None;
    }
    let (first_source, first_percentage) = parse_css_color_mix_stop(parts[1])?;
    let (second_source, second_percentage) = parse_css_color_mix_stop(parts[2])?;
    let first = parse_css_color(first_source)?;
    let second = parse_css_color(second_source)?;
    let (first_weight, second_weight, alpha_multiplier) = match (first_percentage, second_percentage) {
        (None, None) => (0.5, 0.5, 1.0),
        (Some(first), None) => (first, 1.0 - first, 1.0),
        (None, Some(second)) => (1.0 - second, second, 1.0),
        (Some(first), Some(second)) => {
            let total = first + second;
            if total <= 0.0 {
                return None;
            }
            (first / total, second / total, total.min(1.0))
        }
    };
    let first_alpha = f64::from(first.a) / 255.0;
    let second_alpha = f64::from(second.a) / 255.0;
    let mixed_alpha = first_alpha * first_weight + second_alpha * second_weight;
    let mut first_components = css_color_mix_components(first, space);
    let mut second_components = css_color_mix_components(second, space);
    match space {
        CssColorMixSpace::Hsl => {
            let powerless = (first_components[0] == 0.0, second_components[0] == 0.0);
            fixup_css_polar_hues(
                &mut first_components, &mut second_components, powerless.0, powerless.1,
                hue_interpolation,
            );
        }
        CssColorMixSpace::Hwb => {
            let powerless = (
                first_components[0] + first_components[1] >= 0.99999,
                second_components[0] + second_components[1] >= 0.99999,
            );
            fixup_css_polar_hues(
                &mut first_components, &mut second_components, powerless.0, powerless.1,
                hue_interpolation,
            );
        }
        CssColorMixSpace::Lch => {
            let powerless = (first_components[1] <= 0.02, second_components[1] <= 0.02);
            fixup_css_polar_hues(
                &mut first_components, &mut second_components, powerless.0, powerless.1,
                hue_interpolation,
            );
        }
        CssColorMixSpace::Oklch => {
            let powerless = (first_components[1] <= 0.000004, second_components[1] <= 0.000004);
            fixup_css_polar_hues(
                &mut first_components, &mut second_components, powerless.0, powerless.1,
                hue_interpolation,
            );
        }
        _ => {}
    }
    let component = |index: usize| {
        if matches!(
            space,
            CssColorMixSpace::Hsl | CssColorMixSpace::Hwb
                | CssColorMixSpace::Lch | CssColorMixSpace::Oklch
        )
            && index == 2
        {
            first_components[index] * first_weight + second_components[index] * second_weight
        } else if mixed_alpha == 0.0 {
            0.0
        } else {
            (first_components[index] * first_alpha * first_weight
                + second_components[index] * second_alpha * second_weight) / mixed_alpha
        }
    };
    let components = [component(0), component(1), component(2)];
    let alpha = (mixed_alpha * alpha_multiplier * 255.0).clamp(0.0, 255.0).round() as u8;
    Some(match space {
        CssColorMixSpace::Srgb => Color {
            r: (components[0].clamp(0.0, 1.0) * 255.0).round() as u8,
            g: (components[1].clamp(0.0, 1.0) * 255.0).round() as u8,
            b: (components[2].clamp(0.0, 1.0) * 255.0).round() as u8,
            a: alpha,
        },
        CssColorMixSpace::SrgbLinear => linear_srgb_to_color(
            components[0], components[1], components[2], alpha,
        ),
        CssColorMixSpace::DisplayP3 => css_display_p3_to_color(components, alpha),
        CssColorMixSpace::A98Rgb => css_a98_rgb_to_color(components, alpha),
        CssColorMixSpace::ProPhotoRgb => css_prophoto_rgb_to_color(components, alpha),
        CssColorMixSpace::Rec2020 => css_rec2020_to_color(components, alpha),
        CssColorMixSpace::XyzD65 => xyz_d65_to_color(
            components[0], components[1], components[2], alpha,
        ),
        CssColorMixSpace::XyzD50 => xyz_d50_to_color(
            components[0], components[1], components[2], alpha,
        ),
        CssColorMixSpace::Hsl => css_hsl_to_color(
            components[2].to_degrees(), components[0], components[1], alpha,
        ),
        CssColorMixSpace::Hwb => css_hwb_to_color(
            components[2].to_degrees(), components[0], components[1], alpha,
        ),
        CssColorMixSpace::Lab => css_lab_to_color(
            components[0], components[1], components[2], alpha,
        ),
        CssColorMixSpace::Lch => css_lab_to_color(
            components[0],
            components[1] * components[2].cos(),
            components[1] * components[2].sin(),
            alpha,
        ),
        CssColorMixSpace::Oklab => css_oklab_to_color(
            components[0], components[1], components[2], alpha,
        ),
        CssColorMixSpace::Oklch => css_oklab_to_color(
            components[0],
            components[1] * components[2].cos(),
            components[1] * components[2].sin(),
            alpha,
        ),
    })
}

fn css_color_mix_components(color: Color, space: CssColorMixSpace) -> [f64; 3] {
    let encoded = [
        f64::from(color.r) / 255.0,
        f64::from(color.g) / 255.0,
        f64::from(color.b) / 255.0,
    ];
    match space {
        CssColorMixSpace::Srgb => encoded,
        CssColorMixSpace::SrgbLinear => encoded.map(encoded_srgb_to_linear),
        CssColorMixSpace::DisplayP3 => css_color_to_display_p3(encoded),
        CssColorMixSpace::A98Rgb => css_color_to_a98_rgb(encoded),
        CssColorMixSpace::ProPhotoRgb => css_color_to_prophoto_rgb(encoded),
        CssColorMixSpace::Rec2020 => css_color_to_rec2020(encoded),
        CssColorMixSpace::XyzD65 => css_color_to_xyz_d65(encoded),
        CssColorMixSpace::XyzD50 => {
            let [x, y, z] = css_color_to_xyz_d65(encoded);
            xyz_d65_to_d50(x, y, z)
        }
        CssColorMixSpace::Hsl => css_color_to_hsl(encoded),
        CssColorMixSpace::Hwb => css_color_to_hwb(encoded),
        CssColorMixSpace::Lab => css_color_to_lab(encoded),
        CssColorMixSpace::Lch => {
            let [lightness, a, b] = css_color_to_lab(encoded);
            [lightness, a.hypot(b), b.atan2(a)]
        }
        CssColorMixSpace::Oklab => css_color_to_oklab(encoded),
        CssColorMixSpace::Oklch => {
            let [lightness, a, b] = css_color_to_oklab(encoded);
            [lightness, a.hypot(b), b.atan2(a)]
        }
    }
}

fn css_color_to_hsl([r, g, b]: [f64; 3]) -> [f64; 3] {
    let maximum = r.max(g).max(b);
    let minimum = r.min(g).min(b);
    let delta = maximum - minimum;
    let lightness = (maximum + minimum) / 2.0;
    let saturation = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * lightness - 1.0).abs())
    };
    let hue = if delta == 0.0 {
        0.0
    } else if maximum == r {
        ((g - b) / delta).rem_euclid(6.0) * 60.0
    } else if maximum == g {
        ((b - r) / delta + 2.0) * 60.0
    } else {
        ((r - g) / delta + 4.0) * 60.0
    };
    [saturation, lightness, hue.to_radians()]
}

fn css_color_to_hwb(encoded: [f64; 3]) -> [f64; 3] {
    let hue = css_color_to_hsl(encoded)[2];
    let whiteness = encoded[0].min(encoded[1]).min(encoded[2]);
    let blackness = 1.0 - encoded[0].max(encoded[1]).max(encoded[2]);
    [whiteness, blackness, hue]
}

fn css_color_to_lab(encoded: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = encoded.map(encoded_srgb_to_linear);
    let x65 = 0.4124 * r + 0.3576 * g + 0.1805 * b;
    let y65 = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let z65 = 0.0193 * r + 0.1192 * g + 0.9505 * b;
    let x50 = 1.0479298 * x65 + 0.0229468 * y65 - 0.0501922 * z65;
    let y50 = 0.0296278 * x65 + 0.9904345 * y65 - 0.0170738 * z65;
    let z50 = -0.0092430 * x65 + 0.0150552 * y65 + 0.7518743 * z65;
    let transform = |value: f64| {
        if value > 216.0 / 24_389.0 {
            value.cbrt()
        } else {
            ((24_389.0 / 27.0) * value + 16.0) / 116.0
        }
    };
    let f0 = transform(x50 / 0.96422);
    let f1 = transform(y50);
    let f2 = transform(z50 / 0.82521);
    [116.0 * f1 - 16.0, 500.0 * (f0 - f1), 200.0 * (f1 - f2)]
}

fn css_color_to_oklab(encoded: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = encoded.map(encoded_srgb_to_linear);
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}

fn fixup_css_polar_hues(
    first: &mut [f64; 3],
    second: &mut [f64; 3],
    first_powerless: bool,
    second_powerless: bool,
    interpolation: CssHueInterpolation,
) {
    if first_powerless {
        first[2] = second[2];
    }
    if second_powerless {
        second[2] = first[2];
    }
    let turn = std::f64::consts::TAU;
    match interpolation {
        CssHueInterpolation::Shorter => {
            let delta = (second[2] - first[2] + std::f64::consts::PI).rem_euclid(turn)
                - std::f64::consts::PI;
            second[2] = first[2] + delta;
        }
        CssHueInterpolation::Longer => {
            let delta = second[2] - first[2];
            if delta > 0.0 && delta < std::f64::consts::PI {
                first[2] += turn;
            } else if delta < 0.0 && delta > -std::f64::consts::PI {
                second[2] += turn;
            }
        }
        CssHueInterpolation::Increasing => {
            if second[2] < first[2] {
                second[2] += turn;
            }
        }
        CssHueInterpolation::Decreasing => {
            if first[2] < second[2] {
                first[2] += turn;
            }
        }
    }
}

fn split_css_top_level_commas(source: &str) -> Option<Vec<&str>> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0_u32;
    for (index, character) in source.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.checked_sub(1)?,
            ',' if depth == 0 => {
                parts.push(source[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    parts.push(source[start..].trim());
    Some(parts)
}

fn parse_css_color_mix_stop(source: &str) -> Option<(&str, Option<f64>)> {
    let mut depth = 0_u32;
    let mut split = None;
    for (index, character) in source.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.checked_sub(1)?,
            _ if depth == 0 && character.is_whitespace() => split = Some(index),
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    if let Some(index) = split {
        let percentage = source[index..].trim();
        if let Some(value) = percentage.strip_suffix('%').and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
        {
            return Some((source[..index].trim(), Some(value / 100.0)));
        }
    }
    Some((source.trim(), None))
}

fn parse_css_rgb_function(css: &str) -> Option<Color> {
    let source = css.trim().to_ascii_lowercase();
    let inner = source.strip_prefix("rgb(").or_else(|| source.strip_prefix("rgba("))?
        .strip_suffix(')')?.trim();
    let normalized = inner.replace(',', " ").replace('/', " / ");
    let parts = normalized.split_whitespace().collect::<Vec<_>>();
    let slash = parts.iter().position(|part| *part == "/");
    let legacy_alpha = slash.is_none() && parts.len() == 4;
    let color_parts = slash.map_or_else(
        || if legacy_alpha { &parts[..3] } else { &parts[..] },
        |index| &parts[..index],
    );
    if color_parts.len() != 3 {
        return None;
    }
    let component = |value: &str| parse_css_byte(value, 1.0);
    let alpha_source = slash.and_then(|index| parts.get(index + 1).copied())
        .or_else(|| legacy_alpha.then(|| parts[3]));
    let alpha = alpha_source.map_or(Some(255), |value| parse_css_byte(value, 255.0))?;
    Some(Color {
        r: component(color_parts[0])?,
        g: component(color_parts[1])?,
        b: component(color_parts[2])?,
        a: alpha,
    })
}

fn parse_css_hsl_function(css: &str) -> Option<Color> {
    let source = css.trim().to_ascii_lowercase();
    let inner = source.strip_prefix("hsl(").or_else(|| source.strip_prefix("hsla("))?
        .strip_suffix(')')?.trim();
    let normalized = inner.replace(',', " ").replace('/', " / ");
    let parts = normalized.split_whitespace().collect::<Vec<_>>();
    let slash = parts.iter().position(|part| *part == "/");
    let legacy_alpha = slash.is_none() && parts.len() == 4;
    let color_parts = slash.map_or_else(
        || if legacy_alpha { &parts[..3] } else { &parts[..] },
        |index| &parts[..index],
    );
    if color_parts.len() != 3 {
        return None;
    }
    let hue = parse_css_hue(color_parts[0])?;
    let saturation = parse_css_percentage(color_parts[1])?;
    let lightness = parse_css_percentage(color_parts[2])?;
    let alpha_source = slash.and_then(|index| parts.get(index + 1).copied())
        .or_else(|| legacy_alpha.then(|| parts[3]));
    let alpha = alpha_source.map_or(Some(255), |value| parse_css_byte(value, 255.0))?;
    Some(css_hsl_to_color(hue, saturation, lightness, alpha))
}

fn css_hsl_to_color(hue: f64, saturation: f64, lightness: f64, alpha: u8) -> Color {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue.rem_euclid(360.0) / 60.0;
    let x = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match sector as u8 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma / 2.0;
    let channel = |value: f64| ((value + m) * 255.0).clamp(0.0, 255.0).round() as u8;
    Color { r: channel(r), g: channel(g), b: channel(b), a: alpha }
}

fn parse_css_hwb_function(css: &str) -> Option<Color> {
    let source = css.trim().to_ascii_lowercase();
    let inner = source.strip_prefix("hwb(")?.strip_suffix(')')?.trim();
    let normalized = inner.replace('/', " / ");
    let parts = normalized.split_whitespace().collect::<Vec<_>>();
    let slash = parts.iter().position(|part| *part == "/");
    let color_parts = slash.map_or(&parts[..], |index| &parts[..index]);
    if color_parts.len() != 3 {
        return None;
    }
    let hue = parse_css_hue(color_parts[0])?;
    let whiteness = parse_css_percentage(color_parts[1])?;
    let blackness = parse_css_percentage(color_parts[2])?;
    let alpha = slash.and_then(|index| parts.get(index + 1).copied())
        .map_or(Some(255), |value| parse_css_byte(value, 255.0))?;
    Some(css_hwb_to_color(hue, whiteness, blackness, alpha))
}

fn css_hwb_to_color(hue: f64, mut whiteness: f64, mut blackness: f64, alpha: u8) -> Color {
    let sum = whiteness + blackness;
    if sum > 1.0 {
        whiteness /= sum;
        blackness /= sum;
    }
    let sector = hue.rem_euclid(360.0) / 60.0;
    let x = 1.0 - (sector.rem_euclid(2.0) - 1.0).abs();
    let (r, g, b) = match sector as u8 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    let factor = 1.0 - whiteness - blackness;
    let channel = |value: f64| ((value * factor + whiteness) * 255.0).clamp(0.0, 255.0).round() as u8;
    Color { r: channel(r), g: channel(g), b: channel(b), a: alpha }
}

fn parse_css_lab_function(css: &str) -> Option<Color> {
    let source = css.trim().to_ascii_lowercase();
    let inner = source.strip_prefix("lab(")?.strip_suffix(')')?.trim();
    let (components, alpha) = parse_css_modern_color_components(inner)?;
    let lightness = parse_css_lab_lightness(&components[0])?;
    let a = parse_css_lab_axis(&components[1])?;
    let b = parse_css_lab_axis(&components[2])?;
    Some(css_lab_to_color(lightness, a, b, alpha))
}

fn parse_css_lch_function(css: &str) -> Option<Color> {
    let source = css.trim().to_ascii_lowercase();
    let inner = source.strip_prefix("lch(")?.strip_suffix(')')?.trim();
    let (components, alpha) = parse_css_modern_color_components(inner)?;
    let lightness = parse_css_lab_lightness(&components[0])?;
    let chroma = parse_css_lch_chroma(&components[1])?;
    let hue = parse_css_hue(&components[2])?.to_radians();
    Some(css_lab_to_color(lightness, chroma * hue.cos(), chroma * hue.sin(), alpha))
}

fn parse_css_oklab_function(css: &str) -> Option<Color> {
    let source = css.trim().to_ascii_lowercase();
    let inner = source.strip_prefix("oklab(")?.strip_suffix(')')?.trim();
    let (components, alpha) = parse_css_modern_color_components(inner)?;
    let lightness = parse_css_oklab_lightness(&components[0])?;
    let a = parse_css_oklab_axis(&components[1])?;
    let b = parse_css_oklab_axis(&components[2])?;
    Some(css_oklab_to_color(lightness, a, b, alpha))
}

fn parse_css_oklch_function(css: &str) -> Option<Color> {
    let source = css.trim().to_ascii_lowercase();
    let inner = source.strip_prefix("oklch(")?.strip_suffix(')')?.trim();
    let (components, alpha) = parse_css_modern_color_components(inner)?;
    let lightness = parse_css_oklab_lightness(&components[0])?;
    let chroma = parse_css_oklch_chroma(&components[1])?;
    let hue = parse_css_hue(&components[2])?.to_radians();
    Some(css_oklab_to_color(lightness, chroma * hue.cos(), chroma * hue.sin(), alpha))
}

fn parse_css_color_function(css: &str) -> Option<Color> {
    let source = css.trim().to_ascii_lowercase();
    let inner = source.strip_prefix("color(")?.strip_suffix(')')?.trim();
    let profile_end = inner.find(char::is_whitespace)?;
    let (profile, components) = inner.split_at(profile_end);
    let (components, alpha) = parse_css_modern_color_components(components.trim())?;
    let r = parse_css_unit_interval(&components[0])?;
    let g = parse_css_unit_interval(&components[1])?;
    let b = parse_css_unit_interval(&components[2])?;
    match profile {
        "srgb" => Some(Color {
            r: (r * 255.0).round() as u8,
            g: (g * 255.0).round() as u8,
            b: (b * 255.0).round() as u8,
            a: alpha,
        }),
        "srgb-linear" => Some(linear_srgb_to_color(r, g, b, alpha)),
        "display-p3" => {
            Some(css_display_p3_to_color([r, g, b], alpha))
        }
        "a98-rgb" => {
            Some(css_a98_rgb_to_color([r, g, b], alpha))
        }
        "prophoto-rgb" => {
            Some(css_prophoto_rgb_to_color([r, g, b], alpha))
        }
        "rec2020" => {
            Some(css_rec2020_to_color([r, g, b], alpha))
        }
        "xyz" | "xyz-d65" => Some(xyz_d65_to_color(r, g, b, alpha)),
        "xyz-d50" => Some(xyz_d50_to_color(r, g, b, alpha)),
        _ => None,
    }
}

fn parse_css_modern_color_components(inner: &str) -> Option<([String; 3], u8)> {
    let normalized = inner.replace('/', " / ");
    let parts = normalized.split_whitespace().collect::<Vec<_>>();
    let slash = parts.iter().position(|part| *part == "/");
    let color_parts = slash.map_or(&parts[..], |index| &parts[..index]);
    let components = color_parts.iter().map(|part| (*part).to_owned()).collect::<Vec<_>>()
        .try_into().ok()?;
    let alpha = slash.and_then(|index| parts.get(index + 1).copied())
        .map_or(Some(255), |value| parse_css_byte(value, 255.0))?;
    if slash.is_some_and(|index| index + 2 != parts.len()) {
        return None;
    }
    Some((components, alpha))
}

fn parse_css_lab_lightness(value: &str) -> Option<f64> {
    if value == "none" {
        return Some(0.0);
    }
    let lightness = value.strip_suffix('%').unwrap_or(value).parse::<f64>().ok()?;
    lightness.is_finite().then(|| lightness.clamp(0.0, 100.0))
}

fn parse_css_lab_axis(value: &str) -> Option<f64> {
    if value == "none" {
        return Some(0.0);
    }
    let (number, scale) = value.strip_suffix('%').map_or((value, 1.0), |value| (value, 1.25));
    let axis = number.parse::<f64>().ok()? * scale;
    axis.is_finite().then_some(axis)
}

fn parse_css_lch_chroma(value: &str) -> Option<f64> {
    if value == "none" {
        return Some(0.0);
    }
    let (number, scale) = value.strip_suffix('%').map_or((value, 1.0), |value| (value, 1.5));
    let chroma = number.parse::<f64>().ok()? * scale;
    chroma.is_finite().then(|| chroma.max(0.0))
}

fn parse_css_oklab_lightness(value: &str) -> Option<f64> {
    if value == "none" {
        return Some(0.0);
    }
    let (number, scale) = value.strip_suffix('%').map_or((value, 1.0), |value| (value, 0.01));
    let lightness = number.parse::<f64>().ok()? * scale;
    lightness.is_finite().then(|| lightness.clamp(0.0, 1.0))
}

fn parse_css_oklab_axis(value: &str) -> Option<f64> {
    if value == "none" {
        return Some(0.0);
    }
    let (number, scale) = value.strip_suffix('%').map_or((value, 1.0), |value| (value, 0.004));
    let axis = number.parse::<f64>().ok()? * scale;
    axis.is_finite().then_some(axis)
}

fn parse_css_oklch_chroma(value: &str) -> Option<f64> {
    let chroma = parse_css_oklab_axis(value)?;
    Some(chroma.max(0.0))
}

fn parse_css_unit_interval(value: &str) -> Option<f64> {
    if value == "none" {
        return Some(0.0);
    }
    let (number, scale) = value.strip_suffix('%').map_or((value, 1.0), |value| (value, 0.01));
    let component = number.parse::<f64>().ok()? * scale;
    component.is_finite().then(|| component.clamp(0.0, 1.0))
}

fn css_lab_to_color(lightness: f64, a: f64, b: f64, alpha: u8) -> Color {
    let f1 = (lightness + 16.0) / 116.0;
    let f0 = a / 500.0 + f1;
    let f2 = f1 - b / 200.0;
    let to_xyz = |value: f64| {
        let cube = value.powi(3);
        if cube > 216.0 / 24_389.0 { cube } else { (116.0 * value - 16.0) / (24_389.0 / 27.0) }
    };
    let x50 = to_xyz(f0) * 0.96422;
    let y50 = to_xyz(f1);
    let z50 = to_xyz(f2) * 0.82521;
    xyz_d50_to_color(x50, y50, z50, alpha)
}

fn css_oklab_to_color(lightness: f64, a: f64, b: f64, alpha: u8) -> Color {
    let l = (lightness + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m = (lightness - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s = (lightness - 0.0894841775 * a - 1.2914855480 * b).powi(3);
    linear_srgb_to_color(
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
        alpha,
    )
}

fn linear_srgb_to_color(r: f64, g: f64, b: f64, alpha: u8) -> Color {
    Color {
        r: (linear_srgb_to_encoded(r).clamp(0.0, 1.0) * 255.0).round() as u8,
        g: (linear_srgb_to_encoded(g).clamp(0.0, 1.0) * 255.0).round() as u8,
        b: (linear_srgb_to_encoded(b).clamp(0.0, 1.0) * 255.0).round() as u8,
        a: alpha,
    }
}

fn linear_srgb_to_encoded(linear: f64) -> f64 {
    if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

fn encoded_srgb_to_linear(value: f64) -> f64 {
    if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
}

fn css_display_p3_to_color([r, g, b]: [f64; 3], alpha: u8) -> Color {
    let r = encoded_srgb_to_linear(r);
    let g = encoded_srgb_to_linear(g);
    let b = encoded_srgb_to_linear(b);
    let x = 0.4865709486482162 * r + 0.2656676931690931 * g + 0.1982172852343625 * b;
    let y = 0.2289745640697488 * r + 0.6917385218365064 * g + 0.0792869140937450 * b;
    let z = 0.0451133818589026 * g + 1.043_944_368_900_976 * b;
    linear_srgb_to_color(
        3.2409699419045226 * x - 1.537383177570094 * y - 0.4986107602930034 * z,
        -0.9692436362808796 * x + 1.8759675015077202 * y + 0.04155505740717559 * z,
        0.05563007969699366 * x - 0.20397695888897652 * y + 1.0569715142428786 * z,
        alpha,
    )
}

fn css_color_to_display_p3(encoded: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = encoded.map(encoded_srgb_to_linear);
    let x = 0.4123907992659595 * r + 0.357_584_339_383_878 * g + 0.1804807884018343 * b;
    let y = 0.2126390058715104 * r + 0.715_168_678_767_756 * g + 0.0721923153607337 * b;
    let z = 0.0193308187155918 * r + 0.119_194_779_794_626 * g + 0.9505321522496607 * b;
    let linear = [
        2.493496911941425 * x - 0.9313836179191239 * y - 0.40271078445071684 * z,
        -0.8294889695615747 * x + 1.7626640603183463 * y + 0.023624685841943577 * z,
        0.03584583024378447 * x - 0.07617238926804182 * y + 0.9568845240076872 * z,
    ];
    linear.map(|value| {
        let encoded = linear_srgb_to_encoded(value);
        if encoded.abs() < 1e-12 {
            0.0
        } else if (encoded - 1.0).abs() < 1e-12 {
            1.0
        } else {
            encoded
        }
    })
}

fn css_a98_rgb_to_color([r, g, b]: [f64; 3], alpha: u8) -> Color {
    let decode = |value: f64| value.signum() * value.abs().powf(563.0 / 256.0);
    let r = decode(r);
    let g = decode(g);
    let b = decode(b);
    xyz_d65_to_color(
        (573536.0 / 994567.0) * r + (263643.0 / 1420810.0) * g + (187206.0 / 994567.0) * b,
        (591459.0 / 1989134.0) * r + (6239551.0 / 9945670.0) * g + (374412.0 / 4972835.0) * b,
        (53769.0 / 1989134.0) * r + (351524.0 / 4972835.0) * g + (4929758.0 / 4972835.0) * b,
        alpha,
    )
}

fn css_color_to_a98_rgb(encoded: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = encoded.map(encoded_srgb_to_linear);
    let x = 0.4123907992659595 * r + 0.357_584_339_383_878 * g + 0.1804807884018343 * b;
    let y = 0.2126390058715104 * r + 0.715_168_678_767_756 * g + 0.0721923153607337 * b;
    let z = 0.0193308187155918 * r + 0.119_194_779_794_626 * g + 0.9505321522496607 * b;
    let linear = [
        (1829569.0 / 896150.0) * x - (506331.0 / 896150.0) * y - (308931.0 / 896150.0) * z,
        -(851781.0 / 878810.0) * x + (1648619.0 / 878810.0) * y + (36519.0 / 878810.0) * z,
        (16779.0 / 1248040.0) * x - (147721.0 / 1248040.0) * y + (1266979.0 / 1248040.0) * z,
    ];
    linear.map(|value| {
        let encoded = value.signum() * value.abs().powf(256.0 / 563.0);
        if encoded.abs() < 1e-12 {
            0.0
        } else if (encoded - 1.0).abs() < 1e-12 {
            1.0
        } else {
            encoded
        }
    })
}

fn css_prophoto_rgb_to_color([r, g, b]: [f64; 3], alpha: u8) -> Color {
    let decode = |value: f64| {
        if value.abs() <= 16.0 / 512.0 {
            value / 16.0
        } else {
            value.signum() * value.abs().powf(1.8)
        }
    };
    let r = decode(r);
    let g = decode(g);
    let b = decode(b);
    xyz_d50_to_color(
        0.7977666449006423 * r + 0.1351812974005331 * g + 0.0313477341283922 * b,
        0.2880748288194013 * r + 0.711_835_234_241_873 * g + 0.0000899369387256 * b,
        0.8251046025104601 * b,
        alpha,
    )
}

fn css_color_to_prophoto_rgb(encoded: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = encoded.map(encoded_srgb_to_linear);
    let x65 = 0.4123907992659595 * r + 0.357_584_339_383_878 * g + 0.1804807884018343 * b;
    let y65 = 0.2126390058715104 * r + 0.715_168_678_767_756 * g + 0.0721923153607337 * b;
    let z65 = 0.0193308187155918 * r + 0.119_194_779_794_626 * g + 0.9505321522496607 * b;
    let x = 1.0479298 * x65 + 0.0229468 * y65 - 0.0501922 * z65;
    let y = 0.0296278 * x65 + 0.9904345 * y65 - 0.0170738 * z65;
    let z = -0.0092430 * x65 + 0.0150552 * y65 + 0.7518743 * z65;
    let linear = [
        1.3457868816471583 * x - 0.25557208737979464 * y - 0.05110186497554526 * z,
        -0.5446307051249019 * x + 1.5082477428451468 * y + 0.02052744743642139 * z,
        1.2119675456389452 * z,
    ];
    linear.map(|value| {
        let encoded = if value.abs() >= 1.0 / 512.0 {
            value.signum() * value.abs().powf(1.0 / 1.8)
        } else {
            16.0 * value
        };
        if encoded.abs() < 1e-12 {
            0.0
        } else if (encoded - 1.0).abs() < 1e-12 {
            1.0
        } else {
            encoded
        }
    })
}

fn css_rec2020_to_color([r, g, b]: [f64; 3], alpha: u8) -> Color {
    let transfer_alpha = 1.09929682680944;
    let beta = 0.018053968510807;
    let decode = |value: f64| {
        if value.abs() < beta * 4.5 {
            value / 4.5
        } else {
            value.signum()
                * ((value.abs() + transfer_alpha - 1.0) / transfer_alpha).powf(1.0 / 0.45)
        }
    };
    let r = decode(r);
    let g = decode(g);
    let b = decode(b);
    xyz_d65_to_color(
        (63426534.0 / 99577255.0) * r + (20160776.0 / 139408157.0) * g + (47086771.0 / 278816314.0) * b,
        (26158966.0 / 99577255.0) * r + (472592308.0 / 697040785.0) * g + (8267143.0 / 139408157.0) * b,
        (19567812.0 / 697040785.0) * g + (295819943.0 / 278816314.0) * b,
        alpha,
    )
}

fn css_color_to_rec2020(encoded: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = encoded.map(encoded_srgb_to_linear);
    let x = 0.4123907992659595 * r + 0.357_584_339_383_878 * g + 0.1804807884018343 * b;
    let y = 0.2126390058715104 * r + 0.715_168_678_767_756 * g + 0.0721923153607337 * b;
    let z = 0.0193308187155918 * r + 0.119_194_779_794_626 * g + 0.9505321522496607 * b;
    let linear = [
        (30757411.0 / 17917100.0) * x - (6372589.0 / 17917100.0) * y - (4539589.0 / 17917100.0) * z,
        -(19765991.0 / 29648200.0) * x + (47925759.0 / 29648200.0) * y + (467509.0 / 29648200.0) * z,
        (792561.0 / 44930125.0) * x - (1921689.0 / 44930125.0) * y + (42328811.0 / 44930125.0) * z,
    ];
    let transfer_alpha = 1.09929682680944;
    let beta = 0.018053968510807;
    linear.map(|value| {
        let encoded = if value.abs() > beta {
            value.signum() * (transfer_alpha * value.abs().powf(0.45) - (transfer_alpha - 1.0))
        } else {
            4.5 * value
        };
        if encoded.abs() < 1e-12 {
            0.0
        } else if (encoded - 1.0).abs() < 1e-12 {
            1.0
        } else {
            encoded
        }
    })
}

fn css_color_to_xyz_d65(encoded: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = encoded.map(encoded_srgb_to_linear);
    [
        0.4123907992659595 * r + 0.357_584_339_383_878 * g + 0.1804807884018343 * b,
        0.2126390058715104 * r + 0.715_168_678_767_756 * g + 0.0721923153607337 * b,
        0.0193308187155918 * r + 0.119_194_779_794_626 * g + 0.9505321522496607 * b,
    ]
}

fn xyz_d65_to_d50(x: f64, y: f64, z: f64) -> [f64; 3] {
    [
        1.0479298 * x + 0.0229468 * y - 0.0501922 * z,
        0.0296278 * x + 0.9904345 * y - 0.0170738 * z,
        -0.0092430 * x + 0.0150552 * y + 0.7518743 * z,
    ]
}

fn xyz_d65_to_color(x: f64, y: f64, z: f64, alpha: u8) -> Color {
    linear_srgb_to_color(
        3.2406 * x - 1.5372 * y - 0.4986 * z,
        -0.9689 * x + 1.8758 * y + 0.0415 * z,
        0.0557 * x - 0.2040 * y + 1.0570 * z,
        alpha,
    )
}

fn xyz_d50_to_color(x: f64, y: f64, z: f64, alpha: u8) -> Color {
    xyz_d65_to_color(
        0.9554734 * x - 0.0230985 * y + 0.0632593 * z,
        -0.0283697 * x + 1.0099955 * y + 0.0210414 * z,
        0.0123140 * x - 0.0205077 * y + 1.3303659 * z,
        alpha,
    )
}

fn parse_css_hue(value: &str) -> Option<f64> {
    if value == "none" {
        return Some(0.0);
    }
    let (number, scale) = if let Some(value) = value.strip_suffix("deg") {
        (value, 1.0)
    } else if let Some(value) = value.strip_suffix("grad") {
        (value, 0.9)
    } else if let Some(value) = value.strip_suffix("rad") {
        (value, 180.0 / std::f64::consts::PI)
    } else if let Some(value) = value.strip_suffix("turn") {
        (value, 360.0)
    } else {
        (value, 1.0)
    };
    number.parse::<f64>().ok().filter(|value| value.is_finite())
        .map(|value| (value * scale).rem_euclid(360.0))
}

fn parse_css_percentage(value: &str) -> Option<f64> {
    if value == "none" {
        return Some(0.0);
    }
    value.strip_suffix('%')?.parse::<f64>().ok().filter(|value| value.is_finite())
        .map(|value| (value / 100.0).clamp(0.0, 1.0))
}

fn parse_css_byte(value: &str, numeric_scale: f64) -> Option<u8> {
    if value == "none" {
        return Some(0);
    }
    let (number, percentage) = value.strip_suffix('%').map_or((value, false), |value| (value, true));
    number.parse::<f64>().ok().filter(|value| value.is_finite()).map(|value| {
        let scaled = if percentage { value / 100.0 * 255.0 } else { value * numeric_scale };
        scaled.clamp(0.0, 255.0).round() as u8
    })
}

fn parse_css_named_color(css: &str) -> Option<Color> {
    let (r, g, b) = match css.trim().to_ascii_lowercase().as_str() {
        "aqua" => (0, 255, 255),
        "black" => (0, 0, 0),
        "blue" => (0, 0, 255),
        "fuchsia" => (255, 0, 255),
        "gray" | "grey" => (128, 128, 128),
        "green" => (0, 128, 0),
        "lime" => (0, 255, 0),
        "maroon" => (128, 0, 0),
        "navy" => (0, 0, 128),
        "olive" => (128, 128, 0),
        "orange" => (255, 165, 0),
        "purple" => (128, 0, 128),
        "red" => (255, 0, 0),
        "silver" => (192, 192, 192),
        "teal" => (0, 128, 128),
        "white" => (255, 255, 255),
        "yellow" => (255, 255, 0),
        "aliceblue" => (240, 248, 255),
        "antiquewhite" => (250, 235, 215),
        "aquamarine" => (127, 255, 212),
        "azure" => (240, 255, 255),
        "beige" => (245, 245, 220),
        "bisque" => (255, 228, 196),
        "blanchedalmond" => (255, 235, 205),
        "blueviolet" => (138, 43, 226),
        "brown" => (165, 42, 42),
        "burlywood" => (222, 184, 135),
        "cadetblue" => (95, 158, 160),
        "chartreuse" => (127, 255, 0),
        "chocolate" => (210, 105, 30),
        "coral" => (255, 127, 80),
        "cornflowerblue" => (100, 149, 237),
        "cornsilk" => (255, 248, 220),
        "crimson" => (220, 20, 60),
        "cyan" => (0, 255, 255),
        "darkblue" => (0, 0, 139),
        "darkcyan" => (0, 139, 139),
        "darkgoldenrod" => (184, 134, 11),
        "darkgray" | "darkgrey" => (169, 169, 169),
        "darkgreen" => (0, 100, 0),
        "darkkhaki" => (189, 183, 107),
        "darkmagenta" => (139, 0, 139),
        "darkolivegreen" => (85, 107, 47),
        "darkorange" => (255, 140, 0),
        "darkorchid" => (153, 50, 204),
        "darkred" => (139, 0, 0),
        "darksalmon" => (233, 150, 122),
        "darkseagreen" => (143, 188, 143),
        "darkslateblue" => (72, 61, 139),
        "darkslategray" | "darkslategrey" => (47, 79, 79),
        "darkturquoise" => (0, 206, 209),
        "darkviolet" => (148, 0, 211),
        "deeppink" => (255, 20, 147),
        "deepskyblue" => (0, 191, 255),
        "dimgray" | "dimgrey" => (105, 105, 105),
        "dodgerblue" => (30, 144, 255),
        "firebrick" => (178, 34, 34),
        "floralwhite" => (255, 250, 240),
        "forestgreen" => (34, 139, 34),
        "gainsboro" => (220, 220, 220),
        "ghostwhite" => (248, 248, 255),
        "gold" => (255, 215, 0),
        "goldenrod" => (218, 165, 32),
        "greenyellow" => (173, 255, 47),
        "honeydew" => (240, 255, 240),
        "hotpink" => (255, 105, 180),
        "indianred" => (205, 92, 92),
        "indigo" => (75, 0, 130),
        "ivory" => (255, 255, 240),
        "khaki" => (240, 230, 140),
        "lavender" => (230, 230, 250),
        "lavenderblush" => (255, 240, 245),
        "lawngreen" => (124, 252, 0),
        "lemonchiffon" => (255, 250, 205),
        "lightblue" => (173, 216, 230),
        "lightcoral" => (240, 128, 128),
        "lightcyan" => (224, 255, 255),
        "lightgoldenrodyellow" => (250, 250, 210),
        "lightgray" | "lightgrey" => (211, 211, 211),
        "lightgreen" => (144, 238, 144),
        "lightpink" => (255, 182, 193),
        "lightsalmon" => (255, 160, 122),
        "lightseagreen" => (32, 178, 170),
        "lightskyblue" => (135, 206, 250),
        "lightslategray" | "lightslategrey" => (119, 136, 153),
        "lightsteelblue" => (176, 196, 222),
        "lightyellow" => (255, 255, 224),
        "limegreen" => (50, 205, 50),
        "linen" => (250, 240, 230),
        "magenta" => (255, 0, 255),
        "mediumaquamarine" => (102, 205, 170),
        "mediumblue" => (0, 0, 205),
        "mediumorchid" => (186, 85, 211),
        "mediumpurple" => (147, 112, 219),
        "mediumseagreen" => (60, 179, 113),
        "mediumslateblue" => (123, 104, 238),
        "mediumspringgreen" => (0, 250, 154),
        "mediumturquoise" => (72, 209, 204),
        "mediumvioletred" => (199, 21, 133),
        "midnightblue" => (25, 25, 112),
        "mintcream" => (245, 255, 250),
        "mistyrose" => (255, 228, 225),
        "moccasin" => (255, 228, 181),
        "navajowhite" => (255, 222, 173),
        "oldlace" => (253, 245, 230),
        "olivedrab" => (107, 142, 35),
        "orangered" => (255, 69, 0),
        "orchid" => (218, 112, 214),
        "palegoldenrod" => (238, 232, 170),
        "palegreen" => (152, 251, 152),
        "paleturquoise" => (175, 238, 238),
        "palevioletred" => (219, 112, 147),
        "papayawhip" => (255, 239, 213),
        "peachpuff" => (255, 218, 185),
        "peru" => (205, 133, 63),
        "pink" => (255, 192, 203),
        "plum" => (221, 160, 221),
        "powderblue" => (176, 224, 230),
        "rebeccapurple" => (102, 51, 153),
        "rosybrown" => (188, 143, 143),
        "royalblue" => (65, 105, 225),
        "saddlebrown" => (139, 69, 19),
        "salmon" => (250, 128, 114),
        "sandybrown" => (244, 164, 96),
        "seagreen" => (46, 139, 87),
        "seashell" => (255, 245, 238),
        "sienna" => (160, 82, 45),
        "skyblue" => (135, 206, 235),
        "slateblue" => (106, 90, 205),
        "slategray" | "slategrey" => (112, 128, 144),
        "snow" => (255, 250, 250),
        "springgreen" => (0, 255, 127),
        "steelblue" => (70, 130, 180),
        "tan" => (210, 180, 140),
        "thistle" => (216, 191, 216),
        "tomato" => (255, 99, 71),
        "turquoise" => (64, 224, 208),
        "violet" => (238, 130, 238),
        "wheat" => (245, 222, 179),
        "whitesmoke" => (245, 245, 245),
        "yellowgreen" => (154, 205, 50),
        _ => return None,
    };
    Some(Color { r, g, b, a: 255 })
}

fn normalize_css_paint(css: String) -> String {
    if css.eq_ignore_ascii_case("none") {
        return "rgba(0,0,0,0)".into();
    }
    if css.starts_with('#') {
        return css;
    }
    parse_css_color(&css).map_or(css, |color| {
        if color.a == 255 {
            format!("rgb({},{},{})", color.r, color.g, color.b)
        } else {
            format!("rgba({},{},{},{})", color.r, color.g, color.b, css_alpha(f64::from(color.a) / 255.0))
        }
    })
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

fn align_text_node(
    mut node: PositionedNode,
    text_align: TextAlign,
) -> PositionedNode {
    if let Some(Content::Text(text)) = &mut node.content {
        text.text_align = text_align;
    }
    node
}

struct MarkdownLabelBox {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn markdown_label_instructions<S, M, R>(
    label: &DiagramLabel,
    bounds: MarkdownLabelBox,
    font: &FontSpec,
    color: &str,
    options: &DiagramToPaintOptions<'_, S, M, R>,
) -> Vec<PaintInstruction>
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let mut lines = vec![Vec::<(String, bool, bool)>::new()];
    for span in &label.spans {
        for (index, part) in span.text.split('\n').enumerate() {
            if index > 0 {
                lines.push(Vec::new());
            }
            if !part.is_empty() {
                lines
                    .last_mut()
                    .expect("rich label always has a line")
                    .push((part.to_string(), span.bold, span.italic));
            }
        }
    }

    let size = font.size as f32;
    let line_height = font.size * 1.2;
    let text_height = lines.len().max(1) as f64 * line_height;
    let top = bounds.y + (bounds.height - text_height) / 2.0;
    let mut output = Vec::new();

    for (line_index, line) in lines.into_iter().enumerate() {
        let mut shaped_chunks = Vec::new();
        let mut line_advance = 0.0;
        let mut ascent = font.size * 0.8;
        for (text, bold, italic) in line {
            let query = FontQuery::named(font.family.clone())
                .with_weight(FontWeight(if bold {
                    font.weight.max(700)
                } else {
                    font.weight
                }))
                .with_style(if italic || font.italic {
                    FontStyle::Italic
                } else {
                    FontStyle::Normal
                });
            let Ok(handle) = options.resolver.resolve(&query) else {
                continue;
            };
            let units_per_em = options.metrics.units_per_em(&handle).max(1) as f64;
            ascent = ascent.max(
                options.metrics.ascent(&handle) as f64 * font.size / units_per_em,
            );
            let Ok(shaped) = options
                .shaper
                .shape(&text, &handle, size, &ShapeOptions::default())
            else {
                continue;
            };
            line_advance += shaped.total_advance() as f64;
            shaped_chunks.push(shaped);
        }

        let baseline_y = top + line_index as f64 * line_height + ascent;
        let mut pen_x = bounds.x + (bounds.width - line_advance) / 2.0;
        for shaped in shaped_chunks {
            for run in shaped.runs {
                let mut segment_pen = 0.0;
                let glyphs = run
                    .glyphs
                    .iter()
                    .map(|glyph| {
                        let position = GlyphPosition {
                            glyph_id: glyph.glyph_id,
                            x: pen_x + segment_pen + glyph.x_offset as f64,
                            y: baseline_y + glyph.y_offset as f64,
                        };
                        segment_pen += glyph.x_advance as f64;
                        position
                    })
                    .collect();
                output.push(PaintInstruction::GlyphRun(PaintGlyphRun {
                    base: PaintBase::default(),
                    glyphs,
                    font_ref: run.font_ref,
                    font_size: font.size,
                    fill: Some(color.to_string()),
                }));
                pen_x += run.x_advance_total as f64;
            }
        }
    }
    output
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
            stroke_dash: group.style.stroke_dash.clone(),
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
        path.stroke_dash.clone_from(&edge.style.stroke_dash);
        if edge.kind == EdgeKind::NoteAssociation {
            path.stroke_dash = Some(vec![4.0, 4.0]);
        }
        instructions.push(PaintInstruction::Path(path));
        instructions.extend(endpoint_marker(edge, true));
        instructions.extend(endpoint_marker(edge, false));
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
        let icon_width = node.icon_glyph.as_ref().map(|_| node.style.font_size * 1.5).unwrap_or(0.0);
        if let Some(icon) = &node.icon_glyph {
            let icon_height = node.style.font_size * 1.2;
            text_children.push(text_node_no_wrap(
                &icon.text,
                node.x + node.style.font_size * 0.25,
                node.y + (node.height - icon_height) / 2.0,
                icon_width,
                icon_height,
                {
                    let mut font = label_font.clone();
                    font.size = node.style.font_size * 1.1;
                    font.family.clone_from(&icon.font_family);
                    font
                },
                css_to_color(&node.style.text_color),
            ));
        }
        if !node.label.spans.is_empty() {
            let icon_width = node.icon_glyph.as_ref().map(|_| node.style.font_size * 1.5).unwrap_or(0.0);
            let font = FontSpec {
                family: node.style.font_family.clone(),
                size: node.style.font_size,
                weight: node.style.font_weight,
                italic: node.style.font_italic,
                ..label_font.clone()
            };
            instructions.extend(markdown_label_instructions(
                &node.label,
                MarkdownLabelBox {
                    x: node.x + icon_width,
                    y: node.y,
                    width: node.width - icon_width,
                    height: node.height,
                },
                &font,
                &node.style.text_color,
                options,
            ));
            continue;
        }
        let line_count = node.label.text.lines().count().max(1) as f64;
        let text_height = line_count * node.style.font_size * 1.2;
        text_children.push(text_node_no_wrap(
            &node.label.text,
            node.x + icon_width,
            node.y + (node.height - text_height) / 2.0,
            node.width - icon_width,
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
            base: kanban_paint_base(&column.id, &column.classes, column.ticket_url.as_deref()), x: column.x, y: column.y,
            width: column.width, height: column.height,
            fill: Some(column.style.fill.clone()), stroke: Some(column.style.stroke.clone()),
            stroke_width: Some(column.style.stroke_width),
            corner_radius: Some(column.style.corner_radius),
            stroke_dash: None, stroke_dash_offset: None,
        }));
        let mut heading_font = options.title_font.clone();
        heading_font.size = 16.0;
        if column.label.spans.is_empty() {
            text_children.push(text_node_no_wrap(
                &column.label.text, column.x + 12.0, column.y + 14.0,
                column.width - 24.0, 22.0, heading_font,
                css_to_color(&column.style.text_color),
            ));
        } else {
            instructions.extend(markdown_label_instructions(
                &column.label,
                MarkdownLabelBox {
                    x: column.x + 12.0,
                    y: column.y + 14.0,
                    width: column.width - 24.0,
                    height: column.header_height - 28.0,
                },
                &heading_font,
                &column.style.text_color,
                options,
            ));
        }
        for card in &column.cards {
            let has_footer = card.ticket.is_some() || card.assigned.is_some();
            instructions.push(PaintInstruction::Rect(PaintRect {
                base: kanban_paint_base(&card.id, &card.classes, card.ticket_url.as_deref()), x: card.x, y: card.y,
                width: card.width, height: card.height,
                fill: Some(card.style.fill.clone()), stroke: Some(card.style.stroke.clone()),
                stroke_width: Some(card.style.stroke_width),
                corner_radius: Some(card.style.corner_radius),
                stroke_dash: None, stroke_dash_offset: None,
            }));
            if let Some(marker) = kanban_priority_marker(card) {
                instructions.push(marker);
            }
            let (label_x, label_width) = if let Some(icon) = &card.icon {
                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x: card.x + 8.0,
                    y: card.y + 12.0,
                    width: 52.0,
                    height: 22.0,
                    fill: None,
                    stroke: Some(card.style.stroke.clone()),
                    stroke_width: Some(1.0),
                    corner_radius: Some(11.0),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
                let mut icon_font = options.label_font.clone();
                icon_font.size = 9.0;
                text_children.push(text_node_no_wrap(
                    icon,
                    card.x + 11.0,
                    card.y + 15.0,
                    46.0,
                    16.0,
                    icon_font,
                    css_to_color(&card.style.text_color),
                ));
                (card.x + 66.0, card.width - 76.0)
            } else {
                (card.x + 10.0, card.width - 20.0)
            };
            let label_height = card.height - if has_footer { 46.0 } else { 24.0 };
            if card.label.spans.is_empty() {
                text_children.push(text_node(
                    &card.label.text, label_x, card.y + 18.0,
                    label_width, label_height,
                    options.label_font.clone(), css_to_color(&card.style.text_color),
                ));
            } else {
                instructions.extend(markdown_label_instructions(
                    &card.label,
                    MarkdownLabelBox {
                        x: label_x,
                        y: card.y + 18.0,
                        width: label_width,
                        height: label_height,
                    },
                    &options.label_font,
                    &card.style.text_color,
                    options,
                ));
            }
            if has_footer {
                let mut metadata_font = options.label_font.clone();
                metadata_font.size = 11.0;
                if let Some(ticket) = &card.ticket {
                    text_children.push(align_text_node(
                        text_node_no_wrap(
                            ticket,
                            card.x + 10.0,
                            card.y + card.height - 24.0,
                            card.width / 2.0 - 10.0,
                            16.0,
                            metadata_font.clone(),
                            css_to_color(&card.style.text_color),
                        ),
                        TextAlign::Start,
                    ));
                }
                if let Some(assigned) = &card.assigned {
                    text_children.push(align_text_node(
                        text_node_no_wrap(
                            assigned,
                            card.x + card.width / 2.0,
                            card.y + card.height - 24.0,
                            card.width / 2.0 - 10.0,
                            16.0,
                            metadata_font,
                            css_to_color(&card.style.text_color),
                        ),
                        TextAlign::End,
                    ));
                }
            }
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

fn kanban_priority_marker(card: &LayoutedBoardCard) -> Option<PaintInstruction> {
    let color = kanban_priority_color(card.priority.as_deref()?)?;
    let inset = card.style.corner_radius / 2.0;
    Some(PaintInstruction::Path(PaintPath {
        base: PaintBase::default(),
        commands: vec![
            PathCommand::MoveTo { x: card.x + 2.0, y: card.y + inset },
            PathCommand::LineTo { x: card.x + 2.0, y: card.y + card.height - inset },
        ],
        fill: None,
        fill_rule: None,
        stroke: Some(color.into()),
        stroke_width: Some(4.0),
        stroke_cap: Some(StrokeCap::Round),
        stroke_join: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    }))
}

fn kanban_priority_color(priority: &str) -> Option<&'static str> {
    match priority.to_ascii_lowercase().as_str() {
        "very high" => "#ff0000",
        "high" => "#ffa500",
        "low" => "#0000ff",
        "very low" => "#add8e6",
        _ => return None,
    }
    .into()
}

fn kanban_paint_base(id: &str, classes: &[String], ticket_url: Option<&str>) -> PaintBase {
    let mut metadata = HashMap::new();
    if !classes.is_empty() {
        metadata.insert("diagram.classes".into(), classes.join(" "));
    }
    if let Some(ticket_url) = ticket_url {
        metadata.insert("diagram.link.url".into(), ticket_url.into());
        metadata.insert("diagram.link.target".into(), "_blank".into());
    }
    PaintBase { id: Some(id.into()), metadata: (!metadata.is_empty()).then_some(metadata) }
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
            LayoutedChartItem::CubicPath {
                start,
                segments,
                color,
                fill,
                fill_opacity,
                stroke_width,
            } => {
                let mut commands = Vec::with_capacity(segments.len() + 2);
                commands.push(PathCommand::MoveTo {
                    x: start.x,
                    y: start.y,
                });
                commands.extend(segments.iter().map(|segment| PathCommand::CubicTo {
                    cx1: segment.control1.x,
                    cy1: segment.control1.y,
                    cx2: segment.control2.x,
                    cy2: segment.control2.y,
                    x: segment.end.x,
                    y: segment.end.y,
                }));
                commands.push(PathCommand::Close);
                instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands,
                    fill: fill.as_ref().map(|fill| {
                        fill_opacity.map_or_else(|| fill.clone(), |opacity| with_opacity(fill, opacity))
                    }),
                    fill_rule: None,
                    stroke: Some(color.clone()),
                    stroke_width: Some(*stroke_width),
                    stroke_cap: Some(StrokeCap::Round),
                    stroke_join: Some(StrokeJoin::Round),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }
            LayoutedChartItem::FilledLinePath {
                points,
                fill,
                fill_opacity,
                stroke,
                stroke_width,
            } => {
                if let Some(first) = points.first() {
                    let mut commands = Vec::with_capacity(points.len() + 1);
                    commands.push(PathCommand::MoveTo {
                        x: first.x,
                        y: first.y,
                    });
                    commands.extend(points[1..].iter().map(|point| PathCommand::LineTo {
                        x: point.x,
                        y: point.y,
                    }));
                    commands.push(PathCommand::Close);
                    instructions.push(PaintInstruction::Path(PaintPath {
                        base: PaintBase::default(),
                        commands,
                        fill: Some(with_opacity(fill, *fill_opacity)),
                        fill_rule: None,
                        stroke: Some(stroke.clone()),
                        stroke_width: Some(*stroke_width),
                        stroke_cap: Some(StrokeCap::Round),
                        stroke_join: Some(StrokeJoin::Round),
                        stroke_dash: None,
                        stroke_dash_offset: None,
                    }));
                }
            }
            LayoutedChartItem::StyledLine {
                x1,
                y1,
                x2,
                y2,
                color,
                stroke_width,
            } => {
                instructions.push(PaintInstruction::Path(line_path(
                    &[Point { x: *x1, y: *y1 }, Point { x: *x2, y: *y2 }],
                    color,
                    *stroke_width,
                )));
            }
            LayoutedChartItem::AnchoredLabel {
                x,
                y,
                text,
                font_size,
                color,
                anchor,
                baseline,
            } => {
                let width = diagram.width.min(240.0);
                let label_x = match anchor {
                    ChartTextAnchor::Start => *x,
                    ChartTextAnchor::Middle => x - width / 2.0,
                    ChartTextAnchor::End => x - width,
                };
                let label_y = match baseline {
                    ChartTextBaseline::Top => *y,
                    ChartTextBaseline::Middle => y - font_size * 0.6,
                    ChartTextBaseline::Bottom => y - font_size * 0.8,
                };
                let mut node = text_node_no_wrap(
                    text,
                    label_x,
                    label_y,
                    width,
                    font_size * 1.2,
                    font_with_size(&lf, Some(*font_size)),
                    css_to_color(color),
                );
                if let Some(Content::Text(content)) = &mut node.content {
                    content.text_align = match anchor {
                        ChartTextAnchor::Start => TextAlign::Start,
                        ChartTextAnchor::Middle => TextAlign::Center,
                        ChartTextAnchor::End => TextAlign::End,
                    };
                }
                text_children.push(node);
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
            LayoutedChartItem::VerticalLegend {
                x,
                y,
                entries,
                box_size,
                font_size,
                line_height,
                fill_opacity,
            } => {
                for (index, entry) in entries.iter().enumerate() {
                    let entry_y = y + index as f64 * line_height;
                    instructions.push(PaintInstruction::Rect(PaintRect {
                        base: PaintBase::default(),
                        x: *x,
                        y: entry_y,
                        width: *box_size,
                        height: *box_size,
                        fill: Some(with_opacity(&entry.color, *fill_opacity)),
                        stroke: Some(entry.color.clone()),
                        stroke_width: Some(1.0),
                        corner_radius: None,
                        stroke_dash: None,
                        stroke_dash_offset: None,
                    }));
                    let mut label = text_node_no_wrap(
                        &entry.label,
                        x + 16.0,
                        entry_y,
                        120.0,
                        font_size * 1.2,
                        font_with_size(&lf, Some(*font_size)),
                        Color {
                            r: 51,
                            g: 51,
                            b: 51,
                            a: 255,
                        },
                    );
                    if let Some(Content::Text(content)) = &mut label.content {
                        content.text_align = TextAlign::Start;
                    }
                    text_children.push(label);
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
fn push_architecture_icon(instructions: &mut Vec<PaintInstruction>, name: &str, x: f64, y: f64, size: f64) {
    let white_rect = |x, y, width, height| PaintInstruction::Rect(PaintRect {
        base: PaintBase::default(), x, y, width, height, fill: Some("#ffffff".into()),
        stroke: None, stroke_width: None, corner_radius: Some(1.0), stroke_dash: None,
        stroke_dash_offset: None,
    });
    match name {
        "database" => {
            instructions.push(white_rect(x + size * 0.24, y + size * 0.3, size * 0.52, size * 0.42));
            for cy in [0.3, 0.51, 0.72] {
                instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                    base: PaintBase::default(), cx: x + size * 0.5, cy: y + size * cy,
                    rx: size * 0.26, ry: size * 0.1, fill: Some("#ffffff".into()),
                    stroke: Some("#087ebf".into()), stroke_width: Some(1.0), stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }
        }
        "server" => {
            for offset in [0.25, 0.46, 0.67] {
                instructions.push(white_rect(x + size * 0.2, y + size * offset, size * 0.6, size * 0.13));
            }
        }
        "disk" => {
            instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                base: PaintBase::default(), cx: x + size * 0.5, cy: y + size * 0.5,
                rx: size * 0.29, ry: size * 0.29, fill: Some("#ffffff".into()),
                stroke: None, stroke_width: None, stroke_dash: None, stroke_dash_offset: None,
            }));
            instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                base: PaintBase::default(), cx: x + size * 0.5, cy: y + size * 0.5,
                rx: size * 0.08, ry: size * 0.08, fill: Some("#087ebf".into()),
                stroke: None, stroke_width: None, stroke_dash: None, stroke_dash_offset: None,
            }));
        }
        "cloud" => {
            for (cx, cy, rx, ry) in [(0.36, 0.56, 0.2, 0.16), (0.53, 0.43, 0.23, 0.23), (0.68, 0.57, 0.2, 0.16)] {
                instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                    base: PaintBase::default(), cx: x + size * cx, cy: y + size * cy,
                    rx: size * rx, ry: size * ry, fill: Some("#ffffff".into()), stroke: None,
                    stroke_width: None, stroke_dash: None, stroke_dash_offset: None,
                }));
            }
        }
        "internet" => {
            instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                base: PaintBase::default(), cx: x + size * 0.5, cy: y + size * 0.5,
                rx: size * 0.28, ry: size * 0.28, fill: None, stroke: Some("#ffffff".into()),
                stroke_width: Some(2.0), stroke_dash: None, stroke_dash_offset: None,
            }));
            instructions.push(white_rect(x + size * 0.47, y + size * 0.24, size * 0.06, size * 0.52));
            instructions.push(white_rect(x + size * 0.24, y + size * 0.47, size * 0.52, size * 0.06));
        }
        _ => {
            instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                base: PaintBase::default(), cx: x + size * 0.5, cy: y + size * 0.5,
                rx: size * 0.26, ry: size * 0.26, fill: None, stroke: Some("#ffffff".into()),
                stroke_width: Some(2.0), stroke_dash: None, stroke_dash_offset: None,
            }));
            instructions.push(white_rect(x + size * 0.44, y + size * 0.32, size * 0.12, size * 0.36));
        }
    }
}

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

    if let Some(title) = &diagram.title {
        text_children.push(text_node(
            title,
            8.0,
            6.0,
            diagram.width - 16.0,
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
        let (label_x, label_width) = if let Some(icon_name) = &group.icon_name {
            instructions.push(PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: group.x + 10.0,
                y: group.y + 6.0,
                width: 28.0,
                height: 20.0,
                fill: Some("#087ebf".into()),
                stroke: None,
                stroke_width: None,
                corner_radius: Some(2.0),
                stroke_dash: None,
                stroke_dash_offset: None,
            }));
            push_architecture_icon(&mut instructions, icon_name, group.x + 10.0, group.y + 2.0, 28.0);
            (group.x + 46.0, group.width - 56.0)
        } else {
            (group.x + 10.0, group.width - 20.0)
        };
        text_children.push(text_node(
            &label,
            label_x,
            group.y + 6.0,
            label_width,
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
        if rel.end_arrow && rel.points.len() >= 2 {
            let tip = &rel.points[rel.points.len() - 1];
            let prev = &rel.points[rel.points.len() - 2];
            instructions.push(PaintInstruction::Path(structural_arrowhead(
                prev, tip, &rel.kind,
            )));
        }
        if rel.start_arrow && rel.points.len() >= 2 {
            instructions.push(PaintInstruction::Path(structural_arrowhead(
                &rel.points[1],
                &rel.points[0],
                &rel.kind,
            )));
        }
        if let Some((ref pos, ref lbl)) = rel.label {
            instructions.push(PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: pos.x - 44.0,
                y: pos.y - ls * 0.7,
                width: 88.0,
                height: ls * 1.4,
                fill: Some("#ffffff".into()),
                stroke: None,
                stroke_width: None,
                corner_radius: Some(3.0),
                stroke_dash: None,
                stroke_dash_offset: None,
            }));
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
        if node.node_kind == StructuralNodeKind::Junction {
            instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                base: PaintBase::default(),
                cx: node.x + node.width / 2.0,
                cy: node.y + node.height / 2.0,
                rx: node.width / 2.0,
                ry: node.height / 2.0,
                fill: Some("#334155".into()),
                stroke: Some("#0f172a".into()),
                stroke_width: Some(1.5),
                stroke_dash: None,
                stroke_dash_offset: None,
            }));
            continue;
        }
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
        let badge_text = node
            .icon_text
            .as_deref()
            .or(node.icon_name.as_deref());
        let (header_x, header_width) = if let Some(icon_text) = badge_text {
            let icon_size = (header_height - 16.0).min(48.0);
            instructions.push(PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: node.x + 8.0,
                y: node.y + (header_height - icon_size) / 2.0,
                width: icon_size,
                height: icon_size,
                fill: Some("#087ebf".into()),
                stroke: None,
                stroke_width: None,
                corner_radius: Some(4.0),
                stroke_dash: None,
                stroke_dash_offset: None,
            }));
            if node.icon_text.is_some() {
                text_children.push(text_node(
                    icon_text,
                    node.x + 10.0,
                    node.y + (header_height - icon_size) / 2.0 + 4.0,
                    icon_size - 4.0,
                    icon_size - 8.0,
                    node_font.clone(),
                    Color { r: 255, g: 255, b: 255, a: 255 },
                ));
            } else if let Some(icon_name) = &node.icon_name {
                push_architecture_icon(
                    &mut instructions,
                    icon_name,
                    node.x + 8.0,
                    node.y + (header_height - icon_size) / 2.0,
                    icon_size,
                );
            }
            (node.x + icon_size + 16.0, node.width - icon_size - 24.0)
        } else {
            (node.x, node.width)
        };
        // Header text (with optional stereotype)
        let header_label = if let Some(ref st) = node.stereotype {
            format!("«{}»\n{}", st, node.header)
        } else {
            node.header.clone()
        };
        text_children.push(text_node(
            &header_label,
            header_x,
            node.y + 8.0,
            header_width,
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
                stretch: FontStretch::Normal,
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
            stroke_dash: None,
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
                    classes: Vec::new(),
                    icon: None,
                    icon_glyph: None,
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
                    classes: Vec::new(),
                    icon: None,
                    icon_glyph: None,
                },
            ],
            edges: vec![LayoutedGraphEdge {
                id: None,
                from_node_id: "A".to_string(),
                to_node_id: "B".to_string(),
                kind: EdgeKind::Directed,
                start_marker: EdgeMarker::None,
                end_marker: EdgeMarker::Point,
                points: vec![Point { x: 120.0, y: 50.0 }, Point { x: 216.0, y: 50.0 }],
                label: None,
                label_position: None,
                style: edge_style(),
            }],
        }
    }

    #[test]
    fn version_exists() {
        assert_eq!(crate::VERSION, "0.75.0");
    }

    #[test]
    fn kanban_priorities_resolve_to_mermaid_marker_colors() {
        assert_eq!(kanban_priority_color("Very High"), Some("#ff0000"));
        assert_eq!(kanban_priority_color("high"), Some("#ffa500"));
        assert_eq!(kanban_priority_color("Medium"), None);
        assert_eq!(kanban_priority_color("Low"), Some("#0000ff"));
        assert_eq!(kanban_priority_color("Very Low"), Some("#add8e6"));
    }

    #[test]
    fn kanban_markdown_labels_lower_to_backend_neutral_glyph_runs() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let rich_label = |text: &str, source: &str, spans| {
            DiagramLabel::markdown(text, source, spans)
        };
        let layout = LayoutedBoardDiagram {
            width: 300.0,
            height: 180.0,
            columns: vec![diagram_ir::LayoutedBoardColumn {
                id: "todo".into(),
                label: rich_label(
                    "Todo queue",
                    "**Todo** queue",
                    vec![
                        diagram_ir::DiagramTextSpan { text: "Todo".into(), bold: true, italic: false },
                        diagram_ir::DiagramTextSpan { text: " queue".into(), bold: false, italic: false },
                    ],
                ),
                x: 20.0,
                y: 20.0,
                width: 260.0,
                height: 140.0,
                header_height: 52.0,
                cards: vec![diagram_ir::LayoutedBoardCard {
                    id: "card".into(),
                    label: rich_label(
                        "Quoted card",
                        "Quoted *card*",
                        vec![
                            diagram_ir::DiagramTextSpan { text: "Quoted ".into(), bold: false, italic: false },
                            diagram_ir::DiagramTextSpan { text: "card".into(), bold: false, italic: true },
                        ],
                    ),
                    x: 32.0,
                    y: 84.0,
                    width: 236.0,
                    height: 72.0,
                    style: default_style(),
                    ticket: None,
                    ticket_url: None,
                    assigned: None,
                    priority: None,
                    icon: None,
                    classes: Vec::new(),
                }],
                style: default_style(),
                ticket: None,
                ticket_url: None,
                classes: Vec::new(),
            }],
        };

        let scene = diagram_to_paint_board(&layout, &opts);
        let runs = scene.instructions.iter().filter_map(|instruction| match instruction {
            PaintInstruction::GlyphRun(run) => Some(run),
            _ => None,
        }).collect::<Vec<_>>();
        assert_eq!(runs.len(), 4);
        assert!(runs.iter().all(|run| run.glyphs.iter().all(|glyph| {
            glyph.glyph_id != '*' as u32 && glyph.glyph_id != '`' as u32
        })));
    }

    #[test]
    fn architecture_icon_text_lowers_to_backend_neutral_badge_and_glyphs() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedStructuralDiagram {
            width: 200.0,
            height: 100.0,
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            groups: vec![],
            nodes: vec![diagram_ir::LayoutedStructuralNode {
                id: "api".into(),
                node_kind: StructuralNodeKind::Element,
                x: 20.0,
                y: 12.0,
                width: 160.0,
                height: 72.0,
                header: "Gateway".into(),
                stereotype: None,
                icon_name: None,
                icon_text: Some("API".into()),
                style: default_style(),
                compartments: vec![],
            }],
            relationships: vec![],
        };

        let scene = diagram_to_paint_structural(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Rect(rect) if rect.fill.as_deref() == Some("#087ebf")
        )));
        assert!(
            scene
                .instructions
                .iter()
                .filter(|instruction| matches!(instruction, PaintInstruction::GlyphRun(_)))
                .count()
                >= 2
        );
    }

    #[test]
    fn architecture_named_icons_lower_to_backend_neutral_geometry() {
        let mut instructions = Vec::new();
        push_architecture_icon(&mut instructions, "database", 0.0, 0.0, 48.0);
        assert!(instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::Ellipse(_))));
        assert!(instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::Rect(_))));
        let known_count = instructions.len();
        push_architecture_icon(&mut instructions, "aws:lambda", 0.0, 0.0, 48.0);
        assert!(instructions.len() > known_count);
    }

    #[test]
    fn structural_bidirectional_relationship_lowers_both_arrowheads() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedStructuralDiagram {
            width: 200.0,
            height: 80.0,
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            groups: vec![],
            nodes: vec![],
            relationships: vec![diagram_ir::LayoutedStructuralRelationship {
                from_id: "api".into(),
                to_id: "db".into(),
                kind: RelKind::Dependency,
                start_arrow: true,
                end_arrow: true,
                from_group: false,
                to_group: false,
                from_port: None,
                to_port: None,
                routing: diagram_ir::StructuralRouting::Direct,
                points: vec![Point { x: 20.0, y: 40.0 }, Point { x: 180.0, y: 40.0 }],
                from_mult: None,
                to_mult: None,
                label: None,
            }],
        };

        let scene = diagram_to_paint_structural(&layout, &opts);
        assert_eq!(
            scene
                .instructions
                .iter()
                .filter(|instruction| matches!(
                    instruction,
                    PaintInstruction::Path(path) if path.fill.as_deref() == Some("#6b7280")
                ))
                .count(),
            2
        );
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
    fn bidirectional_edge_produces_two_arrowhead_paths() {
        let mut layout = simple_layout();
        layout.edges[0].kind = EdgeKind::Bidirectional;
        layout.edges[0].start_marker = EdgeMarker::Point;
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&layout, &opts);
        let paths = scene.instructions.iter().filter(|instruction| {
            matches!(instruction, PaintInstruction::Path(_))
        }).count();
        assert_eq!(paths, 3, "bidirectional edge: polyline plus two arrowheads");
    }

    #[test]
    fn circle_and_cross_edge_markers_lower_to_backend_neutral_geometry() {
        let mut layout = simple_layout();
        layout.edges[0].kind = EdgeKind::Undirected;
        layout.edges[0].start_marker = EdgeMarker::Circle;
        layout.edges[0].end_marker = EdgeMarker::Cross;
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&layout, &opts);
        assert_eq!(scene.instructions.iter().filter(|instruction| {
            matches!(instruction, PaintInstruction::Ellipse(_))
        }).count(), 1);
        assert_eq!(scene.instructions.iter().filter(|instruction| {
            matches!(instruction, PaintInstruction::Path(_))
        }).count(), 3);
    }

    #[test]
    fn undirected_edge_has_no_arrowhead() {
        let mut layout = simple_layout();
        layout.edges[0].kind = EdgeKind::Undirected;
        layout.edges[0].end_marker = EdgeMarker::None;
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
    fn polygonal_and_stadium_shapes_lower_to_backend_neutral_paint() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);

        for (shape, command_count) in [
            (DiagramShape::Hexagon, 7),
            (DiagramShape::Bang, 17),
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

        let mut layout = simple_layout();
        layout.nodes[0].shape = DiagramShape::Cloud;
        let scene = diagram_to_paint(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| {
            matches!(instruction, PaintInstruction::Path(path)
                if path.commands.len() == 8
                    && path.commands.iter().filter(|command| matches!(command, PathCommand::CubicTo { .. })).count() == 6)
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
    fn block_arrow_shape_lowers_to_backend_neutral_polygon() {
        let mut layout = simple_layout();
        layout.nodes[0].shape = DiagramShape::BlockArrow(diagram_ir::BlockArrowDirections {
            right: true,
            ..diagram_ir::BlockArrowDirections::default()
        });
        let instruction = node_shape_instruction(&layout.nodes[0]);
        assert!(matches!(instruction, PaintInstruction::Path(path)
            if path.commands.len() == 8));
    }

    #[test]
    fn authored_node_dash_pattern_reaches_backend_neutral_paint() {
        let mut layout = simple_layout();
        layout.nodes[0].style.stroke_dash = Some(vec![5.0, 3.0]);
        let instruction = node_shape_instruction(&layout.nodes[0]);
        assert!(matches!(
            instruction,
            PaintInstruction::Rect(rect)
                if rect.stroke_dash.as_deref() == Some(&[5.0, 3.0][..])
        ));
    }

    #[test]
    fn block_asymmetric_shape_lowers_to_five_point_path() {
        let mut layout = simple_layout();
        layout.nodes[0].shape = DiagramShape::Asymmetric;
        assert!(matches!(
            node_shape_instruction(&layout.nodes[0]),
            PaintInstruction::Path(path) if path.commands.len() == 6
        ));
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
    fn styled_graph_edge_dash_reaches_backend_neutral_path() {
        let mut layout = simple_layout();
        layout.edges[0].style.stroke_dash = Some(vec![5.0, 4.0]);
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let scene = diagram_to_paint(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| {
            matches!(instruction, PaintInstruction::Path(path)
                if path.stroke_dash.as_deref() == Some(&[5.0, 4.0][..]))
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
                stroke_dash: Some(vec![7.0, 2.0]),
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
                    && rect.stroke_width == Some(3.0)
                    && rect.stroke_dash.as_deref() == Some(&[7.0, 2.0][..]))
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
    fn css_colors_preserve_shorthand_and_alpha() {
        assert_eq!(css_to_color("#369"), Color { r: 51, g: 102, b: 153, a: 255 });
        assert_eq!(css_to_color("#369c"), Color { r: 51, g: 102, b: 153, a: 204 });
        assert_eq!(css_to_color("#336699cc"), Color { r: 51, g: 102, b: 153, a: 204 });
        assert_eq!(css_to_color("transparent"), Color { r: 0, g: 0, b: 0, a: 0 });
        assert_eq!(with_opacity("#336699cc", 0.5), "rgba(51,102,153,0.4)");
    }

    #[test]
    fn css_colors_parse_legacy_and_modern_rgb_functions() {
        let expected = Color { r: 51, g: 102, b: 153, a: 204 };
        assert_eq!(css_to_color("rgba(51, 102, 153, 0.8)"), expected);
        assert_eq!(css_to_color("rgb(20% 40% 60% / 80%)"), expected);
        assert_eq!(with_opacity("rgb(20% 40% 60% / 80%)", 0.5), "rgba(51,102,153,0.4)");
    }

    #[test]
    fn css_colors_parse_legacy_and_modern_hsl_functions() {
        let expected = Color { r: 51, g: 102, b: 153, a: 204 };
        assert_eq!(css_to_color("hsla(210, 50%, 40%, 0.8)"), expected);
        assert_eq!(css_to_color("hsl(210deg 50% 40% / 80%)"), expected);
        assert_eq!(with_opacity("hsl(210deg 50% 40% / 80%)", 0.5), "rgba(51,102,153,0.4)");
        let cyan = Color { r: 0, g: 255, b: 255, a: 255 };
        assert_eq!(css_to_color("hsl(0.5turn 100% 50%)"), cyan);
        assert_eq!(css_to_color("hsl(200grad 100% 50%)"), cyan);
        assert_eq!(css_to_color("hsl(3.141592653589793rad 100% 50%)"), cyan);
    }

    #[test]
    fn css_colors_parse_hwb_functions() {
        let expected = Color { r: 51, g: 102, b: 153, a: 204 };
        assert_eq!(css_to_color("hwb(210 20% 40% / 80%)"), expected);
        assert_eq!(css_to_color("hwb(-150deg 20% 40% / 0.8)"), expected);
        assert_eq!(with_opacity("hwb(210 20% 40% / 80%)", 0.5), "rgba(51,102,153,0.4)");
        assert_eq!(css_to_color("hwb(0 80% 80%)"), Color { r: 128, g: 128, b: 128, a: 255 });
        assert_eq!(normalize_css_paint("hwb(210 20% 40% / 80%)".into()), "rgba(51,102,153,0.8)");
    }

    #[test]
    fn css_colors_parse_lab_and_lch_functions() {
        let expected = Color { r: 125, g: 35, b: 41, a: 204 };
        assert_eq!(css_to_color("lab(29.2345% 39.3825 20.0664 / 80%)"), expected);
        assert_eq!(css_to_color("lch(29.2345% 44.2 27 / 0.8)"), expected);
        assert_eq!(css_to_color("lab(100% 0 0)"), Color { r: 255, g: 255, b: 255, a: 255 });
        assert_eq!(css_to_color("lch(0% 0 0)"), Color { r: 0, g: 0, b: 0, a: 255 });
        assert_eq!(with_opacity("lab(29.2345% 39.3825 20.0664 / 80%)", 0.5), "rgba(125,35,41,0.4)");
        assert_eq!(normalize_css_paint("lch(29.2345% 44.2 27 / 80%)".into()), "rgba(125,35,41,0.8)");
    }

    #[test]
    fn css_colors_parse_oklab_and_oklch_functions() {
        let expected = Color { r: 125, g: 36, b: 41, a: 204 };
        assert_eq!(css_to_color("oklab(40.1% 0.1143 0.045 / 80%)"), expected);
        assert_eq!(css_to_color("oklch(40.1% 0.123 21.57 / 0.8)"), expected);
        assert_eq!(css_to_color("oklab(1 0 0)"), Color { r: 255, g: 255, b: 255, a: 255 });
        assert_eq!(css_to_color("oklch(0 0 0)"), Color { r: 0, g: 0, b: 0, a: 255 });
        assert_eq!(with_opacity("oklab(40.1% 0.1143 0.045 / 80%)", 0.5), "rgba(125,36,41,0.4)");
        assert_eq!(normalize_css_paint("oklch(40.1% 0.123 21.57 / 80%)".into()), "rgba(125,36,41,0.8)");
    }

    #[test]
    fn css_colors_parse_color_srgb_profiles() {
        let expected = Color { r: 51, g: 102, b: 153, a: 204 };
        assert_eq!(css_to_color("color(srgb 0.2 0.4 0.6 / 0.8)"), expected);
        assert_eq!(css_to_color("color(srgb 20% 40% 60% / 80%)"), expected);
        assert_eq!(
            css_to_color("color(srgb-linear 0.0331047666 0.1328683216 0.3185467781 / 80%)"),
            expected,
        );
        assert_eq!(with_opacity("color(srgb 0.2 0.4 0.6 / 80%)", 0.5), "rgba(51,102,153,0.4)");
        assert_eq!(normalize_css_paint("color(srgb 20% 40% 60% / 80%)".into()), "rgba(51,102,153,0.8)");
    }

    #[test]
    fn css_colors_parse_color_display_p3_profile() {
        let expected = Color { r: 27, g: 104, b: 157, a: 204 };
        assert_eq!(css_to_color("color(display-p3 0.2 0.4 0.6 / 0.8)"), expected);
        assert_eq!(css_to_color("color(display-p3 20% 40% 60% / 80%)"), expected);
        assert_eq!(with_opacity("color(display-p3 0.2 0.4 0.6 / 80%)", 0.5), "rgba(27,104,157,0.4)");
        assert_eq!(normalize_css_paint("color(display-p3 20% 40% 60% / 80%)".into()), "rgba(27,104,157,0.8)");
    }

    #[test]
    fn css_colors_parse_color_a98_rgb_profile() {
        let expected = Color { r: 0, g: 102, b: 156, a: 204 };
        assert_eq!(css_to_color("color(a98-rgb 0.2 0.4 0.6 / 0.8)"), expected);
        assert_eq!(css_to_color("color(a98-rgb 20% 40% 60% / 80%)"), expected);
        assert_eq!(with_opacity("color(a98-rgb 0.2 0.4 0.6 / 80%)", 0.5), "rgba(0,102,156,0.4)");
        assert_eq!(normalize_css_paint("color(a98-rgb 20% 40% 60% / 80%)".into()), "rgba(0,102,156,0.8)");
    }

    #[test]
    fn css_colors_parse_color_prophoto_rgb_profile() {
        let expected = Color { r: 0, g: 130, b: 176, a: 204 };
        assert_eq!(css_to_color("color(prophoto-rgb 0.2 0.4 0.6 / 0.8)"), expected);
        assert_eq!(css_to_color("color(prophoto-rgb 20% 40% 60% / 80%)"), expected);
        assert_eq!(with_opacity("color(prophoto-rgb 0.2 0.4 0.6 / 80%)", 0.5), "rgba(0,130,176,0.4)");
        assert_eq!(normalize_css_paint("color(prophoto-rgb 20% 40% 60% / 80%)".into()), "rgba(0,130,176,0.8)");
    }

    #[test]
    fn css_colors_parse_color_rec2020_profile() {
        let expected = Color { r: 0, g: 119, b: 168, a: 204 };
        assert_eq!(css_to_color("color(rec2020 0.2 0.4 0.6 / 0.8)"), expected);
        assert_eq!(css_to_color("color(rec2020 20% 40% 60% / 80%)"), expected);
        assert_eq!(with_opacity("color(rec2020 0.2 0.4 0.6 / 80%)", 0.5), "rgba(0,119,168,0.4)");
        assert_eq!(normalize_css_paint("color(rec2020 20% 40% 60% / 80%)".into()), "rgba(0,119,168,0.8)");
    }

    #[test]
    fn css_colors_parse_color_xyz_profiles() {
        let d65 = Color { r: 0, g: 167, b: 164, a: 204 };
        let d50 = Color { r: 0, g: 168, b: 189, a: 204 };
        assert_eq!(css_to_color("color(xyz 0.2 0.3 0.4 / 0.8)"), d65);
        assert_eq!(css_to_color("color(xyz-d65 20% 30% 40% / 80%)"), d65);
        assert_eq!(css_to_color("color(xyz-d50 0.2 0.3 0.4 / 80%)"), d50);
        assert_eq!(with_opacity("color(xyz-d65 0.2 0.3 0.4 / 80%)", 0.5), "rgba(0,167,164,0.4)");
        assert_eq!(normalize_css_paint("color(xyz-d50 20% 30% 40% / 80%)".into()), "rgba(0,168,189,0.8)");
    }

    #[test]
    fn css_colors_lower_missing_components_to_zero() {
        assert_eq!(css_to_color("rgb(none 40% 60% / 80%)"), css_to_color("rgb(0% 40% 60% / 80%)"));
        assert_eq!(css_to_color("hsl(none 100% 50%)"), css_to_color("hsl(0 100% 50%)"));
        assert_eq!(css_to_color("hwb(none 20% 40%)"), css_to_color("hwb(0 20% 40%)"));
        assert_eq!(css_to_color("lab(29.2345% none 20.0664)"), css_to_color("lab(29.2345% 0 20.0664)"));
        assert_eq!(css_to_color("lch(29.2345% none none)"), css_to_color("lch(29.2345% 0 0)"));
        assert_eq!(css_to_color("oklab(40.1% none 0.045)"), css_to_color("oklab(40.1% 0 0.045)"));
        assert_eq!(css_to_color("oklch(40.1% none none)"), css_to_color("oklch(40.1% 0 0)"));
        assert_eq!(
            css_to_color("color(display-p3 none 0.4 0.6 / 80%)"),
            css_to_color("color(display-p3 0 0.4 0.6 / 80%)"),
        );
        assert_eq!(css_to_color("rgb(20% 40% 60% / none)").a, 0);
        assert_eq!(css_to_color("rgb(20% 40% 60%)").a, 255);
    }

    #[test]
    fn css_colors_mix_in_srgb() {
        assert_eq!(css_to_color("color-mix(in srgb, red, blue)"), Color { r: 128, g: 0, b: 128, a: 255 });
        assert_eq!(css_to_color("color-mix(in srgb, red 25%, blue)"), Color { r: 64, g: 0, b: 191, a: 255 });
        assert_eq!(
            css_to_color("color-mix(in srgb, rgb(255, 0, 0) 20%, transparent)"),
            Color { r: 255, g: 0, b: 0, a: 51 },
        );
        assert_eq!(
            css_to_color("color-mix(in srgb, red 20%, blue 20%)"),
            Color { r: 128, g: 0, b: 128, a: 102 },
        );
        assert_eq!(with_opacity("color-mix(in srgb, red, blue)", 0.5), "rgba(128,0,128,0.5)");
    }

    #[test]
    fn css_colors_mix_in_linear_srgb() {
        assert_eq!(
            css_to_color("color-mix(in srgb-linear, black, white)"),
            Color { r: 188, g: 188, b: 188, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in srgb-linear, red, blue)"),
            Color { r: 188, g: 0, b: 188, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in srgb-linear, red 20%, transparent)"),
            Color { r: 255, g: 0, b: 0, a: 51 },
        );
        assert_eq!(with_opacity("color-mix(in srgb-linear, black, white)", 0.5), "rgba(188,188,188,0.5)");
    }

    #[test]
    fn css_colors_mix_in_display_p3() {
        assert_eq!(
            css_to_color("color-mix(in display-p3, black, white)"),
            Color { r: 128, g: 128, b: 128, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in display-p3, red, blue)"),
            Color { r: 128, g: 10, b: 145, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in display-p3, red 20%, transparent)"),
            Color { r: 255, g: 0, b: 0, a: 51 },
        );
        assert_eq!(with_opacity("color-mix(in display-p3, black, white)", 0.5), "rgba(128,128,128,0.5)");
    }

    #[test]
    fn css_colors_mix_in_a98_rgb() {
        assert_eq!(
            css_to_color("color-mix(in a98-rgb, black, white)"),
            Color { r: 129, g: 129, b: 129, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in a98-rgb, red, blue)"),
            Color { r: 129, g: 0, b: 129, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in a98-rgb, red 20%, transparent)"),
            Color { r: 255, g: 0, b: 0, a: 51 },
        );
        assert_eq!(with_opacity("color-mix(in a98-rgb, black, white)", 0.5), "rgba(129,129,129,0.5)");
    }

    #[test]
    fn css_colors_mix_in_prophoto_rgb() {
        assert_eq!(
            css_to_color("color-mix(in prophoto-rgb, black, white)"),
            Color { r: 146, g: 146, b: 146, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in prophoto-rgb, red, blue)"),
            Color { r: 186, g: 3, b: 157, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in prophoto-rgb, red 20%, transparent)"),
            Color { r: 255, g: 0, b: 0, a: 51 },
        );
        assert_eq!(with_opacity("color-mix(in prophoto-rgb, black, white)", 0.5), "rgba(146,146,146,0.5)");
    }

    #[test]
    fn css_colors_mix_in_rec2020() {
        assert_eq!(
            css_to_color("color-mix(in rec2020, black, white)"),
            Color { r: 139, g: 139, b: 139, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in rec2020, red, blue)"),
            Color { r: 162, g: 19, b: 148, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in rec2020, red 20%, transparent)"),
            Color { r: 255, g: 0, b: 0, a: 51 },
        );
        assert_eq!(with_opacity("color-mix(in rec2020, black, white)", 0.5), "rgba(139,139,139,0.5)");
    }

    #[test]
    fn css_colors_mix_in_xyz_spaces() {
        assert_eq!(
            css_to_color("color-mix(in xyz, black, white)"),
            Color { r: 188, g: 188, b: 188, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in xyz-d65, red, blue)"),
            Color { r: 188, g: 0, b: 188, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in xyz-d50, red, blue)"),
            Color { r: 188, g: 0, b: 188, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in xyz-d50, red 20%, transparent)"),
            Color { r: 255, g: 0, b: 0, a: 51 },
        );
        assert_eq!(with_opacity("color-mix(in xyz-d65, black, white)", 0.5), "rgba(188,188,188,0.5)");
    }

    #[test]
    fn css_colors_mix_in_hsl() {
        assert_eq!(
            css_to_color("color-mix(in hsl, black, white)"),
            Color { r: 128, g: 128, b: 128, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in hsl, red, blue)"),
            Color { r: 255, g: 0, b: 255, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in hsl, red 20%, transparent)"),
            Color { r: 255, g: 0, b: 0, a: 51 },
        );
        assert_eq!(with_opacity("color-mix(in hsl, black, white)", 0.5), "rgba(128,128,128,0.5)");
    }

    #[test]
    fn css_colors_mix_with_explicit_hue_interpolation() {
        assert_eq!(
            css_to_color("color-mix(in hsl shorter hue, red, blue)"),
            Color { r: 255, g: 0, b: 255, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in hsl longer hue, red, blue)"),
            Color { r: 0, g: 255, b: 0, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in hsl increasing hue, red, blue)"),
            Color { r: 0, g: 255, b: 0, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in hsl decreasing hue, red, blue)"),
            Color { r: 255, g: 0, b: 255, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in srgb longer hue, red, blue)"),
            Color { r: 0, g: 0, b: 0, a: 255 },
        );
    }

    #[test]
    fn css_colors_mix_in_hwb() {
        assert_eq!(
            css_to_color("color-mix(in hwb, black, white)"),
            Color { r: 128, g: 128, b: 128, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in hwb, red, blue)"),
            Color { r: 255, g: 0, b: 255, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in hwb, red 20%, transparent)"),
            Color { r: 255, g: 0, b: 0, a: 51 },
        );
        assert_eq!(with_opacity("color-mix(in hwb, black, white)", 0.5), "rgba(128,128,128,0.5)");
    }

    #[test]
    fn css_colors_mix_in_lab() {
        assert_eq!(
            css_to_color("color-mix(in lab, black, white)"),
            Color { r: 119, g: 119, b: 119, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in lab, red, blue)"),
            Color { r: 193, g: 0, b: 136, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in lab, red 100%, blue 0%)"),
            Color { r: 255, g: 0, b: 0, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in lab, red 20%, transparent)"),
            Color { r: 255, g: 0, b: 0, a: 51 },
        );
        assert_eq!(with_opacity("color-mix(in lab, black, white)", 0.5), "rgba(119,119,119,0.5)");
    }

    #[test]
    fn css_colors_mix_in_lch() {
        assert_eq!(
            css_to_color("color-mix(in lch, black, white)"),
            Color { r: 119, g: 119, b: 119, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in lch, red, blue)"),
            Color { r: 245, g: 0, b: 134, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in lch, red 20%, transparent)"),
            Color { r: 255, g: 0, b: 0, a: 51 },
        );
        assert_eq!(with_opacity("color-mix(in lch, black, white)", 0.5), "rgba(119,119,119,0.5)");
    }

    #[test]
    fn css_colors_mix_in_oklab() {
        assert_eq!(
            css_to_color("color-mix(in oklab, black, white)"),
            Color { r: 99, g: 99, b: 99, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in oklab, red, blue)"),
            Color { r: 140, g: 83, b: 162, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in oklab, red 20%, transparent)"),
            Color { r: 255, g: 0, b: 0, a: 51 },
        );
        assert_eq!(with_opacity("color-mix(in oklab, black, white)", 0.5), "rgba(99,99,99,0.5)");
    }

    #[test]
    fn css_colors_mix_in_oklch() {
        assert_eq!(
            css_to_color("color-mix(in oklch, black, white)"),
            Color { r: 99, g: 99, b: 99, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in oklch, red, blue)"),
            Color { r: 186, g: 0, b: 194, a: 255 },
        );
        assert_eq!(
            css_to_color("color-mix(in oklch, red 20%, transparent)"),
            Color { r: 255, g: 0, b: 0, a: 51 },
        );
        assert_eq!(with_opacity("color-mix(in oklch, black, white)", 0.5), "rgba(99,99,99,0.5)");
    }

    #[test]
    fn css_colors_parse_basic_named_colors() {
        assert_eq!(css_to_color("ORANGE"), Color { r: 255, g: 165, b: 0, a: 255 });
        assert_eq!(css_to_color("navy"), Color { r: 0, g: 0, b: 128, a: 255 });
        assert_eq!(css_to_color("grey"), css_to_color("gray"));
        assert_eq!(with_opacity("teal", 0.5), "rgba(0,128,128,0.5)");
    }

    #[test]
    fn css_colors_parse_extended_named_colors() {
        assert_eq!(css_to_color("rebeccapurple"), Color { r: 102, g: 51, b: 153, a: 255 });
        assert_eq!(css_to_color("CornflowerBlue"), Color { r: 100, g: 149, b: 237, a: 255 });
        assert_eq!(css_to_color("papayawhip"), Color { r: 255, g: 239, b: 213, a: 255 });
        assert_eq!(css_to_color("darkslategrey"), css_to_color("darkslategray"));
    }

    #[test]
    fn css_none_paint_lowers_to_transparent_without_becoming_a_text_color() {
        assert_eq!(normalize_css_paint("none".into()), "rgba(0,0,0,0)");
        assert_eq!(normalize_css_paint("TRANSPARENT".into()), "rgba(0,0,0,0)");
        assert_eq!(with_opacity("none", 0.5), "rgba(0,0,0,0)");
        assert_eq!(css_to_color("none"), Color { r: 0, g: 0, b: 0, a: 255 });
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
            config: Default::default(),
            title: Some("Allocation".into()),
            accessibility_title: Some("Allocation treemap".into()),
            accessibility_description: None,
            nodes: vec![diagram_ir::LayoutedTreemapNode {
                id: "root".into(),
                label: "Root".into(),
                value: 10.0,
                depth: 0,
                has_children: false,
                palette_index: Some(0),
                x: 8.0,
                y: 48.0,
                width: 304.0,
                height: 184.0,
                class_selector: None,
                style: Some(diagram_ir::TreemapStyle {
                    node: diagram_ir::DiagramStyle {
                        fill: Some("currentColor".into()), stroke: Some("currentcolor".into()), stroke_width: Some(3.0),
                        stroke_dash: Some(vec![5.0, 2.0]), text_color: Some("rgb(47.059% 20.784% 5.882% / 80%)".into()), font_size: Some(17.0),
                        font_weight: Some(700), font_italic: Some(true), font_family: Some("Avenir".into()), corner_radius: Some(9.0),
                    },
                    font_size: Some(diagram_ir::TreemapFontSize::Factor(1.25)),
                    border_radius: Some(diagram_ir::TreemapBorderRadius::Factor(0.25)),
                    opacity: Some(0.8), fill_opacity: Some(0.5), stroke_opacity: Some(0.5), stroke_dash_offset: Some(-1.0),
                    text_align: Some(diagram_ir::TreemapTextAlign::End),
                    text_align_last: None,
                    text_justify: None,
                    text_transform: Some(diagram_ir::TreemapTextTransform::FullWidth),
                    text_decoration: Some(diagram_ir::TreemapTextDecoration {
                        underline: true, overline: true, line_through: true,
                    }),
                    text_decoration_color: Some(diagram_ir::TreemapTextDecorationColor::Color("#2563eb".into())),
                    text_decoration_style: Some(diagram_ir::TreemapTextDecorationStyle::Wavy),
                    text_decoration_thickness: Some(diagram_ir::TreemapTextDecorationThickness::Factor(0.25)),
                    text_underline_offset: Some(diagram_ir::TreemapTextUnderlineOffset::Factor(0.25)),
                    text_underline_position: Some(diagram_ir::TreemapTextUnderlinePosition::Under),
                    line_height: Some(diagram_ir::TreemapLineHeight::Pixels(24.0)),
                    text_indent: Some(diagram_ir::TreemapTextIndent::Factor(0.1)),
                    text_indent_hanging: true,
                    text_indent_each_line: true,
                    white_space: Some(diagram_ir::TreemapWhiteSpace::NoWrap),
                    overflow_wrap: None,
                    word_break: None,
                    line_break: None,
                    hyphens: None,
                    hyphenate_character: None,
                    text_overflow: None,
                    text_wrap_mode: None,
                    text_wrap_style: None,
                    letter_spacing: None,
                    word_spacing: None,
                    direction: None,
                    text_shadow: None,
                    tab_size: None,
                    font_stretch: Some(diagram_ir::TreemapFontStretch::Percentage(80.0)),
                }),
            }],
        };

        let mut relative_font = opts.label_font.clone();
        relative_font.size = 17.0;
        apply_treemap_font_size(&mut relative_font, layout.nodes[0].style.as_ref());
        assert_eq!(relative_font.size, 21.25);

        let styled_text = treemap_text_node(
            "Styled node", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            Color { r: 0, g: 0, b: 0, a: 255 }, layout.nodes[0].style.as_ref(),
        );
        assert!(matches!(styled_text.content,
            Some(Content::Text(TextContent { value, wrap: false, text_align: TextAlign::End, font,
                decoration: Some(TextDecoration { lines, color: Some(decoration_color),
                    style: TextDecorationStyle::Wavy, thickness: Some(3.5), underline_offset: Some(3.5),
                    underline_position: TextUnderlinePosition::Under }), .. }))
                if value == "\u{ff33}\u{ff54}\u{ff59}\u{ff4c}\u{ff45}\u{ff44}\u{3000}\u{ff4e}\u{ff4f}\u{ff44}\u{ff45}"
                    && font.stretch == FontStretch::Condensed
                    && lines.contains(TextDecorationLines::UNDERLINE)
                    && lines.contains(TextDecorationLines::OVERLINE)
                    && lines.contains(TextDecorationLines::LINE_THROUGH)
                    && decoration_color == (Color { r: 37, g: 99, b: 235, a: 204 })));
        assert_eq!(styled_text.x, 0.0);
        assert_eq!(styled_text.width, 100.0);
        assert_eq!(styled_text.ext.get("text.indent"), Some(&ExtValue::Float(10.0)));
        assert_eq!(styled_text.ext.get("text.indent-hanging"), Some(&ExtValue::Bool(true)));
        assert_eq!(styled_text.ext.get("text.indent-each-line"), Some(&ExtValue::Bool(true)));
        let mut current_color_style = layout.nodes[0].style.clone().expect("treemap style");
        current_color_style.text_decoration_color =
            Some(diagram_ir::TreemapTextDecorationColor::CurrentColor);
        let current_color = Color { r: 12, g: 34, b: 56, a: 78 };
        let current_color_text = treemap_text_node(
            "Current color", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&current_color_style),
        );
        assert!(matches!(current_color_text.content,
            Some(Content::Text(TextContent {
                decoration: Some(TextDecoration { color: Some(color), .. }), ..
            })) if color == current_color));
        let mut whitespace_style = current_color_style;
        whitespace_style.text_transform = Some(diagram_ir::TreemapTextTransform::None);
        whitespace_style.white_space = Some(diagram_ir::TreemapWhiteSpace::Normal);
        let collapsed_text = treemap_text_node(
            "  one \n two   three ", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert!(matches!(collapsed_text.content,
            Some(Content::Text(TextContent { value, wrap: true, .. })) if value == "one two three"));
        whitespace_style.white_space = Some(diagram_ir::TreemapWhiteSpace::PreLine);
        let pre_line_text = treemap_text_node(
            "  one \n two   three ", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert!(matches!(pre_line_text.content,
            Some(Content::Text(TextContent { value, wrap: true, .. })) if value == "one\ntwo three"));
        whitespace_style.white_space = Some(diagram_ir::TreemapWhiteSpace::Pre);
        whitespace_style.text_transform = Some(diagram_ir::TreemapTextTransform::Capitalize);
        let pre_capitalized_text = treemap_text_node(
            "one  two\nthree", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert!(matches!(pre_capitalized_text.content,
            Some(Content::Text(TextContent { value, wrap: false, .. })) if value == "One  Two\nThree"));
        whitespace_style.text_transform = Some(diagram_ir::TreemapTextTransform::FullSizeKana);
        let full_size_kana_text = treemap_text_node(
            "ゃャㇰ", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert!(matches!(full_size_kana_text.content,
            Some(Content::Text(TextContent { value, .. })) if value == "やヤク"));
        whitespace_style.text_transform = Some(diagram_ir::TreemapTextTransform::Capitalize);
        whitespace_style.white_space = Some(diagram_ir::TreemapWhiteSpace::BreakSpaces);
        let break_spaces_text = treemap_text_node(
            "one  two", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert!(matches!(break_spaces_text.content,
            Some(Content::Text(TextContent { value, wrap: true, .. })) if value == "One  Two"));
        assert_eq!(break_spaces_text.ext.get("text.break-spaces"), Some(&ExtValue::Bool(true)));
        whitespace_style.white_space = Some(diagram_ir::TreemapWhiteSpace::PreserveSpaces);
        let preserve_spaces_text = treemap_text_node(
            "one\t two\nthree", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert!(matches!(preserve_spaces_text.content,
            Some(Content::Text(TextContent { value, wrap: true, .. })) if value == "One  Two Three"));
        assert_eq!(preserve_spaces_text.ext.get("text.preserve-spaces"), Some(&ExtValue::Bool(true)));
        whitespace_style.letter_spacing = Some(diagram_ir::TreemapLetterSpacing::Factor(0.125));
        let letter_spaced_text = treemap_text_node(
            "tracked", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(letter_spaced_text.ext.get("text.letter-spacing"), Some(&ExtValue::Float(1.75)));
        whitespace_style.word_spacing = Some(diagram_ir::TreemapWordSpacing::Factor(0.25));
        let word_spaced_text = treemap_text_node(
            "two words", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(word_spaced_text.ext.get("text.word-spacing"), Some(&ExtValue::Float(3.5)));
        whitespace_style.text_align_last = Some(diagram_ir::TreemapTextAlignLast::Start);
        let last_aligned_text = treemap_text_node(
            "last line", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(last_aligned_text.ext.get("text.align-last"), Some(&ExtValue::Str("start".into())));
        whitespace_style.text_align_last = Some(diagram_ir::TreemapTextAlignLast::Justify);
        let last_justified_text = treemap_text_node(
            "last line", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(last_justified_text.ext.get("text.align-last"), Some(&ExtValue::Str("justify".into())));
        whitespace_style.text_justify = Some(diagram_ir::TreemapTextJustify::InterCharacter);
        let character_justified_text = treemap_text_node(
            "last line", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(character_justified_text.ext.get("text.justify-mode"),
            Some(&ExtValue::Str("inter-character".into())));
        whitespace_style.overflow_wrap = Some(diagram_ir::TreemapOverflowWrap::Anywhere);
        let anywhere_text = treemap_text_node(
            "unbreakable", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(anywhere_text.ext.get("text.overflow-wrap"), Some(&ExtValue::Str("anywhere".into())));
        whitespace_style.word_break = Some(diagram_ir::TreemapWordBreak::KeepAll);
        let keep_all_text = treemap_text_node(
            "日本語", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(keep_all_text.ext.get("text.word-break"), Some(&ExtValue::Str("keep-all".into())));
        whitespace_style.line_break = Some(diagram_ir::TreemapLineBreak::Loose);
        let loose_text = treemap_text_node(
            "あぁ", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(loose_text.ext.get("text.line-break"), Some(&ExtValue::Str("loose".into())));
        whitespace_style.hyphens = Some(diagram_ir::TreemapHyphens::Manual);
        let hyphenated_text = treemap_text_node(
            "extra\u{ad}ordinary", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(hyphenated_text.ext.get("text.hyphens"), Some(&ExtValue::Str("manual".into())));
        whitespace_style.hyphenate_character = Some(
            diagram_ir::TreemapHyphenateCharacter::Character("‐".into()));
        let custom_hyphen_text = treemap_text_node(
            "extra\u{ad}ordinary", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(custom_hyphen_text.ext.get("text.hyphenate-character"),
            Some(&ExtValue::Str("‐".into())));
        whitespace_style.text_overflow = Some(diagram_ir::TreemapTextOverflow::Ellipsis);
        let ellipsis_text = treemap_text_node(
            "overflow", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(ellipsis_text.ext.get("text.overflow"), Some(&ExtValue::Str("ellipsis".into())));
        whitespace_style.text_wrap_mode = Some(diagram_ir::TreemapTextWrapMode::NoWrap);
        let nowrap_text = treemap_text_node(
            "one two", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(nowrap_text.ext.get("text.wrap-mode"), Some(&ExtValue::Str("nowrap".into())));
        whitespace_style.text_wrap_style = Some(diagram_ir::TreemapTextWrapStyle::Balance);
        let balanced_text = treemap_text_node(
            "one two three", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(balanced_text.ext.get("text.wrap-style"), Some(&ExtValue::Str("balance".into())));
        whitespace_style.text_wrap_style = Some(diagram_ir::TreemapTextWrapStyle::Pretty);
        let pretty_text = treemap_text_node(
            "one two three", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(pretty_text.ext.get("text.wrap-style"), Some(&ExtValue::Str("pretty".into())));
        whitespace_style.text_wrap_style = Some(diagram_ir::TreemapTextWrapStyle::Stable);
        let stable_text = treemap_text_node(
            "one two three", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(stable_text.ext.get("text.wrap-style"), Some(&ExtValue::Str("stable".into())));
        whitespace_style.text_align = Some(diagram_ir::TreemapTextAlign::Justify);
        let justified_text = treemap_text_node(
            "one two three", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert_eq!(justified_text.ext.get("text.justify"), Some(&ExtValue::Bool(true)));
        whitespace_style.direction = Some(diagram_ir::TreemapTextDirection::RightToLeft);
        let rtl_text = treemap_text_node(
            "مرحبا", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert!(matches!(rtl_text.ext.get("html"),
            Some(ExtValue::Map(html)) if html.get("dir") == Some(&ExtValue::Str("rtl".into()))));
        whitespace_style.text_shadow = Some(diagram_ir::TreemapTextShadow::Shadows(vec![
            diagram_ir::TreemapTextShadowLayer {
                offset_x: 2.0, offset_y: 3.0, blur_radius: 4.0,
                color: diagram_ir::TreemapTextShadowColor::Color("#334155".into()),
            },
            diagram_ir::TreemapTextShadowLayer {
                offset_x: -1.0, offset_y: 0.0, blur_radius: 0.0,
                color: diagram_ir::TreemapTextShadowColor::CurrentColor,
            },
        ]));
        let shadowed_text = treemap_text_node(
            "shadowed", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert!(matches!(EffectStyle::from_positioned(&shadowed_text).filters.as_slice(),
            [EffectFilter::DropShadow { dx: 2.0, dy: 3.0, blur: 4.0,
                color: EffectColor { r: 51, g: 65, b: 85, a: 204 } },
             EffectFilter::DropShadow { dx: -1.0, dy: 0.0, blur: 0.0,
                color: EffectColor { r: 12, g: 34, b: 56, a: 78 } }]));
        whitespace_style.white_space = Some(diagram_ir::TreemapWhiteSpace::Pre);
        whitespace_style.tab_size = Some(3);
        whitespace_style.text_transform = Some(diagram_ir::TreemapTextTransform::None);
        let tabbed_text = treemap_text_node(
            "one\ttwo", 0.0, 0.0, 100.0, 20.0, opts.label_font.clone(),
            current_color, Some(&whitespace_style),
        );
        assert!(matches!(tabbed_text.content,
            Some(Content::Text(TextContent { value, .. })) if value == "one   two"));
        let mut styled_font = opts.label_font.clone();
        styled_font.size = 16.0;
        apply_treemap_line_height(&mut styled_font, layout.nodes[0].style.as_ref());
        assert_eq!(styled_font.line_height, 1.5);

        let scene = diagram_to_paint_treemap(&layout, &opts);
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction, PaintInstruction::Rect(_))));
        assert_eq!(
            scene.metadata.as_ref().and_then(|metadata| metadata.get("accessibility.title")),
            Some(&"Allocation treemap".to_string())
        );
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction,
            PaintInstruction::Rect(rect) if rect.fill.as_deref() == Some("rgba(120,53,15,0.32)")
                && rect.stroke.as_deref() == Some("rgba(120,53,15,0.32)")
                && rect.stroke_width == Some(3.0)
                && rect.corner_radius == Some(46.0)
                && rect.stroke_dash.as_deref() == Some(&[5.0, 2.0][..])
                && rect.stroke_dash_offset == Some(-1.0))));
        assert!(scene.instructions.iter().any(|instruction| matches!(instruction,
            PaintInstruction::GlyphRun(run) if run.font_size == 21.25)));
        assert_eq!(format_treemap_value(12345.0, "$0,0"), "$12,345");
        assert_eq!(format_treemap_value(12.5, ".2f"), "12.50");
        assert_eq!(format_treemap_value(12345.0, "$0,0.00"), "$12,345.00");
        assert_eq!(format_treemap_value(0.125, ".1%"), "12.5%");
        assert_eq!(format_treemap_value(4500.0, ".2s"), "4.5k");
        assert_eq!(format_treemap_value(255.0, "x"), "ff");
        assert_eq!(format_treemap_value(12.0, "+d"), "+12");
        assert_eq!(format_treemap_value(12.5, ".3~f"), "12.5");
        assert_eq!(format_treemap_value(1234.5, "*>10,.2f"), "**1,234.50");
        assert_eq!(format_treemap_value(12.5, "*^12.1f"), "****12.5****");
        assert_eq!(format_treemap_value(12.5, "=+10.2f"), "+    12.50");
        assert_eq!(format_treemap_value(12345.0, "08,d"), "0012,345");
        assert_eq!(format_treemap_value(255.0, "#x"), "0xff");
        assert_eq!(format_treemap_value(-12.0, "(10.1f"), "    (12.0)");
        assert_eq!(format_treemap_value(12345.0, "invalid"), "12,345");
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
    fn chart_anchored_labels_preserve_horizontal_and_vertical_placement() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let labels = [
            ("S", ChartTextAnchor::Start, ChartTextBaseline::Top),
            ("M", ChartTextAnchor::Middle, ChartTextBaseline::Middle),
            ("E", ChartTextAnchor::End, ChartTextBaseline::Bottom),
        ];
        let layout = LayoutedChartDiagram {
            width: 400.0,
            height: 300.0,
            background_color: None,
            accessibility_title: None,
            accessibility_description: None,
            title_box: None,
            items: labels
                .into_iter()
                .map(|(text, anchor, baseline)| LayoutedChartItem::AnchoredLabel {
                    x: 200.0,
                    y: 100.0,
                    text: text.into(),
                    font_size: 12.0,
                    color: "#203040".into(),
                    anchor,
                    baseline,
                })
                .collect(),
        };

        let scene = diagram_to_paint_chart(&layout, &opts);
        let glyphs = scene
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                PaintInstruction::GlyphRun(run) => run.glyphs.first(),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(glyphs.len(), 3);
        assert_eq!(
            [glyphs[0].x, glyphs[1].x, glyphs[2].x],
            [200.0, 197.0, 194.0]
        );
        assert!(glyphs[0].y > glyphs[1].y && glyphs[1].y > glyphs[2].y);
    }

    #[test]
    fn chart_vertical_legend_lowers_to_stroked_translucent_markers_and_glyphs() {
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_opts(&shaper, &metrics, &resolver);
        let layout = LayoutedChartDiagram {
            width: 700.0,
            height: 700.0,
            background_color: None,
            accessibility_title: None,
            accessibility_description: None,
            title_box: None,
            items: vec![LayoutedChartItem::VerticalLegend {
                x: 612.5,
                y: 87.5,
                entries: vec![
                    diagram_ir::LegendEntry {
                        color: "#8686ff".into(),
                        label: "First".into(),
                    },
                    diagram_ir::LegendEntry {
                        color: "#ffff86".into(),
                        label: "Second".into(),
                    },
                ],
                box_size: 12.0,
                font_size: 12.0,
                line_height: 20.0,
                fill_opacity: 0.5,
            }],
        };

        let scene = diagram_to_paint_chart(&layout, &opts);
        let markers = scene
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                PaintInstruction::Rect(rect) => Some(rect),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(markers.len(), 2);
        assert_eq!((markers[0].x, markers[0].y), (612.5, 87.5));
        assert_eq!((markers[1].x, markers[1].y), (612.5, 107.5));
        assert_eq!(markers[0].fill.as_deref(), Some("rgba(134,134,255,0.5)"));
        assert_eq!(markers[0].stroke.as_deref(), Some("#8686ff"));
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
