//! # layout-to-paint
//!
//! UI04 — converts a positioned layout tree into a [`PaintScene`].
//!
//! ```text
//!  PositionedNode tree  (layout-ir)
//!          │
//!          ▼  layout_to_paint(root, &options)
//!  PaintScene           (paint-instructions / P2D00)
//!          │
//!          ▼  PaintVM dispatch
//!  pixels
//! ```
//!
//! This crate is the **UI04 amendment in action**: it does the shaping
//! itself, using the caller-supplied TXT00 trio (`FontResolver`,
//! `FontMetrics`, `TextShaper`). Paint backends downstream only ever
//! see `PaintGlyphRun` instructions with pre-positioned glyph IDs.
//! They never shape, never wrap, never measure. That is the whole
//! point of the amendment.
//!
//! ## v1 scope
//!
//! - Walks a [`PositionedNode`] tree in pre-order, accumulating
//!   absolute (x, y) from root.
//! - For each node with `ext["paint"]` data, emits background /
//!   border rectangles *before* the content, so paint order is back-
//!   to-front (painter's algorithm, per P2D00).
//! - For `TextContent`: resolves the font once via
//!   [`FontResolver`], then produces one `PaintGlyphRun` per wrapped
//!   line. Greedy word wrap at whitespace boundaries within
//!   `node.width`. Hard `'\n'` newlines force a line break.
//! - For `ImageContent`: emits a stub `PaintImage` referencing
//!   `src` — rasterization is the paint backend's job.
//! - Corner radius applied to background `PaintRect` when present.
//! - Color conversion: layout-ir `Color` (u8 RGBA) → CSS
//!   `"rgba(r, g, b, a/255)"` strings (paint-instructions uses CSS
//!   color strings).
//!
//! ## Explicit simplifications for v1 (documented, not hidden)
//!
//! - **No per-node padding preserved in PositionedNode.** The layout
//!   engine already absorbed padding into outer dimensions. Text
//!   renders at the node's (x, y + ascent) without an extra inset —
//!   code blocks etc. will visually look flush against their
//!   backgrounds. Acceptable for the "just works end to end" MVP;
//!   tracked for v2 as a layout-ir PositionedNode.padding field.
//! - **No clip push** for rounded corners. The rounded background
//!   rectangle is rendered correctly; its content is drawn on top
//!   without being clipped to the radius. Metal and Canvas both
//!   handle this correctly at the background level.
//! - CSS shadows are represented by the shared drop-shadow filter and therefore
//!   follow the composited node subtree rather than a box-only silhouette.
//! - Images render as a `PaintImage`; shared replaced metadata resolves
//!   intrinsic-ratio fit geometry while resource decoding remains host-owned.

use std::collections::HashMap;

use layout_backgrounds::{
    BackgroundBox, BackgroundColor, BackgroundRepeat, BackgroundSize, BackgroundSource,
    BackgroundStyle, BoxEdges, CornerRadius, LengthPercent, Rect as BackgroundRect,
};
use layout_effects::{EffectBlendMode, EffectColor, EffectFilter, EffectStyle};
use layout_ir::{
    Color, Content, ExtValue, FontSpec, PositionedNode, TextAlign, TextContent, TextDecorationLines,
};
use layout_positioned::{Position, PositionedStyle};
use layout_replaced::{object_fit_rect, IntrinsicSize};
use paint_instructions::{
    BlendMode, FillRule, FilterEffect, GlyphPosition, GradientKind, GradientStop, ImageSrc,
    PaintBase, PaintClip, PaintGlyphRun, PaintGradient, PaintGroup, PaintImage, PaintInstruction,
    PaintLayer, PaintPath, PaintRect, PaintScene, PathCommand,
};
use text_flow::{BaseDirection, Direction as FlowDirection, TextFlow};
use text_interfaces::{
    Direction, FontMetrics, FontQuery, FontResolver, FontStretch, FontStyle, FontWeight,
    ShapeOptions, ShapedText, TextShaper,
};

pub const VERSION: &str = "0.6.0";

// ═══════════════════════════════════════════════════════════════════════════
// Options
// ═══════════════════════════════════════════════════════════════════════════

/// Scene-level configuration passed alongside the tree. The
/// `shaper` / `metrics` / `resolver` triple must share a font
/// binding (same `Handle` associated type — the Rust type system
/// enforces this at compile time).
pub struct LayoutToPaintOptions<'a, S, M, R>
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    pub width: f64,
    pub height: f64,
    pub background: Color,
    pub device_pixel_ratio: f64,

    pub shaper: &'a S,
    pub metrics: &'a M,
    pub resolver: &'a R,
}

// ═══════════════════════════════════════════════════════════════════════════
// Entry point
// ═══════════════════════════════════════════════════════════════════════════

/// Walk a positioned layout tree and emit a flat `PaintScene`.
///
/// Shapes every `TextContent` along the way using `options.shaper`
/// against a handle produced by `options.resolver`. Emits
/// `PaintGlyphRun` instructions carrying pre-baked per-glyph
/// positions. The paint backend only has to rasterize.
pub fn layout_to_paint<S, M, R>(
    root: &PositionedNode,
    options: &LayoutToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let dpr = options.device_pixel_ratio.max(0.01);
    let mut out: Vec<PaintInstruction> = Vec::new();

    // Cache resolved handles by (family, weight, italic). Avoids
    // re-resolving the same FontSpec on every paragraph.
    let mut font_cache: HashMap<FontCacheKey, CachedFont<S::Handle>> = HashMap::new();

    // Iterative pre-order walk with an explicit stack. Using
    // recursion here would stack-overflow on hostile trees (e.g.
    // thousands of nested containers from an untrusted document
    // source). An explicit stack is bounded by heap, not thread
    // stack.
    let mut stack: Vec<WalkAction<'_>> = vec![WalkAction::Enter(WalkFrame {
        node: root,
        parent_abs_x: 0.0,
        parent_abs_y: 0.0,
        inherited_direction: BaseDirection::Auto,
    })];
    while let Some(action) = stack.pop() {
        let frame = match action {
            WalkAction::Enter(frame) => frame,
            WalkAction::ExitClip { start, clip } => {
                let children = out.drain(start..).collect();
                out.push(PaintInstruction::Clip(PaintClip { children, ..clip }));
                continue;
            }
            WalkAction::ExitFixed { start } => {
                let children = out.drain(start..).collect();
                out.push(PaintInstruction::Group(PaintGroup {
                    base: PaintBase {
                        id: None,
                        metadata: Some(HashMap::from([("layout.position".into(), "fixed".into())])),
                    },
                    children,
                    transform: None,
                    opacity: None,
                }));
                continue;
            }
            WalkAction::ExitSticky {
                start,
                top,
                original_y,
            } => {
                let children = out.drain(start..).collect();
                out.push(PaintInstruction::Group(PaintGroup {
                    base: PaintBase {
                        id: None,
                        metadata: Some(HashMap::from([
                            ("layout.position".into(), "sticky".into()),
                            ("layout.sticky.top".into(), top.to_string()),
                            ("layout.sticky.y".into(), original_y.to_string()),
                        ])),
                    },
                    children,
                    transform: None,
                    opacity: None,
                }));
                continue;
            }
            WalkAction::ExitEffect {
                start,
                style,
                x,
                y,
                width,
                height,
                dpr,
            } => {
                let children = out.drain(start..).collect();
                let transform = style.resolved_transform(x, y, width, height, dpr);
                let base = PaintBase {
                    id: None,
                    metadata: Some(HashMap::from([("layout.effects".into(), "css".into())])),
                };
                if style.needs_layer() {
                    out.push(PaintInstruction::Layer(PaintLayer {
                        base,
                        children,
                        filters: (!style.filters.is_empty())
                            .then(|| paint_filters(&style.filters, dpr)),
                        blend_mode: (style.blend_mode != EffectBlendMode::Normal)
                            .then(|| paint_blend_mode(style.blend_mode)),
                        opacity: (style.opacity < 1.0).then_some(style.opacity),
                        transform,
                    }));
                } else {
                    out.push(PaintInstruction::Group(PaintGroup {
                        base,
                        children,
                        transform,
                        opacity: None,
                    }));
                }
                continue;
            }
        };
        let abs_x = frame.parent_abs_x + frame.node.x;
        let abs_y = frame.parent_abs_y + frame.node.y;
        let box_w = frame.node.width;
        let box_h = frame.node.height;
        let direction = node_direction(frame.node).unwrap_or(frame.inherited_direction);
        let positioned = PositionedStyle::from_positioned(frame.node);
        let effects = EffectStyle::from_positioned(frame.node);
        let effect_start = (!effects.is_default()).then_some(out.len());
        let fixed_start = (positioned.position == Position::Fixed).then_some(out.len());
        let sticky_start = (positioned.position == Position::Sticky).then_some(out.len());

        // Decorations before content (painter's algorithm).
        emit_box_decorations(frame.node, abs_x, abs_y, box_w, box_h, dpr, &mut out);

        match &frame.node.content {
            Some(Content::Text(tc)) => {
                emit_text_content(
                    tc,
                    abs_x,
                    abs_y,
                    box_w,
                    dpr,
                    direction,
                    options,
                    &mut font_cache,
                    &mut out,
                );
            }
            Some(Content::Image(ic)) => {
                let fit = object_fit_rect(
                    ic.fit,
                    box_w,
                    box_h,
                    IntrinsicSize::from_positioned(frame.node),
                );
                let image = PaintInstruction::Image(PaintImage {
                    base: PaintBase::default(),
                    x: (abs_x + fit.x) * dpr,
                    y: (abs_y + fit.y) * dpr,
                    width: fit.width * dpr,
                    height: fit.height * dpr,
                    src: ImageSrc::Uri(ic.src.clone()),
                    opacity: None,
                });
                if fit.clips {
                    out.push(PaintInstruction::Clip(PaintClip {
                        base: PaintBase::default(),
                        x: abs_x * dpr,
                        y: abs_y * dpr,
                        width: box_w * dpr,
                        height: box_h * dpr,
                        path: clip_path_for_node(frame.node, abs_x, abs_y, box_w, box_h, dpr),
                        children: vec![image],
                    }));
                } else {
                    out.push(image);
                }
            }
            None => {}
        }

        if let Some(start) = effect_start {
            stack.push(WalkAction::ExitEffect {
                start,
                style: effects,
                x: abs_x,
                y: abs_y,
                width: box_w,
                height: box_h,
                dpr,
            });
        }
        if let Some(start) = fixed_start {
            stack.push(WalkAction::ExitFixed { start });
        }
        if let Some(start) = sticky_start {
            stack.push(WalkAction::ExitSticky {
                start,
                top: positioned.insets.top.unwrap_or(0.0),
                original_y: abs_y,
            });
        }
        if positioned.clips_x() || positioned.clips_y() {
            stack.push(WalkAction::ExitClip {
                start: out.len(),
                clip: PaintClip {
                    base: PaintBase::default(),
                    x: abs_x * dpr,
                    y: abs_y * dpr,
                    width: box_w.max(0.0) * dpr,
                    height: box_h.max(0.0) * dpr,
                    path: clip_path_for_node(frame.node, abs_x, abs_y, box_w, box_h, dpr),
                    children: Vec::new(),
                },
            });
        }

        // Push children in reverse so that popping preserves source
        // order (pre-order traversal).
        for child in frame.node.children.iter().rev() {
            stack.push(WalkAction::Enter(WalkFrame {
                node: child,
                parent_abs_x: abs_x,
                parent_abs_y: abs_y,
                inherited_direction: direction,
            }));
        }
    }

    PaintScene {
        width: options.width * dpr,
        height: options.height * dpr,
        background: color_to_css(options.background),
        instructions: out,
        id: None,
        metadata: None,
    }
}

/// One frame of the iterative walk's explicit stack.
struct WalkFrame<'a> {
    node: &'a PositionedNode,
    parent_abs_x: f64,
    parent_abs_y: f64,
    inherited_direction: BaseDirection,
}

enum WalkAction<'a> {
    Enter(WalkFrame<'a>),
    ExitClip {
        start: usize,
        clip: PaintClip,
    },
    ExitFixed {
        start: usize,
    },
    ExitSticky {
        start: usize,
        top: f64,
        original_y: f64,
    },
    ExitEffect {
        start: usize,
        style: EffectStyle,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        dpr: f64,
    },
}

fn paint_filters(filters: &[EffectFilter], dpr: f64) -> Vec<FilterEffect> {
    filters
        .iter()
        .map(|filter| match filter {
            EffectFilter::Blur { radius } => FilterEffect::Blur {
                radius: radius * dpr,
            },
            EffectFilter::DropShadow {
                dx,
                dy,
                blur,
                color,
            } => FilterEffect::DropShadow {
                dx: dx * dpr,
                dy: dy * dpr,
                blur: blur * dpr,
                color: effect_color_to_css(*color),
            },
            EffectFilter::Brightness { amount } => FilterEffect::Brightness { amount: *amount },
            EffectFilter::Contrast { amount } => FilterEffect::Contrast { amount: *amount },
            EffectFilter::Saturate { amount } => FilterEffect::Saturate { amount: *amount },
            EffectFilter::HueRotate { angle } => FilterEffect::HueRotate { angle: *angle },
            EffectFilter::Invert { amount } => FilterEffect::Invert { amount: *amount },
            EffectFilter::Opacity { amount } => FilterEffect::Opacity { amount: *amount },
        })
        .collect()
}

fn paint_blend_mode(mode: EffectBlendMode) -> BlendMode {
    match mode {
        EffectBlendMode::Normal => BlendMode::Normal,
        EffectBlendMode::Multiply => BlendMode::Multiply,
        EffectBlendMode::Screen => BlendMode::Screen,
        EffectBlendMode::Overlay => BlendMode::Overlay,
        EffectBlendMode::Darken => BlendMode::Darken,
        EffectBlendMode::Lighten => BlendMode::Lighten,
        EffectBlendMode::ColorDodge => BlendMode::ColorDodge,
        EffectBlendMode::ColorBurn => BlendMode::ColorBurn,
        EffectBlendMode::HardLight => BlendMode::HardLight,
        EffectBlendMode::SoftLight => BlendMode::SoftLight,
        EffectBlendMode::Difference => BlendMode::Difference,
        EffectBlendMode::Exclusion => BlendMode::Exclusion,
        EffectBlendMode::Hue => BlendMode::Hue,
        EffectBlendMode::Saturation => BlendMode::Saturation,
        EffectBlendMode::Color => BlendMode::Color,
        EffectBlendMode::Luminosity => BlendMode::Luminosity,
    }
}

fn effect_color_to_css(color: EffectColor) -> String {
    color_to_css(Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    })
}

fn node_direction(node: &PositionedNode) -> Option<BaseDirection> {
    let ExtValue::Map(html) = node.ext.get("html")? else {
        return None;
    };
    let ExtValue::Str(direction) = html.get("dir")? else {
        return None;
    };
    match direction.as_str() {
        "ltr" => Some(BaseDirection::Ltr),
        "rtl" => Some(BaseDirection::Rtl),
        "auto" => Some(BaseDirection::Auto),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Background + border rectangles
// ═══════════════════════════════════════════════════════════════════════════

fn emit_box_decorations(
    node: &PositionedNode,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    dpr: f64,
    out: &mut Vec<PaintInstruction>,
) {
    let empty_paint = HashMap::new();
    let paint_map = match node.ext.get("paint") {
        Some(ExtValue::Map(m)) => m,
        _ => &empty_paint,
    };

    let bg = read_color(paint_map, "backgroundColor");
    let border_color = read_color(paint_map, "borderColor");
    let border_width = read_float(paint_map, "borderWidth");
    let border_style = read_string(paint_map, "borderStyle").unwrap_or("solid");
    let corner_radius = read_float(paint_map, "cornerRadius");
    let side_borders = [
        ("borderTopWidth", "borderTopColor", "borderTopStyle"),
        ("borderRightWidth", "borderRightColor", "borderRightStyle"),
        (
            "borderBottomWidth",
            "borderBottomColor",
            "borderBottomStyle",
        ),
        ("borderLeftWidth", "borderLeftColor", "borderLeftStyle"),
    ]
    .map(|(width, color, style)| {
        (
            read_float(paint_map, width).unwrap_or(border_width.unwrap_or(0.0)),
            read_color(paint_map, color).or(border_color),
            read_string(paint_map, style).unwrap_or(border_style),
        )
    });

    let mut backgrounds = BackgroundStyle::from_positioned(node);
    backgrounds.border = BoxEdges {
        top: side_borders[0].0,
        right: side_borders[1].0,
        bottom: side_borders[2].0,
        left: side_borders[3].0,
    };
    if backgrounds.corners.iter().all(|corner| {
        corner.x.factor == 0.0
            && corner.x.offset == 0.0
            && corner.y.factor == 0.0
            && corner.y.offset == 0.0
    }) {
        if let Some(radius) = corner_radius.filter(|radius| *radius > 0.0) {
            backgrounds.corners = [CornerRadius {
                x: LengthPercent::length(radius),
                y: LengthPercent::length(radius),
            }; 4];
        }
    }
    if bg.is_none()
        && backgrounds.layers.is_empty()
        && border_color.is_none()
        && border_width.unwrap_or(0.0) == 0.0
        && side_borders.iter().all(|(width, _, _)| *width == 0.0)
    {
        return;
    }

    if w <= 0.0 || h <= 0.0 {
        return;
    }

    let border_box = BackgroundRect {
        x,
        y,
        width: w,
        height: h,
    };
    if let Some(background) = bg {
        let clip = backgrounds.painting_box(backgrounds.color_clip, border_box);
        if backgrounds.corners.iter().any(|corner| {
            corner.x.factor != 0.0
                || corner.x.offset != 0.0
                || corner.y.factor != 0.0
                || corner.y.offset != 0.0
        }) {
            out.push(PaintInstruction::Path(PaintPath {
                base: PaintBase::default(),
                commands: rounded_rect_path(
                    BackgroundRect {
                        x: clip.x * dpr,
                        y: clip.y * dpr,
                        width: clip.width * dpr,
                        height: clip.height * dpr,
                    },
                    backgrounds
                        .resolved_corners_for_box(backgrounds.color_clip, border_box)
                        .map(|(rx, ry)| (rx * dpr, ry * dpr)),
                ),
                fill: Some(color_to_css(background)),
                fill_rule: None,
                stroke: None,
                stroke_width: None,
                stroke_cap: None,
                stroke_join: None,
                stroke_dash: None,
                stroke_dash_offset: None,
            }));
        } else {
            out.push(PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: clip.x * dpr,
                y: clip.y * dpr,
                width: clip.width * dpr,
                height: clip.height * dpr,
                fill: Some(color_to_css(background)),
                stroke: None,
                stroke_width: None,
                corner_radius: corner_radius.map(|v| v * dpr),
                stroke_dash: None,
                stroke_dash_offset: None,
            }));
        }
    }

    emit_background_layers(node, &backgrounds, border_box, dpr, out);

    emit_borders(&backgrounds, border_box, dpr, side_borders, out);
}

fn emit_borders(
    style: &BackgroundStyle,
    border_box: BackgroundRect,
    dpr: f64,
    sides: [(f64, Option<Color>, &str); 4],
    out: &mut Vec<PaintInstruction>,
) {
    let outer = rounded_rect_path(
        scale_rect(border_box, dpr),
        style
            .resolved_corners(border_box)
            .map(|(rx, ry)| (rx * dpr, ry * dpr)),
    );
    let inner_box = style.painting_box(BackgroundBox::Padding, border_box);
    let inner = rounded_rect_path(
        scale_rect(inner_box, dpr),
        style
            .resolved_corners_for_box(BackgroundBox::Padding, border_box)
            .map(|(rx, ry)| (rx * dpr, ry * dpr)),
    );
    let mut ring = outer.clone();
    ring.extend(inner.clone());

    for (index, (width, color, border_style)) in sides.into_iter().enumerate() {
        if width <= 0.0 || matches!(border_style, "none" | "hidden") {
            continue;
        }
        let color = color_to_css(color.unwrap_or(Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }));
        let child = match border_style {
            "dashed" | "dotted" => stroked_border_path(
                outer.clone(),
                color,
                width * 2.0 * dpr,
                Some(if border_style == "dotted" {
                    vec![0.0, width * 2.0 * dpr]
                } else {
                    vec![width * 3.0 * dpr, width * 2.0 * dpr]
                }),
                (border_style == "dotted").then_some(paint_instructions::StrokeCap::Round),
            ),
            "double" => PaintInstruction::Group(PaintGroup {
                base: PaintBase::default(),
                children: vec![
                    stroked_border_path(
                        outer.clone(),
                        color.clone(),
                        width * 2.0 * dpr / 3.0,
                        None,
                        None,
                    ),
                    stroked_border_path(inner.clone(), color, width * 2.0 * dpr / 3.0, None, None),
                ],
                transform: None,
                opacity: None,
            }),
            _ => PaintInstruction::Path(PaintPath {
                base: PaintBase::default(),
                commands: ring.clone(),
                fill: Some(color),
                fill_rule: Some(FillRule::EvenOdd),
                stroke: None,
                stroke_width: None,
                stroke_cap: None,
                stroke_join: None,
                stroke_dash: None,
                stroke_dash_offset: None,
            }),
        };
        let clip_path = border_side_clip(index, border_box, style.border, dpr);
        out.push(PaintInstruction::Clip(PaintClip {
            base: PaintBase::default(),
            x: border_box.x * dpr,
            y: border_box.y * dpr,
            width: border_box.width * dpr,
            height: border_box.height * dpr,
            path: Some(clip_path),
            children: vec![child],
        }));
    }
}

fn scale_rect(rect: BackgroundRect, scale: f64) -> BackgroundRect {
    BackgroundRect {
        x: rect.x * scale,
        y: rect.y * scale,
        width: rect.width * scale,
        height: rect.height * scale,
    }
}

fn stroked_border_path(
    commands: Vec<PathCommand>,
    color: String,
    width: f64,
    dash: Option<Vec<f64>>,
    cap: Option<paint_instructions::StrokeCap>,
) -> PaintInstruction {
    PaintInstruction::Path(PaintPath {
        base: PaintBase::default(),
        commands,
        fill: None,
        fill_rule: None,
        stroke: Some(color),
        stroke_width: Some(width),
        stroke_cap: cap,
        stroke_join: Some(paint_instructions::StrokeJoin::Round),
        stroke_dash: dash,
        stroke_dash_offset: None,
    })
}

fn border_side_clip(
    side: usize,
    rect: BackgroundRect,
    border: layout_backgrounds::BoxEdges,
    dpr: f64,
) -> Vec<PathCommand> {
    let x = rect.x * dpr;
    let y = rect.y * dpr;
    let right = (rect.x + rect.width) * dpr;
    let bottom = (rect.y + rect.height) * dpr;
    let inner_left = (rect.x + border.left) * dpr;
    let inner_top = (rect.y + border.top) * dpr;
    let inner_right = (rect.x + rect.width - border.right) * dpr;
    let inner_bottom = (rect.y + rect.height - border.bottom) * dpr;
    let points = match side {
        0 => [
            (x, y),
            (right, y),
            (inner_right, inner_top),
            (inner_left, inner_top),
        ],
        1 => [
            (right, y),
            (right, bottom),
            (inner_right, inner_bottom),
            (inner_right, inner_top),
        ],
        2 => [
            (right, bottom),
            (x, bottom),
            (inner_left, inner_bottom),
            (inner_right, inner_bottom),
        ],
        _ => [
            (x, bottom),
            (x, y),
            (inner_left, inner_top),
            (inner_left, inner_bottom),
        ],
    };
    vec![
        PathCommand::MoveTo {
            x: points[0].0,
            y: points[0].1,
        },
        PathCommand::LineTo {
            x: points[1].0,
            y: points[1].1,
        },
        PathCommand::LineTo {
            x: points[2].0,
            y: points[2].1,
        },
        PathCommand::LineTo {
            x: points[3].0,
            y: points[3].1,
        },
        PathCommand::Close,
    ]
}

fn emit_background_layers(
    node: &PositionedNode,
    style: &BackgroundStyle,
    border_box: BackgroundRect,
    dpr: f64,
    out: &mut Vec<PaintInstruction>,
) {
    for (paint_index, layer) in style.layers.iter().rev().enumerate() {
        let origin = style.painting_box(layer.origin, border_box);
        let clip = style.painting_box(layer.clip, border_box);
        let (mut tile_w, mut tile_h) = background_tile_size(layer.size, origin);
        if layer.repeat_x == BackgroundRepeat::Round && tile_w > 0.0 {
            tile_w = clip.width / (clip.width / tile_w).round().max(1.0);
        }
        if layer.repeat_y == BackgroundRepeat::Round && tile_h > 0.0 {
            tile_h = clip.height / (clip.height / tile_h).round().max(1.0);
        }
        if tile_w <= 0.0 || tile_h <= 0.0 || clip.width <= 0.0 || clip.height <= 0.0 {
            continue;
        }
        let tile_x = origin.x + layer.position_x.resolve((origin.width - tile_w).max(0.0));
        let tile_y = origin.y + layer.position_y.resolve((origin.height - tile_h).max(0.0));
        let source_key = format!("{:?}", layer.source);
        let tiles = background_tiles(
            tile_x,
            tile_y,
            tile_w,
            tile_h,
            clip,
            layer.repeat_x,
            layer.repeat_y,
        );
        match &layer.source {
            BackgroundSource::LinearGradient {
                angle_degrees,
                stops,
            } => {
                for (tile_index, (x, y)) in tiles.iter().copied().enumerate() {
                    let tile = BackgroundRect {
                        x,
                        y,
                        width: tile_w,
                        height: tile_h,
                    };
                    let Some(painted) = intersect_rect(tile, clip) else {
                        continue;
                    };
                    let id = background_id(node, paint_index, tile_index, &source_key, border_box);
                    let angle = angle_degrees.to_radians();
                    let dx = angle.sin() * tile_w / 2.0;
                    let dy = -angle.cos() * tile_h / 2.0;
                    out.push(PaintInstruction::Gradient(PaintGradient {
                        base: PaintBase {
                            id: Some(id.clone()),
                            metadata: None,
                        },
                        kind: GradientKind::Linear {
                            x1: (x + tile_w / 2.0 - dx) * dpr,
                            y1: (y + tile_h / 2.0 - dy) * dpr,
                            x2: (x + tile_w / 2.0 + dx) * dpr,
                            y2: (y + tile_h / 2.0 + dy) * dpr,
                        },
                        stops: paint_gradient_stops(stops),
                    }));
                    emit_gradient_shape(style, border_box, layer.clip, painted, dpr, id, out);
                }
            }
            BackgroundSource::RadialGradient { stops } => {
                for (tile_index, (x, y)) in tiles.iter().copied().enumerate() {
                    let tile = BackgroundRect {
                        x,
                        y,
                        width: tile_w,
                        height: tile_h,
                    };
                    let Some(painted) = intersect_rect(tile, clip) else {
                        continue;
                    };
                    let id = background_id(node, paint_index, tile_index, &source_key, border_box);
                    out.push(PaintInstruction::Gradient(PaintGradient {
                        base: PaintBase {
                            id: Some(id.clone()),
                            metadata: None,
                        },
                        kind: GradientKind::Radial {
                            cx: (x + tile_w / 2.0) * dpr,
                            cy: (y + tile_h / 2.0) * dpr,
                            r: tile_w.max(tile_h) * dpr / 2.0,
                        },
                        stops: paint_gradient_stops(stops),
                    }));
                    emit_gradient_shape(style, border_box, layer.clip, painted, dpr, id, out);
                }
            }
            BackgroundSource::Image(uri) => {
                let mut children = Vec::new();
                for (tile_x, tile_y) in tiles {
                    children.push(PaintInstruction::Image(PaintImage {
                        base: PaintBase::default(),
                        x: tile_x * dpr,
                        y: tile_y * dpr,
                        width: tile_w * dpr,
                        height: tile_h * dpr,
                        src: ImageSrc::Uri(uri.clone()),
                        opacity: None,
                    }));
                }
                out.push(PaintInstruction::Clip(PaintClip {
                    base: PaintBase::default(),
                    x: clip.x * dpr,
                    y: clip.y * dpr,
                    width: clip.width * dpr,
                    height: clip.height * dpr,
                    path: rounded_clip_path(style, border_box, layer.clip, dpr),
                    children,
                }));
            }
        }
    }
}

fn background_tile_size(size: BackgroundSize, origin: BackgroundRect) -> (f64, f64) {
    match size {
        BackgroundSize::Explicit { width, height } => (
            width
                .map(|value| value.resolve(origin.width))
                .unwrap_or(origin.width),
            height
                .map(|value| value.resolve(origin.height))
                .unwrap_or(origin.height),
        ),
        BackgroundSize::Auto | BackgroundSize::Cover | BackgroundSize::Contain => {
            (origin.width, origin.height)
        }
    }
}

fn background_tiles(
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    clip: BackgroundRect,
    repeat_x: BackgroundRepeat,
    repeat_y: BackgroundRepeat,
) -> Vec<(f64, f64)> {
    let xs = tile_axis(x, w, clip.x, clip.width, repeat_x);
    let ys = tile_axis(y, h, clip.y, clip.height, repeat_y);
    xs.into_iter()
        .flat_map(|x| ys.iter().copied().map(move |y| (x, y)))
        .take(256)
        .collect()
}

fn tile_axis(
    start: f64,
    size: f64,
    clip_start: f64,
    clip_size: f64,
    repeat: BackgroundRepeat,
) -> Vec<f64> {
    if repeat == BackgroundRepeat::NoRepeat || size <= 0.0 {
        return vec![start];
    }
    if repeat == BackgroundRepeat::Space {
        let count = (clip_size / size).floor() as usize;
        if count <= 1 {
            return vec![start];
        }
        let gap = (clip_size - count as f64 * size) / (count - 1) as f64;
        return (0..count)
            .map(|index| clip_start + index as f64 * (size + gap))
            .collect();
    }
    let mut first = start;
    while first > clip_start {
        first -= size;
    }
    let mut out = Vec::new();
    let mut cursor = first;
    while cursor < clip_start + clip_size && out.len() < 256 {
        out.push(cursor);
        cursor += size;
    }
    out
}

fn emit_gradient_shape(
    style: &BackgroundStyle,
    border_box: BackgroundRect,
    clip_kind: BackgroundBox,
    painted: BackgroundRect,
    dpr: f64,
    id: String,
    out: &mut Vec<PaintInstruction>,
) {
    let clip = style.painting_box(clip_kind, border_box);
    let clip_corners = style.resolved_corners_for_box(clip_kind, border_box);
    let at_left = (painted.x - clip.x).abs() < f64::EPSILON;
    let at_top = (painted.y - clip.y).abs() < f64::EPSILON;
    let at_right = (painted.x + painted.width - clip.x - clip.width).abs() < f64::EPSILON;
    let at_bottom = (painted.y + painted.height - clip.y - clip.height).abs() < f64::EPSILON;
    let corners = [
        if at_left && at_top {
            clip_corners[0]
        } else {
            (0.0, 0.0)
        },
        if at_right && at_top {
            clip_corners[1]
        } else {
            (0.0, 0.0)
        },
        if at_right && at_bottom {
            clip_corners[2]
        } else {
            (0.0, 0.0)
        },
        if at_left && at_bottom {
            clip_corners[3]
        } else {
            (0.0, 0.0)
        },
    ]
    .map(|(rx, ry)| (rx * dpr, ry * dpr));
    out.push(PaintInstruction::Path(PaintPath {
        base: PaintBase::default(),
        commands: rounded_rect_path(
            BackgroundRect {
                x: painted.x * dpr,
                y: painted.y * dpr,
                width: painted.width * dpr,
                height: painted.height * dpr,
            },
            corners,
        ),
        fill: Some(format!("url(#{id})")),
        fill_rule: None,
        stroke: None,
        stroke_width: None,
        stroke_cap: None,
        stroke_join: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    }));
}

fn intersect_rect(a: BackgroundRect, b: BackgroundRect) -> Option<BackgroundRect> {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    (right > x && bottom > y).then_some(BackgroundRect {
        x,
        y,
        width: right - x,
        height: bottom - y,
    })
}

fn rounded_rect_path(rect: BackgroundRect, c: [(f64, f64); 4]) -> Vec<PathCommand> {
    const KAPPA: f64 = 0.552_284_749_830_793_6;
    vec![
        PathCommand::MoveTo {
            x: rect.x + c[0].0,
            y: rect.y,
        },
        PathCommand::LineTo {
            x: rect.x + rect.width - c[1].0,
            y: rect.y,
        },
        PathCommand::CubicTo {
            cx1: rect.x + rect.width - c[1].0 + c[1].0 * KAPPA,
            cy1: rect.y,
            cx2: rect.x + rect.width,
            cy2: rect.y + c[1].1 - c[1].1 * KAPPA,
            x: rect.x + rect.width,
            y: rect.y + c[1].1,
        },
        PathCommand::LineTo {
            x: rect.x + rect.width,
            y: rect.y + rect.height - c[2].1,
        },
        PathCommand::CubicTo {
            cx1: rect.x + rect.width,
            cy1: rect.y + rect.height - c[2].1 + c[2].1 * KAPPA,
            cx2: rect.x + rect.width - c[2].0 + c[2].0 * KAPPA,
            cy2: rect.y + rect.height,
            x: rect.x + rect.width - c[2].0,
            y: rect.y + rect.height,
        },
        PathCommand::LineTo {
            x: rect.x + c[3].0,
            y: rect.y + rect.height,
        },
        PathCommand::CubicTo {
            cx1: rect.x + c[3].0 - c[3].0 * KAPPA,
            cy1: rect.y + rect.height,
            cx2: rect.x,
            cy2: rect.y + rect.height - c[3].1 + c[3].1 * KAPPA,
            x: rect.x,
            y: rect.y + rect.height - c[3].1,
        },
        PathCommand::LineTo {
            x: rect.x,
            y: rect.y + c[0].1,
        },
        PathCommand::CubicTo {
            cx1: rect.x,
            cy1: rect.y + c[0].1 - c[0].1 * KAPPA,
            cx2: rect.x + c[0].0 - c[0].0 * KAPPA,
            cy2: rect.y,
            x: rect.x + c[0].0,
            y: rect.y,
        },
        PathCommand::Close,
    ]
}

fn rounded_clip_path(
    style: &BackgroundStyle,
    border_box: BackgroundRect,
    kind: BackgroundBox,
    dpr: f64,
) -> Option<Vec<PathCommand>> {
    let corners = style.resolved_corners_for_box(kind, border_box);
    corners
        .iter()
        .any(|&(rx, ry)| rx > 0.0 || ry > 0.0)
        .then(|| {
            let rect = style.painting_box(kind, border_box);
            rounded_rect_path(
                BackgroundRect {
                    x: rect.x * dpr,
                    y: rect.y * dpr,
                    width: rect.width * dpr,
                    height: rect.height * dpr,
                },
                corners.map(|(rx, ry)| (rx * dpr, ry * dpr)),
            )
        })
}

fn clip_path_for_node(
    node: &PositionedNode,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    dpr: f64,
) -> Option<Vec<PathCommand>> {
    rounded_clip_path(
        &BackgroundStyle::from_positioned(node),
        BackgroundRect {
            x,
            y,
            width: width.max(0.0),
            height: height.max(0.0),
        },
        BackgroundBox::Border,
        dpr,
    )
}

fn paint_gradient_stops(stops: &[layout_backgrounds::GradientStop]) -> Vec<GradientStop> {
    let last = stops.len().saturating_sub(1).max(1) as f64;
    stops
        .iter()
        .enumerate()
        .map(|(index, stop)| GradientStop {
            offset: stop.offset.unwrap_or(index as f64 / last).clamp(0.0, 1.0),
            color: background_color_to_css(stop.color),
        })
        .collect()
}

fn background_color_to_css(color: BackgroundColor) -> String {
    color_to_css(Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    })
}
fn background_id(
    node: &PositionedNode,
    layer_index: usize,
    tile_index: usize,
    source: &str,
    rect: BackgroundRect,
) -> String {
    let hash = source.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    });
    format!(
        "background-{}-{layer_index}-{tile_index}-{:x}-{:x}-{:x}-{:x}-{hash:x}",
        node.id.as_deref().unwrap_or("anonymous"),
        rect.x.to_bits(),
        rect.y.to_bits(),
        rect.width.to_bits(),
        rect.height.to_bits(),
    )
}

fn read_color(m: &HashMap<String, ExtValue>, key: &str) -> Option<Color> {
    match m.get(key) {
        Some(ExtValue::Map(inner)) => {
            let r = read_byte(inner, "r")?;
            let g = read_byte(inner, "g")?;
            let b = read_byte(inner, "b")?;
            let a = read_byte(inner, "a").unwrap_or(255);
            Some(Color { r, g, b, a })
        }
        _ => None,
    }
}

fn read_byte(m: &HashMap<String, ExtValue>, key: &str) -> Option<u8> {
    match m.get(key)? {
        ExtValue::Int(v) => Some((*v).clamp(0, 255) as u8),
        ExtValue::Float(v) => Some((v.round() as i64).clamp(0, 255) as u8),
        _ => None,
    }
}

fn read_float(m: &HashMap<String, ExtValue>, key: &str) -> Option<f64> {
    match m.get(key)? {
        ExtValue::Float(v) => Some(*v),
        ExtValue::Int(v) => Some(*v as f64),
        _ => None,
    }
}

fn read_string<'a>(m: &'a HashMap<String, ExtValue>, key: &str) -> Option<&'a str> {
    match m.get(key)? {
        ExtValue::Str(value) => Some(value),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Text content — shape + emit PaintGlyphRun per wrapped line
// ═══════════════════════════════════════════════════════════════════════════

// Text shaping needs the content plus independent layout-box geometry and the
// generic shaper/measure/render hooks; each argument is semantically distinct.
#[allow(clippy::too_many_arguments)]
fn emit_text_content<S, M, R>(
    tc: &TextContent,
    box_x: f64,
    box_y: f64,
    box_width: f64,
    dpr: f64,
    direction: BaseDirection,
    options: &LayoutToPaintOptions<'_, S, M, R>,
    font_cache: &mut HashMap<FontCacheKey, CachedFont<S::Handle>>,
    out: &mut Vec<PaintInstruction>,
) where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    if tc.value.is_empty() {
        return;
    }

    let key = FontCacheKey::from(&tc.font);
    let cached = font_cache.entry(key.clone()).or_insert_with(|| {
        let query = query_from_font(&tc.font);
        let handle = options.resolver.resolve(&query).ok();
        CachedFont {
            handle,
            font_ref_template: None,
        }
    });

    let handle = match &cached.handle {
        Some(h) => h,
        None => return, // resolver failed; silently drop content for v1
    };

    // Scaled size to device pixels.
    let size_dpr = (tc.font.size * dpr) as f32;
    // Line height in device pixels.
    let line_height_dpr = compute_line_height_dpr(options.metrics, handle, &tc.font, dpr);
    // Ascent (baseline offset from top) in device pixels.
    let ascent_dpr = compute_ascent_dpr(options.metrics, handle, &tc.font, dpr);

    let max_width_dpr = box_width * dpr;
    // First baseline y, in device-pixel scene coordinates.
    let box_x_dpr = box_x * dpr;
    let box_y_dpr = box_y * dpr;
    let mut baseline_y = box_y_dpr + ascent_dpr;

    let fill_css = color_to_css(tc.color);

    for segment in tc.value.split('\n') {
        let wrapped = if tc.wrap {
            wrap_line(
                options.shaper,
                handle,
                segment,
                size_dpr,
                max_width_dpr,
                direction,
            )
        } else {
            vec![segment.to_string()]
        };
        for line in wrapped {
            // Shape the line once. This gives us total_advance for
            // alignment AND the glyph IDs/positions for emission.
            if line.is_empty() {
                baseline_y += line_height_dpr;
                continue;
            }
            let shaped = match shape_visual_line(options.shaper, handle, &line, size_dpr, direction)
            {
                Ok(s) => s,
                Err(_) => {
                    baseline_y += line_height_dpr;
                    continue;
                }
            };

            // Compute the starting x position based on text alignment.
            let line_advance = shaped.total_advance() as f64;
            let baseline_x = match tc.text_align {
                TextAlign::Center => box_x_dpr + (max_width_dpr - line_advance) / 2.0,
                TextAlign::End => box_x_dpr + max_width_dpr - line_advance,
                TextAlign::Start => box_x_dpr,
            };

            emit_glyph_runs_from_shaped(&shaped, size_dpr, baseline_x, baseline_y, &fill_css, out);
            emit_text_decorations(
                tc,
                handle,
                options.metrics,
                &shaped,
                size_dpr,
                baseline_x,
                baseline_y,
                ascent_dpr,
                dpr,
                out,
            );
            baseline_y += line_height_dpr;
        }
    }
}

fn shape_visual_line<S: TextShaper>(
    shaper: &S,
    handle: &S::Handle,
    line: &str,
    size: f32,
    direction: BaseDirection,
) -> Result<ShapedText, text_interfaces::ShapingError> {
    let flow = TextFlow::analyze(line, direction);
    let mut runs = Vec::new();
    for index in flow.visual_run_order {
        let run = &flow.logical_runs[index];
        let options = ShapeOptions {
            direction: match run.direction {
                FlowDirection::Ltr => Direction::Ltr,
                FlowDirection::Rtl => Direction::Rtl,
            },
            ..ShapeOptions::default()
        };
        runs.extend(
            shaper
                .shape(&line[run.bytes.clone()], handle, size, &options)?
                .runs,
        );
    }
    Ok(ShapedText { runs })
}

#[allow(clippy::too_many_arguments)]
fn emit_text_decorations<M>(
    tc: &TextContent,
    handle: &M::Handle,
    metrics: &M,
    shaped: &ShapedText,
    size: f32,
    x: f64,
    baseline_y: f64,
    ascent: f64,
    dpr: f64,
    out: &mut Vec<PaintInstruction>,
) where
    M: FontMetrics,
{
    let Some(decoration) = tc.decoration else {
        return;
    };
    let width = shaped.total_advance() as f64;
    if width <= 0.0 {
        return;
    }

    let units_per_em = f64::from(metrics.units_per_em(handle).max(1));
    let scale = f64::from(size) / units_per_em;
    let thickness = metrics
        .underline_thickness(handle)
        .map(|value| f64::from(value.max(1)) * scale)
        .unwrap_or_else(|| (f64::from(size) * 0.05).max(dpr));
    let color = color_to_css(decoration.color.unwrap_or(tc.color));
    let mut emit_line = |y: f64| {
        out.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x,
            y,
            width,
            height: thickness,
            fill: Some(color.clone()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
    };

    if decoration.lines.contains(TextDecorationLines::UNDERLINE) {
        let position = metrics
            .underline_position(handle)
            .map(|value| f64::from(value) * scale)
            .unwrap_or_else(|| f64::from(size) * 0.08);
        emit_line(baseline_y + position);
    }
    if decoration.lines.contains(TextDecorationLines::OVERLINE) {
        emit_line(baseline_y - ascent);
    }
    if decoration.lines.contains(TextDecorationLines::LINE_THROUGH) {
        let x_height = metrics
            .x_height(handle)
            .map(|value| f64::from(value) * scale)
            .unwrap_or(ascent * 0.5);
        emit_line(baseline_y - x_height * 0.5);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Emit PaintGlyphRun instructions from a pre-shaped ShapedText
// ═══════════════════════════════════════════════════════════════════════════

/// Emit one `PaintGlyphRun` per `ShapedRun` in `shaped`, starting the pen
/// at `(baseline_x, baseline_y)` in device-pixel scene coordinates.
fn emit_glyph_runs_from_shaped(
    shaped: &ShapedText,
    size: f32,
    baseline_x: f64,
    baseline_y: f64,
    fill_css: &str,
    out: &mut Vec<PaintInstruction>,
) {
    // The shaper returns potentially MULTIPLE ShapedRuns, one per
    // font-fallback segment. Each must become its own PaintGlyphRun
    // so the paint backend can route on the segment's actual font_ref
    // and pick up the correct fallback font. The line's pen
    // accumulates across segments.
    let mut line_pen_x: f64 = 0.0;
    let mut line_pen_y: f64 = 0.0;

    for run in &shaped.runs {
        if run.glyphs.is_empty() {
            continue;
        }

        // Glyph positions are absolute scene coordinates. Each
        // segment's glyphs carry x_offset / y_offset relative to the
        // segment's start, so we add the line-level pen on top.
        let mut positions: Vec<GlyphPosition> = Vec::with_capacity(run.glyphs.len());
        let mut seg_pen_x: f64 = 0.0;
        let mut seg_pen_y: f64 = 0.0;
        for g in &run.glyphs {
            let gx = baseline_x + line_pen_x + seg_pen_x + g.x_offset as f64;
            let gy = baseline_y + line_pen_y + seg_pen_y + g.y_offset as f64;
            positions.push(GlyphPosition {
                glyph_id: g.glyph_id,
                x: gx,
                y: gy,
            });
            seg_pen_x += g.x_advance as f64;
            seg_pen_y += g.y_advance as f64;
        }

        out.push(PaintInstruction::GlyphRun(PaintGlyphRun {
            base: PaintBase::default(),
            glyphs: positions,
            font_ref: run.font_ref.clone(),
            font_size: size as f64,
            fill: Some(fill_css.to_string()),
        }));

        // Advance the line-level pen by this segment's total advance
        // so the next segment starts where this one ended.
        line_pen_x += run.x_advance_total as f64;
        line_pen_y += run.glyphs.iter().map(|g| g.y_advance as f64).sum::<f64>();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Greedy word-wrap (mirrors the layout-text-measure-native algorithm)
// ═══════════════════════════════════════════════════════════════════════════

fn wrap_line<S: TextShaper>(
    shaper: &S,
    handle: &S::Handle,
    segment: &str,
    size: f32,
    max_width: f64,
    direction: BaseDirection,
) -> Vec<String> {
    if segment.is_empty() {
        return vec![String::new()];
    }
    if max_width <= 0.0 {
        return vec![segment.to_string()];
    }

    // Preserve source whitespace for fixed-format text when it already fits.
    // The greedy wrapper below intentionally collapses whitespace for paragraph
    // text, but ASCII art/code-like content should not be rewritten just because
    // it passed through UI04.
    if let Ok(shaped) = shape_visual_line(shaper, handle, segment, size, direction) {
        if shaped.total_advance() as f64 <= max_width {
            return vec![segment.to_string()];
        }
    }

    let space_width = shape_visual_line(shaper, handle, " ", size, direction)
        .map(|r| r.total_advance() as f64)
        .unwrap_or((size as f64) * 0.25);

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width: f64 = 0.0;

    for piece in paint_wrap_pieces(segment, direction) {
        let word_width = shape_visual_line(shaper, handle, piece.value, size, direction)
            .map(|r| r.total_advance() as f64)
            .unwrap_or(piece.value.chars().count() as f64 * (size as f64) * 0.5);
        let gap = if piece.leading_space && !current.is_empty() {
            space_width
        } else {
            0.0
        };

        if current.is_empty() {
            current.push_str(piece.value);
            current_width = word_width;
        } else if current_width + gap + word_width <= max_width {
            if gap > 0.0 {
                current.push(' ');
            }
            current.push_str(piece.value);
            current_width += gap + word_width;
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(piece.value);
            current_width = word_width;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

struct PaintWrapPiece<'a> {
    value: &'a str,
    leading_space: bool,
}

fn paint_wrap_pieces(segment: &str, direction: BaseDirection) -> Vec<PaintWrapPiece<'_>> {
    let flow = TextFlow::analyze(segment, direction);
    let mut boundaries: Vec<_> = flow
        .breaks
        .iter()
        .filter(|opportunity| opportunity.kind == text_flow::BreakKind::Allowed)
        .map(|opportunity| opportunity.byte_index)
        .collect();
    if boundaries.last().copied() != Some(segment.len()) {
        boundaries.push(segment.len());
    }
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut pending_space = false;
    for end in boundaries {
        let source = &segment[start..end];
        start = end;
        let value = source.trim_matches(char::is_whitespace);
        if value.is_empty() {
            pending_space = true;
            continue;
        }
        pieces.push(PaintWrapPiece {
            value,
            leading_space: pending_space || source.chars().next().is_some_and(char::is_whitespace),
        });
        pending_space = source.chars().last().is_some_and(char::is_whitespace);
    }
    pieces
}

// ═══════════════════════════════════════════════════════════════════════════
// Metrics helpers — produce device-pixel ascent / line-height
// ═══════════════════════════════════════════════════════════════════════════

fn compute_line_height_dpr<M: FontMetrics>(
    metrics: &M,
    handle: &M::Handle,
    font: &FontSpec,
    dpr: f64,
) -> f64 {
    let upem = metrics.units_per_em(handle) as f64;
    if upem <= 0.0 {
        return font.size * font.line_height * dpr;
    }
    let ascent = metrics.ascent(handle) as f64;
    let descent = metrics.descent(handle) as f64;
    let line_gap = metrics.line_gap(handle) as f64;

    let size_dpr = font.size * dpr;
    let raw = (ascent + descent + line_gap) * size_dpr / upem;
    raw.max(size_dpr) * font.line_height.max(1.0) / 1.2
}

fn compute_ascent_dpr<M: FontMetrics>(
    metrics: &M,
    handle: &M::Handle,
    font: &FontSpec,
    dpr: f64,
) -> f64 {
    let upem = metrics.units_per_em(handle) as f64;
    if upem <= 0.0 {
        // Fallback: ~0.8 em as a generic ascent
        return font.size * dpr * 0.8;
    }
    metrics.ascent(handle) as f64 * (font.size * dpr) / upem
}

// ═══════════════════════════════════════════════════════════════════════════
// FontQuery conversion + cache key
// ═══════════════════════════════════════════════════════════════════════════

fn query_from_font(font: &FontSpec) -> FontQuery {
    let family = if font.family.is_empty() {
        "Helvetica".to_string() // matches layout-text-measure-native's choice
    } else {
        font.family.clone()
    };
    FontQuery {
        family_names: vec![family],
        weight: FontWeight(font.weight),
        style: if font.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
        stretch: FontStretch::Normal,
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
struct FontCacheKey {
    family: String,
    weight: u16,
    italic: bool,
}

impl From<&FontSpec> for FontCacheKey {
    fn from(f: &FontSpec) -> Self {
        Self {
            family: if f.family.is_empty() {
                "Helvetica".into()
            } else {
                f.family.clone()
            },
            weight: f.weight,
            italic: f.italic,
        }
    }
}

struct CachedFont<H> {
    handle: Option<H>,
    // Reserved for future use — pre-computed font_ref template when
    // we eventually want to avoid re-calling shaper.font_ref() per run.
    #[allow(dead_code)]
    font_ref_template: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// Color → CSS string
// ═══════════════════════════════════════════════════════════════════════════

fn color_to_css(c: Color) -> String {
    if c.a == 255 {
        format!("rgb({}, {}, {})", c.r, c.g, c.b)
    } else {
        let alpha = c.a as f64 / 255.0;
        format!("rgba({}, {}, {}, {:.4})", c.r, c.g, c.b, alpha)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use layout_ir::{
        color_black, color_white, font_spec, rgb, TextAlign, TextContent, TextDecoration,
    };
    use layout_positioned::{Overflow, PositionedStyle};

    fn color_ext(r: u8, g: u8, b: u8) -> ExtValue {
        ExtValue::Map(HashMap::from([
            ("r".into(), ExtValue::Int(i64::from(r))),
            ("g".into(), ExtValue::Int(i64::from(g))),
            ("b".into(), ExtValue::Int(i64::from(b))),
            ("a".into(), ExtValue::Int(255)),
        ]))
    }
    use text_interfaces::{
        Direction, FontResolutionError, Glyph, ShapedRun, ShapedText, ShapingError,
    };

    // ─── Minimal in-memory backend for the tests ────────────────

    struct FakeResolver;
    impl FontResolver for FakeResolver {
        type Handle = FakeHandle;
        fn resolve(&self, _q: &FontQuery) -> Result<Self::Handle, FontResolutionError> {
            Ok(FakeHandle)
        }
    }

    struct FailingResolver;
    impl FontResolver for FailingResolver {
        type Handle = FakeHandle;
        fn resolve(&self, _q: &FontQuery) -> Result<Self::Handle, FontResolutionError> {
            Err(FontResolutionError::NoFamilyFound)
        }
    }

    #[derive(Clone)]
    struct FakeHandle;

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

    /// Shaper: every character is (size / 2) wide at 0 y-offset.
    /// For the simple tests below, the whole input is one font
    /// binding ("fake:test") → one ShapedRun.
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

    /// Shaper that simulates font fallback: splits the input on `→`
    /// (U+2192) and emits a separate ShapedRun for each side, tagged
    /// with a different font_ref. Used to verify layout-to-paint
    /// emits one PaintGlyphRun per segment with the correct font_ref.
    struct FallbackSplittingShaper;
    impl TextShaper for FallbackSplittingShaper {
        type Handle = FakeHandle;
        fn shape(
            &self,
            text: &str,
            _font: &FakeHandle,
            size: f32,
            _opts: &ShapeOptions,
        ) -> Result<ShapedText, ShapingError> {
            let advance = size / 2.0;
            let mut runs: Vec<ShapedRun> = Vec::new();
            let mut current = String::new();
            let mut current_font = "fake:primary";
            let mut cluster: u32 = 0;

            let push = |runs: &mut Vec<ShapedRun>, s: &str, font: &str, start_cluster: u32| {
                if s.is_empty() {
                    return;
                }
                let glyphs: Vec<Glyph> = s
                    .chars()
                    .enumerate()
                    .map(|(i, c)| Glyph {
                        glyph_id: c as u32,
                        cluster: start_cluster + i as u32,
                        x_advance: advance,
                        y_advance: 0.0,
                        x_offset: 0.0,
                        y_offset: 0.0,
                    })
                    .collect();
                let total = glyphs.len() as f32 * advance;
                runs.push(ShapedRun {
                    glyphs,
                    x_advance_total: total,
                    font_ref: font.into(),
                });
            };

            for ch in text.chars() {
                let needs_fallback = ch == '\u{2192}';
                let want_font = if needs_fallback {
                    "fake:fallback"
                } else {
                    "fake:primary"
                };
                if want_font != current_font && !current.is_empty() {
                    let start = cluster - current.chars().count() as u32;
                    push(&mut runs, &current, current_font, start);
                    current.clear();
                }
                current_font = want_font;
                current.push(ch);
                cluster += 1;
            }
            let start = cluster - current.chars().count() as u32;
            push(&mut runs, &current, current_font, start);
            Ok(ShapedText { runs })
        }

        fn font_ref(&self, _h: &FakeHandle) -> String {
            "fake:primary".into()
        }
    }

    fn make_options<'a>(
        shaper: &'a FakeShaper,
        metrics: &'a FakeMetrics,
        resolver: &'a FakeResolver,
    ) -> LayoutToPaintOptions<'a, FakeShaper, FakeMetrics, FakeResolver> {
        LayoutToPaintOptions {
            width: 800.0,
            height: 600.0,
            background: color_white(),
            device_pixel_ratio: 1.0,
            shaper,
            metrics,
            resolver,
        }
    }

    fn text_content(value: &str) -> TextContent {
        TextContent {
            value: value.into(),
            font: font_spec("Test", 16.0),
            color: color_black(),
            decoration: None,
            max_lines: None,
            wrap: true,
            text_align: TextAlign::Start,
        }
    }

    fn positioned_leaf(tc: TextContent, x: f64, y: f64, w: f64, h: f64) -> PositionedNode {
        PositionedNode {
            x,
            y,
            width: w,
            height: h,
            id: None,
            content: Some(Content::Text(tc)),
            children: Vec::new(),
            ext: HashMap::new(),
        }
    }

    fn positioned_container(children: Vec<PositionedNode>, w: f64, h: f64) -> PositionedNode {
        PositionedNode {
            x: 0.0,
            y: 0.0,
            width: w,
            height: h,
            id: None,
            content: None,
            children,
            ext: HashMap::new(),
        }
    }

    // ─── Tests ──────────────────────────────────────────────────

    #[test]
    fn empty_container_produces_empty_scene() {
        let root = positioned_container(Vec::new(), 800.0, 600.0);
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_options(&shaper, &metrics, &resolver);
        let scene = layout_to_paint(&root, &opts);
        assert_eq!(scene.width, 800.0);
        assert_eq!(scene.height, 600.0);
        assert!(scene.instructions.is_empty());
    }

    #[test]
    fn underline_uses_shaped_width_color_and_device_pixel_floor() {
        let mut text = text_content("link");
        text.color = rgb(85, 26, 139);
        text.decoration = Some(TextDecoration::underline());
        let root = positioned_leaf(text, 3.0, 4.0, 200.0, 30.0);
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let mut options = make_options(&shaper, &metrics, &resolver);
        options.device_pixel_ratio = 2.0;

        let scene = layout_to_paint(&root, &options);
        let glyph = scene
            .instructions
            .iter()
            .find_map(|instruction| match instruction {
                PaintInstruction::GlyphRun(run) => Some(run),
                _ => None,
            })
            .expect("decorated text should still emit glyphs");
        let underline = scene
            .instructions
            .iter()
            .find_map(|instruction| match instruction {
                PaintInstruction::Rect(rect)
                    if rect.fill.as_deref() == Some("rgb(85, 26, 139)") =>
                {
                    Some(rect)
                }
                _ => None,
            })
            .expect("underline should be a backend-independent paint rectangle");

        assert_eq!(glyph.fill.as_deref(), Some("rgb(85, 26, 139)"));
        assert_eq!(underline.x, 6.0);
        assert_eq!(underline.width, 64.0);
        assert_eq!(underline.height, 2.0);
        assert!(underline.y > glyph.glyphs[0].y);
    }

    #[test]
    fn background_color_emits_rect() {
        let mut ext = HashMap::new();
        let mut paint = HashMap::new();
        paint.insert(
            "backgroundColor".to_string(),
            ExtValue::Map({
                let mut m = HashMap::new();
                m.insert("r".to_string(), ExtValue::Int(200));
                m.insert("g".to_string(), ExtValue::Int(200));
                m.insert("b".to_string(), ExtValue::Int(200));
                m.insert("a".to_string(), ExtValue::Int(255));
                m
            }),
        );
        ext.insert("paint".to_string(), ExtValue::Map(paint));

        let root = PositionedNode {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
            id: None,
            content: None,
            children: Vec::new(),
            ext,
        };

        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_options(&shaper, &metrics, &resolver);
        let scene = layout_to_paint(&root, &opts);

        assert_eq!(scene.instructions.len(), 1);
        match &scene.instructions[0] {
            PaintInstruction::Rect(r) => {
                assert_eq!(r.width, 100.0);
                assert_eq!(r.height, 50.0);
                assert_eq!(r.fill, Some("rgb(200, 200, 200)".into()));
            }
            other => panic!("expected Rect, got {:?}", other),
        }
    }

    #[test]
    fn text_content_emits_glyph_run() {
        let leaf = positioned_leaf(text_content("Hello"), 10.0, 20.0, 500.0, 20.0);
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_options(&shaper, &metrics, &resolver);
        let scene = layout_to_paint(&leaf, &opts);

        // Expect exactly one PaintGlyphRun with 5 glyphs.
        assert_eq!(scene.instructions.len(), 1);
        match &scene.instructions[0] {
            PaintInstruction::GlyphRun(gr) => {
                assert_eq!(gr.glyphs.len(), 5);
                assert_eq!(gr.font_ref, "fake:test");
                assert_eq!(gr.fill, Some("rgb(0, 0, 0)".into()));
                // First glyph x at baseline_x == 10, y at baseline
                // which is 20 + ascent = 20 + (800/1000 * 16) = 32.8
                assert_eq!(gr.glyphs[0].x, 10.0);
                assert!((gr.glyphs[0].y - 32.8).abs() < 1e-6);
                // Advances: each char = size/2 = 8 px. Glyph i at x=10+8i
                for (i, g) in gr.glyphs.iter().enumerate() {
                    assert!((g.x - (10.0 + i as f64 * 8.0)).abs() < 1e-6);
                }
            }
            other => panic!("expected GlyphRun, got {:?}", other),
        }
    }

    #[test]
    fn fixed_format_text_preserves_spaces_when_line_fits() {
        let leaf = positioned_leaf(text_content("  / \\  "), 0.0, 0.0, 500.0, 20.0);
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_options(&shaper, &metrics, &resolver);
        let scene = layout_to_paint(&leaf, &opts);

        match &scene.instructions[0] {
            PaintInstruction::GlyphRun(gr) => {
                let glyph_ids: Vec<u32> = gr.glyphs.iter().map(|g| g.glyph_id).collect();
                assert_eq!(
                    glyph_ids,
                    vec![
                        ' ' as u32,
                        ' ' as u32,
                        '/' as u32,
                        ' ' as u32,
                        '\\' as u32,
                        ' ' as u32,
                        ' ' as u32,
                    ]
                );
            }
            other => panic!("expected GlyphRun, got {:?}", other),
        }
    }

    #[test]
    fn hard_newline_produces_multiple_glyph_runs() {
        let leaf = positioned_leaf(text_content("line one\nline two"), 0.0, 0.0, 500.0, 40.0);
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_options(&shaper, &metrics, &resolver);
        let scene = layout_to_paint(&leaf, &opts);

        let glyph_runs: Vec<&PaintGlyphRun> = scene
            .instructions
            .iter()
            .filter_map(|i| match i {
                PaintInstruction::GlyphRun(g) => Some(g),
                _ => None,
            })
            .collect();
        assert_eq!(glyph_runs.len(), 2);
        assert_eq!(glyph_runs[0].glyphs.len(), 8); // "line one"
        assert_eq!(glyph_runs[1].glyphs.len(), 8); // "line two"
                                                   // Second line's baseline should be strictly greater than the first.
        assert!(glyph_runs[1].glyphs[0].y > glyph_runs[0].glyphs[0].y);
    }

    #[test]
    fn text_wraps_when_exceeding_box_width() {
        // "aa bb cc dd ee ff" = 6 words × 2 chars + spaces.
        // At 16px each char = 8, each space = 8. Two-word line = "aa bb" = 2+1+2 chars = 5 × 8 = 40px.
        // max_width = 40 → two words per line, three lines total.
        let leaf = positioned_leaf(text_content("aa bb cc dd ee ff"), 0.0, 0.0, 40.0, 60.0);
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_options(&shaper, &metrics, &resolver);
        let scene = layout_to_paint(&leaf, &opts);

        let runs: Vec<&PaintGlyphRun> = scene
            .instructions
            .iter()
            .filter_map(|i| match i {
                PaintInstruction::GlyphRun(g) => Some(g),
                _ => None,
            })
            .collect();
        assert_eq!(runs.len(), 3);
    }

    #[test]
    fn emergency_wrapper_uses_cjk_break_opportunities() {
        let leaf = positioned_leaf(text_content("日本語文"), 0.0, 0.0, 16.0, 40.0);
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_options(&shaper, &metrics, &resolver);
        let scene = layout_to_paint(&leaf, &opts);
        let runs: Vec<_> = scene
            .instructions
            .iter()
            .filter(|instruction| matches!(instruction, PaintInstruction::GlyphRun(_)))
            .collect();
        assert_eq!(runs.len(), 2);
    }

    #[test]
    fn text_can_disable_soft_wrapping() {
        let mut content = text_content("aa bb cc dd ee ff");
        content.wrap = false;
        let leaf = positioned_leaf(content, 0.0, 0.0, 40.0, 20.0);
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_options(&shaper, &metrics, &resolver);
        let scene = layout_to_paint(&leaf, &opts);

        assert_eq!(
            scene
                .instructions
                .iter()
                .filter(|instruction| matches!(instruction, PaintInstruction::GlyphRun(_)))
                .count(),
            1
        );
    }

    #[test]
    fn absolute_positioning_accumulates_through_nesting() {
        // Outer at (10, 20); inner at (5, 7); leaf at (0, 0) inside inner.
        let leaf = positioned_leaf(text_content("A"), 0.0, 0.0, 100.0, 16.0);
        let inner = PositionedNode {
            x: 5.0,
            y: 7.0,
            width: 100.0,
            height: 16.0,
            id: None,
            content: None,
            children: vec![leaf],
            ext: HashMap::new(),
        };
        let outer = PositionedNode {
            x: 10.0,
            y: 20.0,
            width: 120.0,
            height: 40.0,
            id: None,
            content: None,
            children: vec![inner],
            ext: HashMap::new(),
        };

        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_options(&shaper, &metrics, &resolver);
        let scene = layout_to_paint(&outer, &opts);

        match &scene.instructions[0] {
            PaintInstruction::GlyphRun(gr) => {
                // abs_x = 10 + 5 + 0 = 15
                assert_eq!(gr.glyphs[0].x, 15.0);
                // abs_y = 20 + 7 + 0, baseline = abs_y + ascent = 27 + 12.8 = 39.8
                assert!((gr.glyphs[0].y - 39.8).abs() < 1e-6);
            }
            _ => panic!("expected GlyphRun"),
        }
    }

    #[test]
    fn overflow_and_viewport_positioning_emit_backend_neutral_groups() {
        let leaf = positioned_leaf(text_content("clipped"), 80.0, 0.0, 80.0, 16.0);
        let mut root = PositionedNode {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 30.0,
            id: None,
            content: None,
            children: vec![leaf],
            ext: HashMap::new(),
        };
        root.ext.insert(
            "positioned".into(),
            PositionedStyle {
                overflow_x: Overflow::Hidden,
                overflow_y: Overflow::Hidden,
                ..Default::default()
            }
            .to_ext(),
        );

        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let scene = layout_to_paint(&root, &make_options(&shaper, &metrics, &resolver));
        let PaintInstruction::Clip(clip) = &scene.instructions[0] else {
            panic!("expected overflow clip");
        };
        assert_eq!(
            (clip.x, clip.y, clip.width, clip.height),
            (0.0, 0.0, 100.0, 30.0)
        );
        assert!(matches!(clip.children[0], PaintInstruction::GlyphRun(_)));
    }

    #[test]
    fn device_pixel_ratio_scales_everything() {
        let leaf = positioned_leaf(text_content("A"), 10.0, 10.0, 100.0, 16.0);
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let mut opts = make_options(&shaper, &metrics, &resolver);
        opts.device_pixel_ratio = 2.0;
        let scene = layout_to_paint(&leaf, &opts);

        assert_eq!(scene.width, 800.0 * 2.0);
        assert_eq!(scene.height, 600.0 * 2.0);
        match &scene.instructions[0] {
            PaintInstruction::GlyphRun(gr) => {
                // font_size in scene coordinates
                assert_eq!(gr.font_size, 32.0);
                // baseline x = 10 × 2 = 20
                assert_eq!(gr.glyphs[0].x, 20.0);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn failing_resolver_drops_text_silently() {
        let leaf = positioned_leaf(text_content("Hello"), 0.0, 0.0, 100.0, 16.0);
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FailingResolver;
        let opts: LayoutToPaintOptions<'_, _, _, _> = LayoutToPaintOptions {
            width: 800.0,
            height: 600.0,
            background: color_white(),
            device_pixel_ratio: 1.0,
            shaper: &shaper,
            metrics: &metrics,
            resolver: &resolver,
        };
        let scene = layout_to_paint(&leaf, &opts);
        // No glyph run emitted. The scene is technically valid but
        // silent — the paint backend renders background only.
        let runs: Vec<_> = scene
            .instructions
            .iter()
            .filter(|i| matches!(i, PaintInstruction::GlyphRun(_)))
            .collect();
        assert!(runs.is_empty());
    }

    #[test]
    fn color_to_css_handles_alpha() {
        assert_eq!(
            color_to_css(Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255
            }),
            "rgb(255, 0, 0)"
        );
        let half = color_to_css(Color {
            r: 0,
            g: 0,
            b: 0,
            a: 128,
        });
        assert!(half.starts_with("rgba(0, 0, 0, 0.50"));
    }

    #[test]
    fn image_content_emits_paint_image() {
        use layout_ir::ImageContent;
        let img = ImageContent {
            src: "file:///logo.png".into(),
            fit: layout_ir::ImageFit::Contain,
        };
        let leaf = PositionedNode {
            x: 10.0,
            y: 10.0,
            width: 64.0,
            height: 64.0,
            id: None,
            content: Some(Content::Image(img)),
            children: Vec::new(),
            ext: HashMap::new(),
        };

        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_options(&shaper, &metrics, &resolver);
        let scene = layout_to_paint(&leaf, &opts);
        assert_eq!(scene.instructions.len(), 1);
        match &scene.instructions[0] {
            PaintInstruction::Image(i) => {
                match &i.src {
                    ImageSrc::Uri(s) => assert_eq!(s, "file:///logo.png"),
                    _ => panic!("expected Uri src"),
                }
                assert_eq!(i.x, 10.0);
                assert_eq!(i.y, 10.0);
                assert_eq!(i.width, 64.0);
                assert_eq!(i.height, 64.0);
            }
            _ => panic!("expected PaintImage"),
        }
    }

    #[test]
    fn cover_image_uses_intrinsic_ratio_and_clips_to_replaced_box() {
        use layout_ir::ImageContent;
        let mut ext = HashMap::new();
        ext.insert(
            "replaced".into(),
            layout_replaced::replaced_ext(Some(200.0), Some(100.0), None),
        );
        let leaf = PositionedNode {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 100.0,
            id: None,
            content: Some(Content::Image(ImageContent {
                src: "file:///cover.png".into(),
                fit: layout_ir::ImageFit::Cover,
            })),
            children: Vec::new(),
            ext,
        };
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let scene = layout_to_paint(&leaf, &make_options(&shaper, &metrics, &resolver));
        let [PaintInstruction::Clip(clip)] = scene.instructions.as_slice() else {
            panic!("cover image should emit a content-box clip");
        };
        assert_eq!(
            (clip.x, clip.y, clip.width, clip.height),
            (10.0, 20.0, 100.0, 100.0)
        );
        let [PaintInstruction::Image(image)] = clip.children.as_slice() else {
            panic!("clip should contain one fitted image");
        };
        assert_eq!(
            (image.x, image.y, image.width, image.height),
            (-40.0, 20.0, 200.0, 100.0)
        );
    }

    #[test]
    fn font_resolution_is_cached_across_nodes() {
        // Two sibling TextContent nodes with the same font should
        // only invoke the resolver once. We verify indirectly via
        // a wrapper resolver that counts resolve() calls.
        struct CountingResolver {
            count: std::cell::Cell<usize>,
        }
        impl FontResolver for CountingResolver {
            type Handle = FakeHandle;
            fn resolve(&self, _q: &FontQuery) -> Result<Self::Handle, FontResolutionError> {
                self.count.set(self.count.get() + 1);
                Ok(FakeHandle)
            }
        }

        let a = positioned_leaf(text_content("one"), 0.0, 0.0, 500.0, 16.0);
        let b = positioned_leaf(text_content("two"), 0.0, 20.0, 500.0, 16.0);
        let root = positioned_container(vec![a, b], 500.0, 40.0);

        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let counting = CountingResolver {
            count: std::cell::Cell::new(0),
        };

        let opts: LayoutToPaintOptions<'_, _, _, CountingResolver> = LayoutToPaintOptions {
            width: 500.0,
            height: 40.0,
            background: color_white(),
            device_pixel_ratio: 1.0,
            shaper: &shaper,
            metrics: &metrics,
            resolver: &counting,
        };
        let _scene = layout_to_paint(&root, &opts);
        assert_eq!(counting.count.get(), 1);
    }

    #[test]
    fn visual_effects_wrap_each_node_in_one_isolated_layer() {
        let mut leaf = positioned_leaf(text_content("effect"), 10.0, 20.0, 80.0, 30.0);
        let effects = EffectStyle {
            opacity: 0.5,
            transform: layout_effects::translation(6.0, 4.0),
            transform_origin: layout_effects::TransformOrigin {
                x: layout_effects::OriginComponent::percent(0.0),
                y: layout_effects::OriginComponent::percent(0.0),
            },
            filters: vec![EffectFilter::Blur { radius: 2.0 }],
            blend_mode: EffectBlendMode::Multiply,
            isolation: true,
        };
        leaf.ext.insert("effects".into(), effects.to_ext());
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let scene = layout_to_paint(&leaf, &make_options(&shaper, &metrics, &resolver));

        let [PaintInstruction::Layer(layer)] = scene.instructions.as_slice() else {
            panic!("effect node must become one isolated layer");
        };
        assert_eq!(layer.opacity, Some(0.5));
        assert_eq!(layer.blend_mode, Some(BlendMode::Multiply));
        assert_eq!(layer.transform, Some([1.0, 0.0, 0.0, 1.0, 6.0, 4.0]));
        assert_eq!(
            layer.filters,
            Some(vec![FilterEffect::Blur { radius: 2.0 }])
        );
        assert!(matches!(
            layer.children.as_slice(),
            [PaintInstruction::GlyphRun(_)]
        ));
    }

    #[test]
    fn layered_gradients_and_images_emit_clipped_backend_neutral_paint() {
        let mut leaf = positioned_leaf(text_content("background"), 4.0, 6.0, 80.0, 40.0);
        let style = BackgroundStyle {
            layers: vec![
                layout_backgrounds::BackgroundLayer::new(BackgroundSource::LinearGradient {
                    angle_degrees: 90.0,
                    stops: vec![
                        layout_backgrounds::GradientStop {
                            color: BackgroundColor {
                                r: 255,
                                g: 0,
                                b: 0,
                                a: 255,
                            },
                            offset: Some(0.0),
                        },
                        layout_backgrounds::GradientStop {
                            color: BackgroundColor {
                                r: 0,
                                g: 0,
                                b: 255,
                                a: 255,
                            },
                            offset: Some(1.0),
                        },
                    ],
                }),
                layout_backgrounds::BackgroundLayer {
                    size: BackgroundSize::Explicit {
                        width: Some(layout_backgrounds::LengthPercent::length(10.0)),
                        height: Some(layout_backgrounds::LengthPercent::length(10.0)),
                    },
                    repeat_y: BackgroundRepeat::NoRepeat,
                    ..layout_backgrounds::BackgroundLayer::new(BackgroundSource::Image(
                        "checker.gif".into(),
                    ))
                },
            ],
            corners: [layout_backgrounds::CornerRadius {
                x: layout_backgrounds::LengthPercent::length(8.0),
                y: layout_backgrounds::LengthPercent::length(4.0),
            }; 4],
            ..BackgroundStyle::default()
        };
        leaf.ext.insert("backgrounds".into(), style.to_ext());
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let scene = layout_to_paint(&leaf, &make_options(&shaper, &metrics, &resolver));
        assert!(scene
            .instructions
            .iter()
            .any(|value| matches!(value, PaintInstruction::Gradient(_))));
        assert!(scene.instructions.iter().any(|value| matches!(
            value,
            PaintInstruction::Path(path)
                if matches!(path.commands.get(2), Some(PathCommand::CubicTo { .. }))
        )));
        assert!(scene.instructions.iter().any(|value| matches!(
            value,
            PaintInstruction::Clip(clip)
                if clip.path.is_some()
                    && clip.children.iter().any(|child| matches!(child, PaintInstruction::Image(_)))
        )));
    }

    #[test]
    fn elliptical_per_side_borders_emit_join_clips_and_style_geometry() {
        let mut leaf = positioned_container(Vec::new(), 120.0, 70.0);
        let mut style = BackgroundStyle {
            corners: [layout_backgrounds::CornerRadius {
                x: layout_backgrounds::LengthPercent::length(18.0),
                y: layout_backgrounds::LengthPercent::length(12.0),
            }; 4],
            border: layout_backgrounds::BoxEdges {
                top: 6.0,
                right: 8.0,
                bottom: 10.0,
                left: 12.0,
            },
            ..BackgroundStyle::default()
        };
        style.color_clip = BackgroundBox::Padding;
        leaf.ext.insert("backgrounds".into(), style.to_ext());
        leaf.ext.insert(
            "paint".into(),
            ExtValue::Map(HashMap::from([
                ("borderTopWidth".into(), ExtValue::Float(6.0)),
                ("borderTopColor".into(), color_ext(255, 0, 0)),
                ("borderTopStyle".into(), ExtValue::Str("solid".into())),
                ("borderRightWidth".into(), ExtValue::Float(8.0)),
                ("borderRightColor".into(), color_ext(0, 128, 0)),
                ("borderRightStyle".into(), ExtValue::Str("dashed".into())),
                ("borderBottomWidth".into(), ExtValue::Float(10.0)),
                ("borderBottomColor".into(), color_ext(0, 0, 255)),
                ("borderBottomStyle".into(), ExtValue::Str("dotted".into())),
                ("borderLeftWidth".into(), ExtValue::Float(12.0)),
                ("borderLeftColor".into(), color_ext(0, 0, 0)),
                ("borderLeftStyle".into(), ExtValue::Str("double".into())),
            ])),
        );
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let scene = layout_to_paint(&leaf, &make_options(&shaper, &metrics, &resolver));
        let clips = scene
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                PaintInstruction::Clip(clip) if clip.path.is_some() => Some(clip),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(clips.len(), 4);
        assert!(matches!(clips[0].children[0], PaintInstruction::Path(_)));
        assert!(matches!(clips[3].children[0], PaintInstruction::Group(_)));
    }

    #[test]
    fn deeply_nested_tree_does_not_stack_overflow_in_walk() {
        // Regression lock against the unbounded-recursion DoS that
        // the initial draft had. A 1000-deep chain exceeds what
        // recursive descent can handle without blowing the test
        // thread's default stack budget. The iterative walk must
        // traverse it heap-bounded.
        //
        // Note: we iteratively deallocate the tree at the end too —
        // `Vec<PositionedNode>`'s default Drop impl IS recursive and
        // would itself overflow at this depth. Manually flattening
        // children into an explicit work queue keeps the test honest.
        let mut current = positioned_container(Vec::new(), 100.0, 100.0);
        for _ in 0..1000 {
            current = PositionedNode {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
                id: None,
                content: None,
                children: vec![current],
                ext: HashMap::new(),
            };
        }
        let shaper = FakeShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts = make_options(&shaper, &metrics, &resolver);
        // The walk must not stack-overflow at 1000 levels — recursion
        // would have ~1000 × frame_size of stack. Iterative walk is
        // heap-bounded.
        let _scene = layout_to_paint(&current, &opts);

        // Drain the tree iteratively so that Drop doesn't recurse
        // the whole way down and crash the test thread.
        iteratively_drain(current);
    }

    /// Consume the tree by moving children off each node into a
    /// heap-allocated queue before dropping the parent — so the
    /// default recursive Drop impl for `Vec<PositionedNode>` never
    /// actually recurses deeper than a single level.
    fn iteratively_drain(root: PositionedNode) {
        let mut queue: Vec<PositionedNode> = vec![root];
        while let Some(mut node) = queue.pop() {
            let children = std::mem::take(&mut node.children);
            queue.extend(children);
            // `node` drops here with its children already moved out,
            // so the recursive Drop doesn't descend further.
        }
    }

    #[test]
    fn rgb_helper_works_in_test_fixture() {
        // Sanity: the `rgb()` helper from layout-ir is used in the
        // document-default-theme, so its behavior matters for downstream.
        assert_eq!(
            rgb(1, 2, 3),
            Color {
                r: 1,
                g: 2,
                b: 3,
                a: 255
            }
        );
    }

    #[test]
    fn font_fallback_emits_one_paint_glyph_run_per_segment() {
        // Regression test for the ẁ-for-arrow bug: a shaper that
        // splits on U+2192 into two font-bound segments (primary
        // for ASCII, fallback for the arrow) must produce two
        // PaintGlyphRuns, each carrying its own font_ref and
        // correctly-positioned glyphs.
        let tc = TextContent {
            value: "a → b".into(),
            font: font_spec("Test", 10.0),
            color: color_black(),
            decoration: None,
            max_lines: None,
            wrap: true,
            text_align: TextAlign::Start,
        };
        let leaf = positioned_leaf(tc, 0.0, 0.0, 500.0, 20.0);

        let shaper = FallbackSplittingShaper;
        let metrics = FakeMetrics;
        let resolver = FakeResolver;
        let opts: LayoutToPaintOptions<'_, _, _, _> = LayoutToPaintOptions {
            width: 500.0,
            height: 20.0,
            background: color_white(),
            device_pixel_ratio: 1.0,
            shaper: &shaper,
            metrics: &metrics,
            resolver: &resolver,
        };
        let scene = layout_to_paint(&leaf, &opts);

        let runs: Vec<&PaintGlyphRun> = scene
            .instructions
            .iter()
            .filter_map(|i| match i {
                PaintInstruction::GlyphRun(g) => Some(g),
                _ => None,
            })
            .collect();

        // "a → b" = "a " (primary) + "→" (fallback) + " b" (primary).
        // The fallback-splitting shaper emits 3 ShapedRuns; layout-to-paint
        // must emit 3 PaintGlyphRuns.
        assert_eq!(
            runs.len(),
            3,
            "expected 3 PaintGlyphRuns (a / → / b), got {}",
            runs.len()
        );

        // At least two distinct font_refs among the runs — the core
        // invariant that the bug violated.
        let unique_refs: std::collections::HashSet<&str> =
            runs.iter().map(|r| r.font_ref.as_str()).collect();
        assert!(
            unique_refs.len() >= 2,
            "expected >= 2 distinct font_refs, got {:?}",
            unique_refs
        );

        // The middle run is the fallback one.
        assert_eq!(runs[1].font_ref, "fake:fallback");
        assert_eq!(runs[0].font_ref, "fake:primary");
        assert_eq!(runs[2].font_ref, "fake:primary");

        // Glyphs positions increase monotonically across the whole
        // line (no segment overlap).
        let glyph_xs: Vec<f64> = runs
            .iter()
            .flat_map(|r| r.glyphs.iter().map(|g| g.x))
            .collect();
        for pair in glyph_xs.windows(2) {
            assert!(
                pair[1] >= pair[0],
                "glyph x should be monotonically non-decreasing across segments: {:?}",
                glyph_xs
            );
        }
    }
}
