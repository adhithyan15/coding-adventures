//! Skia backend for the Paint VM runtime.
//!
//! This crate renders [`PaintScene`] into an offscreen Skia raster surface and
//! reads the result back as a [`PixelContainer`]. It is intentionally CPU-raster
//! first so the backend is useful in CI and headless pipelines before we wire
//! the GPU/Graphite surfaces.

use std::collections::HashMap;

use paint_instructions::{
    BlendMode as PaintBlendMode, FillRule, GradientKind, ImageSrc, PaintClip, PaintEllipse,
    PaintGlyphRun, PaintGradient, PaintGroup, PaintImage, PaintInstruction, PaintLayer, PaintLine,
    PaintPath, PaintRect, PaintScene, PaintText, PathCommand, PixelContainer, StrokeCap,
    StrokeJoin, TextAlign, Transform2D,
};
use paint_vm_runtime::{
    PaintAcceleration, PaintBackendCapabilities, PaintBackendDescriptor, PaintBackendFamily,
    PaintBackendTier, PaintPlatformSupport, PaintRenderError, PaintRenderer, SupportLevel,
};
use skia_safe::{
    dash_path_effect, images, paint, shaders, surfaces, AlphaType, ClipOp, Color4f, ColorType,
    Data, Font, FontMgr, FontStyle, ImageInfo, Matrix, Paint, PathBuilder, PathFillType, Point,
    RRect, Rect, TileMode,
};

pub const VERSION: &str = "0.1.0";

pub struct SkiaPaintBackend;

pub fn descriptor() -> PaintBackendDescriptor {
    PaintBackendDescriptor {
        id: "paint-vm-skia",
        display_name: "Paint VM Skia",
        family: PaintBackendFamily::Skia,
        acceleration: PaintAcceleration::Hybrid,
        tier: PaintBackendTier::Tier1Smoke,
        platforms: PaintPlatformSupport::all_desktop(),
        capabilities: PaintBackendCapabilities {
            rect: SupportLevel::Supported,
            line: SupportLevel::Supported,
            ellipse: SupportLevel::Supported,
            path: SupportLevel::Supported,
            path_arc_to: SupportLevel::Unsupported,
            glyph_run: SupportLevel::Degraded,
            text: SupportLevel::Degraded,
            image: SupportLevel::Supported,
            clip: SupportLevel::Supported,
            group: SupportLevel::Supported,
            group_transform: SupportLevel::Supported,
            group_opacity: SupportLevel::Supported,
            layer: SupportLevel::Supported,
            layer_opacity: SupportLevel::Supported,
            layer_filters: SupportLevel::Unsupported,
            layer_blend_modes: SupportLevel::Unsupported,
            linear_gradient: SupportLevel::Supported,
            radial_gradient: SupportLevel::Supported,
            antialiasing: SupportLevel::Supported,
            offscreen_pixels: SupportLevel::Supported,
        },
        priority: 35,
    }
}

pub fn renderer() -> SkiaPaintBackend {
    SkiaPaintBackend
}

pub fn render(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    renderer().render(scene)
}

impl PaintRenderer for SkiaPaintBackend {
    fn descriptor(&self) -> PaintBackendDescriptor {
        descriptor()
    }

    fn render(&self, scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        render_scene(scene)
    }
}

struct RenderContext {
    gradients: HashMap<String, PaintGradient>,
    font_mgr: FontMgr,
}

fn render_scene(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    let width = scene.width.max(0.0).ceil() as u32;
    let height = scene.height.max(0.0).ceil() as u32;
    if width == 0 || height == 0 {
        return Ok(PixelContainer::new(width, height));
    }

    let mut surface =
        surfaces::raster_n32_premul((width as i32, height as i32)).ok_or_else(|| {
            PaintRenderError::RenderFailed {
                backend: "paint-vm-skia",
                message: format!("failed to allocate Skia raster surface {width}x{height}"),
            }
        })?;

    let mut ctx = RenderContext {
        gradients: collect_gradients(&scene.instructions),
        font_mgr: FontMgr::new(),
    };
    surface.canvas().clear(color4f(&scene.background));
    render_instructions(&mut ctx, surface.canvas(), &scene.instructions)?;

    let mut data = vec![0u8; width as usize * height as usize * 4];
    let info = ImageInfo::new(
        (width as i32, height as i32),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        None,
    );
    let ok = surface.read_pixels(&info, &mut data, width as usize * 4, (0, 0));
    if !ok {
        return Err(PaintRenderError::RenderFailed {
            backend: "paint-vm-skia",
            message: "failed to read pixels from Skia surface".to_string(),
        });
    }

    Ok(PixelContainer::from_data(width, height, data))
}

fn render_instructions(
    ctx: &mut RenderContext,
    canvas: &skia_safe::Canvas,
    instructions: &[PaintInstruction],
) -> Result<(), PaintRenderError> {
    for instruction in instructions {
        match instruction {
            PaintInstruction::Rect(rect) => render_rect(ctx, canvas, rect),
            PaintInstruction::Ellipse(ellipse) => render_ellipse(ctx, canvas, ellipse),
            PaintInstruction::Path(path) => render_path(ctx, canvas, path)?,
            PaintInstruction::Text(text) => render_text(ctx, canvas, text),
            PaintInstruction::GlyphRun(run) => render_glyph_run(ctx, canvas, run),
            PaintInstruction::Group(group) => render_group(ctx, canvas, group)?,
            PaintInstruction::Layer(layer) => render_layer(ctx, canvas, layer)?,
            PaintInstruction::Line(line) => render_line(canvas, line),
            PaintInstruction::Clip(clip) => render_clip(ctx, canvas, clip)?,
            PaintInstruction::Gradient(_) => {}
            PaintInstruction::Image(image) => render_image(canvas, image)?,
        }
    }
    Ok(())
}

fn render_rect(ctx: &RenderContext, canvas: &skia_safe::Canvas, rect: &PaintRect) {
    let sk_rect = Rect::from_xywh(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
    );
    let radius = rect.corner_radius.unwrap_or(0.0).max(0.0) as f32;

    if let Some(mut paint) = fill_paint(ctx, rect.fill.as_deref()) {
        paint.set_style(paint::Style::Fill);
        if radius > 0.0 {
            canvas.draw_rrect(RRect::new_rect_xy(sk_rect, radius, radius), &paint);
        } else {
            canvas.draw_rect(sk_rect, &paint);
        }
    }

    if let Some(paint) = stroke_paint(
        rect.stroke.as_deref(),
        rect.stroke_width,
        rect.stroke_dash.as_deref(),
        rect.stroke_dash_offset,
        None,
        None,
    ) {
        if radius > 0.0 {
            canvas.draw_rrect(RRect::new_rect_xy(sk_rect, radius, radius), &paint);
        } else {
            canvas.draw_rect(sk_rect, &paint);
        }
    }
}

fn render_ellipse(ctx: &RenderContext, canvas: &skia_safe::Canvas, ellipse: &PaintEllipse) {
    let bounds = Rect::from_ltrb(
        (ellipse.cx - ellipse.rx) as f32,
        (ellipse.cy - ellipse.ry) as f32,
        (ellipse.cx + ellipse.rx) as f32,
        (ellipse.cy + ellipse.ry) as f32,
    );

    if let Some(mut paint) = fill_paint(ctx, ellipse.fill.as_deref()) {
        paint.set_style(paint::Style::Fill);
        canvas.draw_oval(bounds, &paint);
    }
    if let Some(paint) = stroke_paint(
        ellipse.stroke.as_deref(),
        ellipse.stroke_width,
        ellipse.stroke_dash.as_deref(),
        ellipse.stroke_dash_offset,
        None,
        None,
    ) {
        canvas.draw_oval(bounds, &paint);
    }
}

fn render_path(
    ctx: &RenderContext,
    canvas: &skia_safe::Canvas,
    path: &PaintPath,
) -> Result<(), PaintRenderError> {
    let mut builder = PathBuilder::new();
    for command in &path.commands {
        match *command {
            PathCommand::MoveTo { x, y } => {
                builder.move_to((x as f32, y as f32));
            }
            PathCommand::LineTo { x, y } => {
                builder.line_to((x as f32, y as f32));
            }
            PathCommand::QuadTo { cx, cy, x, y } => {
                builder.quad_to((cx as f32, cy as f32), (x as f32, y as f32));
            }
            PathCommand::CubicTo {
                cx1,
                cy1,
                cx2,
                cy2,
                x,
                y,
            } => {
                builder.cubic_to(
                    (cx1 as f32, cy1 as f32),
                    (cx2 as f32, cy2 as f32),
                    (x as f32, y as f32),
                );
            }
            PathCommand::ArcTo { .. } => {
                return Err(PaintRenderError::RenderFailed {
                    backend: "paint-vm-skia",
                    message: "PaintPath ArcTo is not wired in the Skia backend yet".to_string(),
                });
            }
            PathCommand::Close => {
                builder.close();
            }
        }
    }
    let mut sk_path = builder.detach();
    sk_path.set_fill_type(
        match path.fill_rule.as_ref().unwrap_or(&FillRule::NonZero) {
            FillRule::NonZero => PathFillType::Winding,
            FillRule::EvenOdd => PathFillType::EvenOdd,
        },
    );

    if let Some(mut paint) = fill_paint(ctx, path.fill.as_deref()) {
        paint.set_style(paint::Style::Fill);
        canvas.draw_path(&sk_path, &paint);
    }
    if let Some(paint) = stroke_paint(
        path.stroke.as_deref(),
        path.stroke_width,
        path.stroke_dash.as_deref(),
        path.stroke_dash_offset,
        path.stroke_cap.as_ref(),
        path.stroke_join.as_ref(),
    ) {
        canvas.draw_path(&sk_path, &paint);
    }

    Ok(())
}

fn render_line(canvas: &skia_safe::Canvas, line: &PaintLine) {
    if let Some(paint) = stroke_paint(
        Some(&line.stroke),
        line.stroke_width,
        line.stroke_dash.as_deref(),
        line.stroke_dash_offset,
        line.stroke_cap.as_ref(),
        None,
    ) {
        canvas.draw_line(
            (line.x1 as f32, line.y1 as f32),
            (line.x2 as f32, line.y2 as f32),
            &paint,
        );
    }
}

fn render_text(ctx: &mut RenderContext, canvas: &skia_safe::Canvas, text: &PaintText) {
    if text.text.is_empty() {
        return;
    }
    let mut paint = solid_paint(text.fill.as_deref().unwrap_or("#000000"));
    paint.set_style(paint::Style::Fill);
    let font = font_for_ref(&ctx.font_mgr, text.font_ref.as_deref(), text.font_size);
    let (width, _) = font.measure_str(&text.text, Some(&paint));
    let x = match text.text_align.as_ref().unwrap_or(&TextAlign::Left) {
        TextAlign::Left => text.x,
        TextAlign::Center => text.x - width as f64 / 2.0,
        TextAlign::Right => text.x - width as f64,
    };
    canvas.draw_str(&text.text, (x as f32, text.y as f32), &font, &paint);
}

fn render_glyph_run(ctx: &mut RenderContext, canvas: &skia_safe::Canvas, run: &PaintGlyphRun) {
    if run.glyphs.is_empty() {
        return;
    }
    let glyphs_and_positions: Vec<(skia_safe::GlyphId, Point)> = run
        .glyphs
        .iter()
        .filter_map(|glyph| {
            Some((
                glyph.glyph_id.try_into().ok()?,
                Point::new(glyph.x as f32, glyph.y as f32),
            ))
        })
        .collect();
    if glyphs_and_positions.is_empty() {
        return;
    }
    let (glyphs, positions): (Vec<_>, Vec<_>) = glyphs_and_positions.into_iter().unzip();
    let mut paint = solid_paint(run.fill.as_deref().unwrap_or("#000000"));
    paint.set_style(paint::Style::Fill);
    let font = font_for_ref(&ctx.font_mgr, Some(&run.font_ref), run.font_size);
    canvas.draw_glyphs_at(&glyphs, positions.as_slice(), (0.0, 0.0), &font, &paint);
}

fn render_group(
    ctx: &mut RenderContext,
    canvas: &skia_safe::Canvas,
    group: &PaintGroup,
) -> Result<(), PaintRenderError> {
    let opacity = group.opacity.unwrap_or(1.0).clamp(0.0, 1.0) as f32;
    canvas.save();
    if opacity < 1.0 {
        canvas.save_layer_alpha_f(None, opacity);
    }
    if let Some(transform) = group.transform {
        concat_transform(canvas, transform);
    }
    let result = render_instructions(ctx, canvas, &group.children);
    if opacity < 1.0 {
        canvas.restore();
    }
    canvas.restore();
    result
}

fn render_layer(
    ctx: &mut RenderContext,
    canvas: &skia_safe::Canvas,
    layer: &PaintLayer,
) -> Result<(), PaintRenderError> {
    if layer
        .filters
        .as_ref()
        .is_some_and(|filters| !filters.is_empty())
    {
        return Err(PaintRenderError::RenderFailed {
            backend: "paint-vm-skia",
            message: "PaintLayer filters are not wired in the Skia backend yet".to_string(),
        });
    }
    if !matches!(
        layer.blend_mode.as_ref(),
        None | Some(PaintBlendMode::Normal)
    ) {
        return Err(PaintRenderError::RenderFailed {
            backend: "paint-vm-skia",
            message: "non-normal PaintLayer blend modes are not wired in the Skia backend yet"
                .to_string(),
        });
    }

    canvas.save();
    if let Some(transform) = layer.transform {
        concat_transform(canvas, transform);
    }
    canvas.save_layer_alpha_f(None, layer.opacity.unwrap_or(1.0).clamp(0.0, 1.0) as f32);
    let result = render_instructions(ctx, canvas, &layer.children);
    canvas.restore();
    canvas.restore();
    result
}

fn render_clip(
    ctx: &mut RenderContext,
    canvas: &skia_safe::Canvas,
    clip: &PaintClip,
) -> Result<(), PaintRenderError> {
    let rect = Rect::from_xywh(
        clip.x as f32,
        clip.y as f32,
        clip.width as f32,
        clip.height as f32,
    );
    canvas.save();
    canvas.clip_rect(rect, ClipOp::Intersect, true);
    let result = render_instructions(ctx, canvas, &clip.children);
    canvas.restore();
    result
}

fn render_image(canvas: &skia_safe::Canvas, image: &PaintImage) -> Result<(), PaintRenderError> {
    let ImageSrc::Pixels(pixels) = &image.src else {
        return Err(PaintRenderError::RenderFailed {
            backend: "paint-vm-skia",
            message: "Skia PaintImage currently accepts ImageSrc::Pixels only".to_string(),
        });
    };
    if pixels.width == 0 || pixels.height == 0 {
        return Ok(());
    }

    let info = ImageInfo::new(
        (pixels.width as i32, pixels.height as i32),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        None,
    );
    let data = Data::new_copy(&pixels.data);
    let Some(skia_image) = images::raster_from_data(&info, data, pixels.width as usize * 4) else {
        return Err(PaintRenderError::RenderFailed {
            backend: "paint-vm-skia",
            message: "failed to create Skia image from PixelContainer".to_string(),
        });
    };
    let dst = Rect::from_xywh(
        image.x as f32,
        image.y as f32,
        image.width as f32,
        image.height as f32,
    );
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_alpha_f(image.opacity.unwrap_or(1.0).clamp(0.0, 1.0) as f32);
    canvas.draw_image_rect(&skia_image, None, dst, &paint);
    Ok(())
}

fn fill_paint(ctx: &RenderContext, fill: Option<&str>) -> Option<Paint> {
    let fill = fill?;
    if is_none_paint(fill) {
        return None;
    }
    if let Some(id) = gradient_ref(fill) {
        if let Some(shader) = ctx
            .gradients
            .get(id)
            .and_then(|gradient| shader_for_gradient(gradient))
        {
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            paint.set_shader(shader);
            return Some(paint);
        }
        return None;
    }
    Some(solid_paint(fill))
}

fn stroke_paint(
    stroke: Option<&str>,
    width: Option<f64>,
    dash: Option<&[f64]>,
    dash_offset: Option<f64>,
    cap: Option<&StrokeCap>,
    join: Option<&StrokeJoin>,
) -> Option<Paint> {
    let stroke = stroke?;
    if is_none_paint(stroke) {
        return None;
    }

    let mut paint = solid_paint(stroke);
    paint.set_style(paint::Style::Stroke);
    paint.set_stroke_width(width.unwrap_or(1.0).max(0.0) as f32);
    paint.set_stroke_cap(match cap.unwrap_or(&StrokeCap::Butt) {
        StrokeCap::Butt => paint::Cap::Butt,
        StrokeCap::Round => paint::Cap::Round,
        StrokeCap::Square => paint::Cap::Square,
    });
    paint.set_stroke_join(match join.unwrap_or(&StrokeJoin::Miter) {
        StrokeJoin::Miter => paint::Join::Miter,
        StrokeJoin::Round => paint::Join::Round,
        StrokeJoin::Bevel => paint::Join::Bevel,
    });
    if let Some(dash) = dash.filter(|dash| !dash.is_empty()) {
        let intervals: Vec<f32> = dash.iter().map(|value| value.max(1.0) as f32).collect();
        if let Some(effect) = dash_path_effect::new(&intervals, dash_offset.unwrap_or(0.0) as f32) {
            paint.set_path_effect(effect);
        }
    }
    Some(paint)
}

fn solid_paint(color: &str) -> Paint {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color4f(color4f(color), None);
    paint
}

fn color4f(color: &str) -> Color4f {
    let (r, g, b, a) = parse_css_color(color);
    Color4f::new(r as f32, g as f32, b as f32, a as f32)
}

fn is_none_paint(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.eq_ignore_ascii_case("none") || trimmed.eq_ignore_ascii_case("transparent")
}

fn parse_css_color(s: &str) -> (f64, f64, f64, f64) {
    let s = s.trim();
    if s.eq_ignore_ascii_case("transparent") || s.eq_ignore_ascii_case("none") {
        return (0.0, 0.0, 0.0, 0.0);
    }
    if let Some(inner) = s.strip_prefix("rgba(").and_then(|v| v.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 4 {
            return (
                parse_css_channel(parts[0]),
                parse_css_channel(parts[1]),
                parse_css_channel(parts[2]),
                parts[3].parse::<f64>().unwrap_or(1.0).clamp(0.0, 1.0),
            );
        }
    }
    if let Some(inner) = s.strip_prefix("rgb(").and_then(|v| v.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 3 {
            return (
                parse_css_channel(parts[0]),
                parse_css_channel(parts[1]),
                parse_css_channel(parts[2]),
                1.0,
            );
        }
    }

    let hex = s.trim_start_matches('#');
    let hex = if hex.len() == 3 {
        let mut expanded = String::with_capacity(6);
        for c in hex.chars() {
            expanded.push(c);
            expanded.push(c);
        }
        expanded
    } else {
        hex.to_string()
    };
    if hex.len() < 6 {
        return (0.0, 0.0, 0.0, 1.0);
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f64 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f64 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f64 / 255.0;
    let a = if hex.len() >= 8 {
        u8::from_str_radix(&hex[6..8], 16).unwrap_or(255) as f64 / 255.0
    } else {
        1.0
    };
    (r, g, b, a)
}

fn parse_css_channel(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or(0.0).clamp(0.0, 255.0) / 255.0
}

fn concat_transform(canvas: &skia_safe::Canvas, transform: Transform2D) {
    let affine = [
        transform[0] as f32,
        transform[1] as f32,
        transform[2] as f32,
        transform[3] as f32,
        transform[4] as f32,
        transform[5] as f32,
    ];
    canvas.concat(&Matrix::from_affine(&affine));
}

fn collect_gradients(instructions: &[PaintInstruction]) -> HashMap<String, PaintGradient> {
    let mut gradients = HashMap::new();
    collect_gradients_into(instructions, &mut gradients);
    gradients
}

fn collect_gradients_into(
    instructions: &[PaintInstruction],
    gradients: &mut HashMap<String, PaintGradient>,
) {
    for instruction in instructions {
        match instruction {
            PaintInstruction::Gradient(gradient) => {
                if let Some(id) = gradient.base.id.as_ref() {
                    gradients.insert(id.clone(), gradient.clone());
                }
            }
            PaintInstruction::Group(group) => collect_gradients_into(&group.children, gradients),
            PaintInstruction::Layer(layer) => collect_gradients_into(&layer.children, gradients),
            PaintInstruction::Clip(clip) => collect_gradients_into(&clip.children, gradients),
            _ => {}
        }
    }
}

fn gradient_ref(value: &str) -> Option<&str> {
    let value = value.trim();
    value
        .strip_prefix("url(#")
        .and_then(|value| value.strip_suffix(')'))
}

fn shader_for_gradient(gradient: &PaintGradient) -> Option<skia_safe::Shader> {
    let mut colors = Vec::with_capacity(gradient.stops.len());
    let mut positions = Vec::with_capacity(gradient.stops.len());
    for stop in &gradient.stops {
        colors.push(color4f(&stop.color));
        positions.push(stop.offset.clamp(0.0, 1.0) as f32);
    }
    if colors.len() < 2 {
        return None;
    }
    let colors = skia_safe::gradient::Colors::new(&colors, Some(&positions), TileMode::Clamp, None);
    let spec =
        skia_safe::gradient::Gradient::new(colors, skia_safe::gradient::Interpolation::default());
    match gradient.kind {
        GradientKind::Linear { x1, y1, x2, y2 } => shaders::linear_gradient(
            ((x1 as f32, y1 as f32), (x2 as f32, y2 as f32)),
            &spec,
            None,
        ),
        GradientKind::Radial { cx, cy, r } => {
            shaders::radial_gradient(((cx as f32, cy as f32), r as f32), &spec, None)
        }
    }
}

fn font_for_ref(font_mgr: &FontMgr, font_ref: Option<&str>, font_size: f64) -> Font {
    let spec = font_ref
        .and_then(parse_font_ref)
        .unwrap_or_else(default_font_spec);
    let font_style = if spec.italic && spec.weight >= 700 {
        FontStyle::bold_italic()
    } else if spec.italic {
        FontStyle::italic()
    } else if spec.weight >= 700 {
        FontStyle::bold()
    } else {
        FontStyle::normal()
    };
    let typeface = font_mgr
        .match_family_style(&spec.family, font_style)
        .or_else(|| font_mgr.match_family_style(&default_font_spec().family, FontStyle::normal()));
    match typeface {
        Some(typeface) => Font::from_typeface(typeface, Some(font_size.max(1.0) as f32)),
        None => {
            let mut font = Font::default();
            font.set_size(font_size.max(1.0) as f32);
            font
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FontSpec {
    family: String,
    weight: i32,
    italic: bool,
}

fn default_font_spec() -> FontSpec {
    FontSpec {
        family: default_font_family().to_string(),
        weight: 400,
        italic: false,
    }
}

fn default_font_family() -> &'static str {
    if cfg!(target_os = "windows") {
        "Segoe UI"
    } else if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
        "Helvetica Neue"
    } else {
        "DejaVu Sans"
    }
}

fn parse_font_ref(font_ref: &str) -> Option<FontSpec> {
    parse_directwrite_font_ref(font_ref).or_else(|| parse_canvas_font_ref(font_ref))
}

fn parse_directwrite_font_ref(font_ref: &str) -> Option<FontSpec> {
    let body = font_ref.strip_prefix("directwrite:")?;
    let (family_part, rest) = body.split_once('@')?;
    let mut weight = 400i32;
    let mut italic = false;
    for part in rest.split(';').skip(1) {
        if let Some(value) = part.strip_prefix("w=") {
            weight = value.parse().unwrap_or(weight);
        } else if let Some(value) = part.strip_prefix("style=") {
            italic = matches!(value, "italic" | "oblique");
        }
    }
    Some(FontSpec {
        family: map_font_family(&unescape_ref_component(family_part)),
        weight,
        italic,
    })
}

fn parse_canvas_font_ref(font_ref: &str) -> Option<FontSpec> {
    let body = font_ref.strip_prefix("canvas:")?;
    let (family_part, rest) = body.split_once('@')?;
    let parts: Vec<&str> = rest.split(':').collect();
    let weight = parts
        .get(1)
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(400);
    let italic = parts
        .get(2)
        .is_some_and(|value| matches!(value.trim(), "italic" | "oblique"));
    Some(FontSpec {
        family: map_font_family(family_part),
        weight,
        italic,
    })
}

fn map_font_family(family: &str) -> String {
    match family.trim().to_ascii_lowercase().as_str() {
        "" | "system-ui" | "ui-sans-serif" | "sans-serif" => default_font_family().to_string(),
        _ => family
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string(),
    }
}

fn unescape_ref_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{
        GlyphPosition, GradientStop, PaintBase, PaintGroup, PaintInstruction, PaintRect,
    };
    use paint_vm_runtime::{
        PaintBackendPreference, PaintBackendRegistry, PaintRenderOptions, SupportLevel,
    };

    fn dark_pixel_count(pixels: &PixelContainer) -> usize {
        pixels
            .data
            .chunks_exact(4)
            .filter(|px| px[0] < 96 && px[1] < 96 && px[2] < 96 && px[3] > 0)
            .count()
    }

    #[test]
    fn exposes_tier1_descriptor() {
        let descriptor = descriptor();
        assert_eq!(descriptor.id, "paint-vm-skia");
        assert_eq!(descriptor.family, PaintBackendFamily::Skia);
        assert_eq!(descriptor.tier, PaintBackendTier::Tier1Smoke);
        assert_eq!(descriptor.capabilities.rect, SupportLevel::Supported);
        assert_eq!(descriptor.capabilities.text, SupportLevel::Degraded);
    }

    #[test]
    fn renders_red_rect_on_white() {
        let mut scene = PaintScene::new(32.0, 32.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                4.0, 4.0, 24.0, 24.0, "#ff0000",
            )));

        let pixels = render(&scene).unwrap();
        assert_eq!((pixels.width, pixels.height), (32, 32));
        assert_eq!(pixels.pixel_at(16, 16), (255, 0, 0, 255));
        assert_eq!(pixels.pixel_at(1, 1), (255, 255, 255, 255));
    }

    #[test]
    fn clip_restricts_drawing() {
        let mut scene = PaintScene::new(32.0, 32.0);
        scene.instructions.push(PaintInstruction::Clip(PaintClip {
            base: PaintBase::default(),
            x: 8.0,
            y: 8.0,
            width: 16.0,
            height: 16.0,
            children: vec![PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 32.0, 32.0, "#000000",
            ))],
        }));

        let pixels = render(&scene).unwrap();
        assert_eq!(pixels.pixel_at(16, 16), (0, 0, 0, 255));
        assert_eq!(pixels.pixel_at(2, 2), (255, 255, 255, 255));
    }

    #[test]
    fn group_transform_translates_children() {
        let mut scene = PaintScene::new(40.0, 30.0);
        scene.instructions.push(PaintInstruction::Group(PaintGroup {
            base: PaintBase::default(),
            children: vec![PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 10.0, 10.0, "#ff0000",
            ))],
            transform: Some([1.0, 0.0, 0.0, 1.0, 20.0, 10.0]),
            opacity: None,
        }));

        let pixels = render(&scene).unwrap();
        assert_eq!(pixels.pixel_at(5, 5), (255, 255, 255, 255));
        assert_eq!(pixels.pixel_at(25, 15), (255, 0, 0, 255));
    }

    #[test]
    fn render_text_draws_visible_pixels() {
        let mut scene = PaintScene::new(160.0, 80.0);
        scene.instructions.push(PaintInstruction::Text(PaintText {
            base: PaintBase::default(),
            x: 16.0,
            y: 52.0,
            text: "Paint VM".to_string(),
            font_ref: Some("canvas:system-ui@28:400".to_string()),
            font_size: 28.0,
            fill: Some("#000000".to_string()),
            text_align: Some(TextAlign::Left),
        }));

        let pixels = render(&scene).unwrap();
        assert!(dark_pixel_count(&pixels) > 20);
    }

    #[test]
    fn render_glyph_run_draws_visible_pixels() {
        let mut scene = PaintScene::new(120.0, 80.0);
        scene
            .instructions
            .push(PaintInstruction::GlyphRun(PaintGlyphRun {
                base: PaintBase::default(),
                glyphs: vec![
                    GlyphPosition {
                        glyph_id: 'H' as u32,
                        x: 16.0,
                        y: 52.0,
                    },
                    GlyphPosition {
                        glyph_id: 'i' as u32,
                        x: 40.0,
                        y: 52.0,
                    },
                ],
                font_ref: "canvas:system-ui@28:400".to_string(),
                font_size: 28.0,
                fill: Some("#000000".to_string()),
            }));

        let pixels = render(&scene).unwrap();
        assert!(dark_pixel_count(&pixels) > 5);
    }

    #[test]
    fn pixel_image_renders() {
        let mut source = PixelContainer::new(2, 1);
        source.set_pixel(0, 0, 255, 0, 0, 255);
        source.set_pixel(1, 0, 0, 0, 255, 255);

        let mut scene = PaintScene::new(20.0, 10.0);
        scene.instructions.push(PaintInstruction::Image(PaintImage {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 10.0,
            src: ImageSrc::Pixels(source),
            opacity: None,
        }));

        let pixels = render(&scene).unwrap();
        assert_eq!(pixels.pixel_at(5, 5), (255, 0, 0, 255));
        assert_eq!(pixels.pixel_at(15, 5), (0, 0, 255, 255));
    }

    #[test]
    fn linear_gradient_fill_renders() {
        let mut scene = PaintScene::new(20.0, 4.0);
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase {
                    id: Some("fade".to_string()),
                    metadata: None,
                },
                kind: GradientKind::Linear {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 20.0,
                    y2: 0.0,
                },
                stops: vec![
                    GradientStop {
                        offset: 0.0,
                        color: "#000000".to_string(),
                    },
                    GradientStop {
                        offset: 1.0,
                        color: "#ffffff".to_string(),
                    },
                ],
            }));
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 4.0,
            fill: Some("url(#fade)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let pixels = render(&scene).unwrap();
        let (left_r, _, _, _) = pixels.pixel_at(1, 2);
        let (right_r, _, _, _) = pixels.pixel_at(18, 2);
        assert!(left_r < right_r);
    }

    #[test]
    fn runtime_selects_skia_for_degraded_text_when_allowed() {
        let backend = renderer();
        let mut registry = PaintBackendRegistry::new();
        registry.register(&backend);
        let mut scene = PaintScene::new(80.0, 40.0);
        scene.instructions.push(PaintInstruction::Text(PaintText {
            base: PaintBase::default(),
            x: 8.0,
            y: 24.0,
            text: "Skia".to_string(),
            font_ref: None,
            font_size: 18.0,
            fill: Some("#000000".to_string()),
            text_align: None,
        }));

        let selected = registry
            .select(
                &scene,
                PaintRenderOptions {
                    preference: PaintBackendPreference::Named("paint-vm-skia".to_string()),
                    allow_degraded: true,
                    require_exact_text: false,
                    ..PaintRenderOptions::default()
                },
            )
            .unwrap();
        assert_eq!(selected.descriptor().id, "paint-vm-skia");
    }
}
