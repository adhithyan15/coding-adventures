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
//! - **No shadows / opacity / layer filters.** ext["paint"] fields
//!   for these are silently ignored in v1.
//! - Images render as a `PaintImage` with the `src` string
//!   unchanged; no intrinsic-size resolution.

use std::collections::HashMap;

use layout_ir::{Color, Content, ExtValue, FontSpec, PositionedNode, TextAlign, TextContent};
use paint_instructions::{
    GlyphPosition, ImageSrc, PaintBase, PaintGlyphRun, PaintImage, PaintInstruction, PaintRect,
    PaintScene,
};
use text_interfaces::{
    FontMetrics, FontQuery, FontResolver, FontStretch, FontStyle, FontWeight, ShapeOptions,
    ShapedText, TextShaper,
};

pub const VERSION: &str = "0.1.0";

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
    let mut stack: Vec<WalkFrame<'_>> = vec![WalkFrame {
        node: root,
        parent_abs_x: 0.0,
        parent_abs_y: 0.0,
    }];
    while let Some(frame) = stack.pop() {
        let abs_x = frame.parent_abs_x + frame.node.x;
        let abs_y = frame.parent_abs_y + frame.node.y;
        let box_w = frame.node.width;
        let box_h = frame.node.height;

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
                    options,
                    &mut font_cache,
                    &mut out,
                );
            }
            Some(Content::Image(ic)) => {
                out.push(PaintInstruction::Image(PaintImage {
                    base: PaintBase::default(),
                    x: abs_x * dpr,
                    y: abs_y * dpr,
                    width: box_w * dpr,
                    height: box_h * dpr,
                    src: ImageSrc::Uri(ic.src.clone()),
                    opacity: None,
                }));
            }
            None => {}
        }

        // Push children in reverse so that popping preserves source
        // order (pre-order traversal).
        for child in frame.node.children.iter().rev() {
            stack.push(WalkFrame {
                node: child,
                parent_abs_x: abs_x,
                parent_abs_y: abs_y,
            });
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
    let paint_map = match node.ext.get("paint") {
        Some(ExtValue::Map(m)) => m,
        _ => return,
    };

    let bg = read_color(paint_map, "backgroundColor");
    let border_color = read_color(paint_map, "borderColor");
    let border_width = read_float(paint_map, "borderWidth");
    let corner_radius = read_float(paint_map, "cornerRadius");

    if bg.is_none() && border_color.is_none() && border_width.unwrap_or(0.0) == 0.0 {
        return;
    }

    if w <= 0.0 || h <= 0.0 {
        return;
    }

    out.push(PaintInstruction::Rect(PaintRect {
        base: PaintBase::default(),
        x: x * dpr,
        y: y * dpr,
        width: w * dpr,
        height: h * dpr,
        fill: bg.map(color_to_css),
        stroke: if border_width.unwrap_or(0.0) > 0.0 {
            border_color.map(color_to_css)
        } else {
            None
        },
        stroke_width: border_width.map(|v| v * dpr),
        corner_radius: corner_radius.map(|v| v * dpr),
        stroke_dash: None,
        stroke_dash_offset: None,
    }));
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

// ═══════════════════════════════════════════════════════════════════════════
// Text content — shape + emit PaintGlyphRun per wrapped line
// ═══════════════════════════════════════════════════════════════════════════

fn emit_text_content<S, M, R>(
    tc: &TextContent,
    box_x: f64,
    box_y: f64,
    box_width: f64,
    dpr: f64,
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
    let shape_opts = ShapeOptions::default();

    // First baseline y, in device-pixel scene coordinates.
    let box_x_dpr = box_x * dpr;
    let box_y_dpr = box_y * dpr;
    let mut baseline_y = box_y_dpr + ascent_dpr;

    let fill_css = color_to_css(tc.color);

    for segment in tc.value.split('\n') {
        let wrapped = wrap_line(options.shaper, handle, segment, size_dpr, max_width_dpr);
        for line in wrapped {
            // Shape the line once. This gives us total_advance for
            // alignment AND the glyph IDs/positions for emission.
            if line.is_empty() {
                baseline_y += line_height_dpr;
                continue;
            }
            let shaped = match options.shaper.shape(&line, handle, size_dpr, &shape_opts) {
                Ok(s) => s,
                Err(_) => { baseline_y += line_height_dpr; continue; }
            };

            // Compute the starting x position based on text alignment.
            let line_advance = shaped.total_advance() as f64;
            let baseline_x = match tc.text_align {
                TextAlign::Center => box_x_dpr + (max_width_dpr - line_advance) / 2.0,
                TextAlign::End    => box_x_dpr + max_width_dpr - line_advance,
                TextAlign::Start  => box_x_dpr,
            };

            emit_glyph_runs_from_shaped(&shaped, size_dpr, baseline_x, baseline_y, &fill_css, out);
            baseline_y += line_height_dpr;
        }
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
        line_pen_y += run
            .glyphs
            .iter()
            .map(|g| g.y_advance as f64)
            .sum::<f64>();
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
) -> Vec<String> {
    if segment.is_empty() {
        return vec![String::new()];
    }
    if max_width <= 0.0 {
        return vec![segment.to_string()];
    }

    let space_width = shaper
        .shape(" ", handle, size, &ShapeOptions::default())
        .map(|r| r.total_advance() as f64)
        .unwrap_or((size as f64) * 0.25);

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width: f64 = 0.0;

    for word in segment.split_whitespace() {
        let word_width = shaper
            .shape(word, handle, size, &ShapeOptions::default())
            .map(|r| r.total_advance() as f64)
            .unwrap_or(word.chars().count() as f64 * (size as f64) * 0.5);

        if current.is_empty() {
            current.push_str(word);
            current_width = word_width;
        } else if current_width + space_width + word_width <= max_width {
            current.push(' ');
            current.push_str(word);
            current_width += space_width + word_width;
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
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
        style: if font.italic { FontStyle::Italic } else { FontStyle::Normal },
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
        color_black, color_white, font_spec, rgb, TextAlign, TextContent,
    };
    use text_interfaces::{
        Direction, FontResolutionError, Glyph, ShapedRun, ShapedText, ShapingError,
    };

    // ─── Minimal in-memory backend for the tests ────────────────

    struct FakeResolver;
    impl FontResolver for FakeResolver {
        type Handle = FakeHandle;
        fn resolve(
            &self,
            _q: &FontQuery,
        ) -> Result<Self::Handle, FontResolutionError> {
            Ok(FakeHandle)
        }
    }

    struct FailingResolver;
    impl FontResolver for FailingResolver {
        type Handle = FakeHandle;
        fn resolve(
            &self,
            _q: &FontQuery,
        ) -> Result<Self::Handle, FontResolutionError> {
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
                let want_font = if needs_fallback { "fake:fallback" } else { "fake:primary" };
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
            max_lines: None,
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
            color_to_css(Color { r: 255, g: 0, b: 0, a: 255 }),
            "rgb(255, 0, 0)"
        );
        let half = color_to_css(Color { r: 0, g: 0, b: 0, a: 128 });
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
    fn font_resolution_is_cached_across_nodes() {
        // Two sibling TextContent nodes with the same font should
        // only invoke the resolver once. We verify indirectly via
        // a wrapper resolver that counts resolve() calls.
        struct CountingResolver {
            count: std::cell::Cell<usize>,
        }
        impl FontResolver for CountingResolver {
            type Handle = FakeHandle;
            fn resolve(
                &self,
                _q: &FontQuery,
            ) -> Result<Self::Handle, FontResolutionError> {
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
        assert_eq!(rgb(1, 2, 3), Color { r: 1, g: 2, b: 3, a: 255 });
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
            max_lines: None,
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
        assert_eq!(runs.len(), 3, "expected 3 PaintGlyphRuns (a / → / b), got {}", runs.len());

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
