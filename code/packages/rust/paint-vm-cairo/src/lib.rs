//! Cairo backend adapter for the Paint VM runtime.
//!
//! Linux and BSD targets render through native `cairo-rs` image surfaces.
//! Other desktop targets keep a deterministic software smoke path so pipeline
//! selection and compatibility fixtures can still exercise the Cairo-family
//! backend without requiring Cairo DLLs/frameworks everywhere.

use std::collections::HashMap;

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
use paint_instructions::{
    GlyphPosition, ImageSrc, PaintClip, PaintEllipse, PaintGlyphRun, PaintGroup, PaintImage,
    PaintLayer, PaintPath, PaintRect, PaintText, PathCommand, TextAlign,
};
use paint_instructions::{
    GradientKind, GradientStop, PaintGradient, PaintInstruction, PaintScene, PixelContainer,
};
use paint_vm_runtime::{
    PaintAcceleration, PaintBackendCapabilities, PaintBackendDescriptor, PaintBackendFamily,
    PaintBackendTier, PaintPlatformSupport, PaintRenderError, PaintRenderer, SupportLevel,
};

pub const VERSION: &str = "0.1.0";

const BACKEND_ID: &str = "paint-vm-cairo";

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Rgba {
    const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    fn with_alpha(self, opacity: f64) -> Self {
        Self {
            a: ((self.a as f64 * opacity.clamp(0.0, 1.0)).round() as u8),
            ..self
        }
    }
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
    value
        .trim()
        .strip_prefix("url(#")
        .and_then(|value| value.strip_suffix(')'))
}

fn gradient_stops(stops: &[GradientStop], opacity: f64) -> Vec<(f64, Rgba)> {
    let mut stops: Vec<(f64, Rgba)> = stops
        .iter()
        .map(|stop| {
            (
                stop.offset.clamp(0.0, 1.0),
                parse_css_color(&stop.color).with_alpha(opacity),
            )
        })
        .collect();
    stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    stops
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
fn sample_gradient_stops(stops: &[(f64, Rgba)], t: f64) -> Rgba {
    if stops.is_empty() {
        return Rgba::TRANSPARENT;
    }
    let t = t.clamp(0.0, 1.0);
    if t <= stops[0].0 {
        return stops[0].1;
    }
    for pair in stops.windows(2) {
        let (left_offset, left_color) = pair[0];
        let (right_offset, right_color) = pair[1];
        if t <= right_offset {
            let width = (right_offset - left_offset).max(f64::EPSILON);
            return mix_rgba(left_color, right_color, (t - left_offset) / width);
        }
    }
    stops
        .last()
        .map(|(_, color)| *color)
        .unwrap_or(Rgba::TRANSPARENT)
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
fn mix_rgba(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let mix = |left: u8, right: u8| -> u8 {
        (left as f64 + (right as f64 - left as f64) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Rgba {
        r: mix(a.r, b.r),
        g: mix(a.g, b.g),
        b: mix(a.b, b.b),
        a: mix(a.a, b.a),
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
fn linear_gradient_t(x: f64, y: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len2 = dx * dx + dy * dy;
    if len2 <= f64::EPSILON {
        return 0.0;
    }
    (((x - x1) * dx + (y - y1) * dy) / len2).clamp(0.0, 1.0)
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
fn software_paint(
    paint: &str,
    opacity: f64,
    gradients: &HashMap<String, PaintGradient>,
) -> Result<Option<SoftwarePaint>, PaintRenderError> {
    if paint.trim().eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    if let Some(id) = gradient_ref(paint) {
        let Some(gradient) = gradients.get(id) else {
            return Err(render_failed(format!(
                "gradient reference '{id}' is not defined"
            )));
        };
        let stops = gradient_stops(&gradient.stops, opacity);
        if stops.is_empty() {
            return Ok(Some(SoftwarePaint::Solid(Rgba::TRANSPARENT)));
        }
        if stops.len() == 1 {
            return Ok(Some(SoftwarePaint::Solid(stops[0].1)));
        }
        return Ok(Some(match gradient.kind {
            GradientKind::Linear { x1, y1, x2, y2 } => SoftwarePaint::Linear {
                x1,
                y1,
                x2,
                y2,
                stops,
            },
            GradientKind::Radial { cx, cy, r } => SoftwarePaint::Radial { cx, cy, r, stops },
        }));
    }
    Ok(Some(SoftwarePaint::Solid(
        parse_css_color(paint).with_alpha(opacity),
    )))
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ClipRect {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
impl ClipRect {
    fn full(width: u32, height: u32) -> Self {
        Self {
            x0: 0,
            y0: 0,
            x1: width as i32,
            y1: height as i32,
        }
    }

    fn intersect(self, other: Self) -> Self {
        Self {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        }
    }

    fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
#[derive(Clone, Copy, Debug, PartialEq)]
struct RenderState {
    clip: ClipRect,
    opacity: f64,
}

pub struct CairoPaintBackend;

pub fn descriptor() -> PaintBackendDescriptor {
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    let capabilities = PaintBackendCapabilities {
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
    };

    #[cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd"
    )))]
    let capabilities = PaintBackendCapabilities {
        rect: SupportLevel::Supported,
        line: SupportLevel::Supported,
        ellipse: SupportLevel::Supported,
        path: SupportLevel::Degraded,
        path_arc_to: SupportLevel::Unsupported,
        glyph_run: SupportLevel::Degraded,
        text: SupportLevel::Degraded,
        image: SupportLevel::Degraded,
        clip: SupportLevel::Supported,
        group: SupportLevel::Supported,
        group_transform: SupportLevel::Unsupported,
        group_opacity: SupportLevel::Degraded,
        layer: SupportLevel::Degraded,
        layer_opacity: SupportLevel::Degraded,
        layer_filters: SupportLevel::Unsupported,
        layer_blend_modes: SupportLevel::Unsupported,
        linear_gradient: SupportLevel::Supported,
        radial_gradient: SupportLevel::Supported,
        antialiasing: SupportLevel::Degraded,
        offscreen_pixels: SupportLevel::Supported,
    };

    PaintBackendDescriptor {
        id: BACKEND_ID,
        display_name: "Paint VM Cairo",
        family: PaintBackendFamily::Cairo,
        acceleration: PaintAcceleration::Cpu,
        tier: PaintBackendTier::Tier1Smoke,
        platforms: PaintPlatformSupport::all_desktop(),
        capabilities,
        priority: 40,
    }
}

pub fn renderer() -> CairoPaintBackend {
    CairoPaintBackend
}

pub fn render(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    renderer().render(scene)
}

impl PaintRenderer for CairoPaintBackend {
    fn descriptor(&self) -> PaintBackendDescriptor {
        descriptor()
    }

    fn render(&self, scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        #[cfg(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            native_cairo::render(scene)
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        )))]
        {
            render_software(scene)
        }
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
fn render_software(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    let width = scene.width.ceil().max(0.0) as u32;
    let height = scene.height.ceil().max(0.0) as u32;
    let mut surface = SoftwareSurface::new(width, height);
    surface.clear(parse_css_color(&scene.background));
    let gradients = collect_gradients(&scene.instructions);

    let state = RenderState {
        clip: ClipRect::full(width, height),
        opacity: 1.0,
    };
    for instruction in &scene.instructions {
        surface.render_instruction(instruction, state, &gradients)?;
    }

    Ok(surface.into_pixels())
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
))]
mod native_cairo {
    use super::{parse_css_color, render_failed, Rgba, BACKEND_ID};
    use cairo::{
        Context, FillRule as CairoFillRule, FontSlant, FontWeight, Format, Glyph, ImageSurface,
        LineCap, LineJoin, LinearGradient, Matrix, Operator, RadialGradient,
    };
    use paint_instructions::{
        FillRule as PaintFillRule, GlyphPosition, ImageSrc, PaintClip, PaintEllipse, PaintGlyphRun,
        PaintGradient, PaintGroup, PaintImage, PaintInstruction, PaintLayer, PaintLine, PaintPath,
        PaintRect, PaintScene, PaintText, PathCommand, PixelContainer, StrokeCap, StrokeJoin,
        TextAlign, Transform2D,
    };
    use paint_vm_runtime::{PaintRenderError, SupportLevel};
    use std::collections::HashMap;

    pub fn render(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        let width = scene.width.ceil().max(0.0) as i32;
        let height = scene.height.ceil().max(0.0) as i32;
        let surface = ImageSurface::create(Format::ARgb32, width, height).map_err(cairo_error)?;

        {
            let cr = Context::new(&surface).map_err(cairo_error)?;
            clear(&cr, parse_css_color(&scene.background))?;
            let gradients = super::collect_gradients(&scene.instructions);
            for instruction in &scene.instructions {
                render_instruction(&cr, instruction, 1.0, &gradients)?;
            }
        }

        surface.flush();
        surface_to_pixels(surface, width as u32, height as u32)
    }

    fn clear(cr: &Context, color: Rgba) -> Result<(), PaintRenderError> {
        cr.save().map_err(cairo_error)?;
        cr.set_operator(Operator::Source);
        set_source(cr, color, 1.0);
        cr.paint().map_err(cairo_error)?;
        cr.restore().map_err(cairo_error)
    }

    fn render_instruction(
        cr: &Context,
        instruction: &PaintInstruction,
        opacity: f64,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        match instruction {
            PaintInstruction::Rect(rect) => render_rect(cr, rect, opacity, gradients),
            PaintInstruction::Line(line) => render_line(cr, line, opacity, gradients),
            PaintInstruction::Ellipse(ellipse) => render_ellipse(cr, ellipse, opacity, gradients),
            PaintInstruction::Path(path) => render_path(cr, path, opacity, gradients),
            PaintInstruction::Text(text) => render_text(cr, text, opacity),
            PaintInstruction::GlyphRun(run) => render_glyph_run(cr, run, opacity),
            PaintInstruction::Group(group) => render_group(cr, group, opacity, gradients),
            PaintInstruction::Layer(layer) => render_layer(cr, layer, opacity, gradients),
            PaintInstruction::Clip(clip) => render_clip(cr, clip, opacity, gradients),
            PaintInstruction::Gradient(_) => Ok(()),
            PaintInstruction::Image(image) => render_image(cr, image, opacity),
        }
    }

    fn render_rect(
        cr: &Context,
        rect: &PaintRect,
        opacity: f64,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        append_rounded_rect(
            cr,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            rect.corner_radius,
        );
        if let Some(fill) = &rect.fill {
            set_paint_source(cr, fill, opacity, gradients)?;
            if rect.stroke.is_some() {
                cr.fill_preserve().map_err(cairo_error)?;
            } else {
                cr.fill().map_err(cairo_error)?;
            }
        }
        if let Some(stroke) = &rect.stroke {
            apply_stroke(
                cr,
                rect.stroke_width,
                None,
                rect.stroke_dash.as_deref(),
                rect.stroke_dash_offset,
            );
            set_paint_source(cr, stroke, opacity, gradients)?;
            cr.stroke().map_err(cairo_error)?;
        } else {
            cr.new_path();
        }
        Ok(())
    }

    fn render_line(
        cr: &Context,
        line: &PaintLine,
        opacity: f64,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        cr.new_path();
        cr.move_to(line.x1, line.y1);
        cr.line_to(line.x2, line.y2);
        apply_stroke(
            cr,
            line.stroke_width,
            line.stroke_cap.as_ref(),
            line.stroke_dash.as_deref(),
            line.stroke_dash_offset,
        );
        set_paint_source(cr, &line.stroke, opacity, gradients)?;
        cr.stroke().map_err(cairo_error)
    }

    fn render_ellipse(
        cr: &Context,
        ellipse: &PaintEllipse,
        opacity: f64,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        if ellipse.rx <= 0.0 || ellipse.ry <= 0.0 {
            return Ok(());
        }

        cr.save().map_err(cairo_error)?;
        cr.translate(ellipse.cx, ellipse.cy);
        cr.scale(ellipse.rx, ellipse.ry);
        cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
        cr.restore().map_err(cairo_error)?;

        if let Some(fill) = &ellipse.fill {
            set_paint_source(cr, fill, opacity, gradients)?;
            if ellipse.stroke.is_some() {
                cr.fill_preserve().map_err(cairo_error)?;
            } else {
                cr.fill().map_err(cairo_error)?;
            }
        }
        if let Some(stroke) = &ellipse.stroke {
            apply_stroke(
                cr,
                ellipse.stroke_width,
                None,
                ellipse.stroke_dash.as_deref(),
                ellipse.stroke_dash_offset,
            );
            set_paint_source(cr, stroke, opacity, gradients)?;
            cr.stroke().map_err(cairo_error)?;
        } else {
            cr.new_path();
        }
        Ok(())
    }

    fn render_path(
        cr: &Context,
        path: &PaintPath,
        opacity: f64,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        if path
            .commands
            .iter()
            .any(|command| matches!(command, PathCommand::ArcTo { .. }))
        {
            return Err(render_failed(
                "native Cairo renderer does not lower SVG ArcTo commands yet",
            ));
        }

        cr.new_path();
        let mut first = None::<(f64, f64)>;
        let mut cursor = None::<(f64, f64)>;
        for command in &path.commands {
            match *command {
                PathCommand::MoveTo { x, y } => {
                    first = Some((x, y));
                    cursor = Some((x, y));
                    cr.move_to(x, y);
                }
                PathCommand::LineTo { x, y } => {
                    cursor = Some((x, y));
                    cr.line_to(x, y);
                }
                PathCommand::QuadTo { cx, cy, x, y } => {
                    if let Some((x0, y0)) = cursor {
                        let c1x = x0 + (2.0 / 3.0) * (cx - x0);
                        let c1y = y0 + (2.0 / 3.0) * (cy - y0);
                        let c2x = x + (2.0 / 3.0) * (cx - x);
                        let c2y = y + (2.0 / 3.0) * (cy - y);
                        cr.curve_to(c1x, c1y, c2x, c2y, x, y);
                    }
                    cursor = Some((x, y));
                }
                PathCommand::CubicTo {
                    cx1,
                    cy1,
                    cx2,
                    cy2,
                    x,
                    y,
                } => {
                    cursor = Some((x, y));
                    cr.curve_to(cx1, cy1, cx2, cy2, x, y);
                }
                PathCommand::Close => {
                    if first.is_some() {
                        cr.close_path();
                        cursor = first;
                    }
                }
                PathCommand::ArcTo { .. } => unreachable!("ArcTo rejected before path lowering"),
            }
        }

        cr.set_fill_rule(
            match path.fill_rule.as_ref().unwrap_or(&PaintFillRule::NonZero) {
                PaintFillRule::NonZero => CairoFillRule::Winding,
                PaintFillRule::EvenOdd => CairoFillRule::EvenOdd,
            },
        );

        if let Some(fill) = &path.fill {
            set_paint_source(cr, fill, opacity, gradients)?;
            if path.stroke.is_some() {
                cr.fill_preserve().map_err(cairo_error)?;
            } else {
                cr.fill().map_err(cairo_error)?;
            }
        }
        if let Some(stroke) = &path.stroke {
            apply_stroke(
                cr,
                path.stroke_width,
                path.stroke_cap.as_ref(),
                path.stroke_dash.as_deref(),
                path.stroke_dash_offset,
            );
            if let Some(join) = &path.stroke_join {
                cr.set_line_join(match join {
                    StrokeJoin::Miter => LineJoin::Miter,
                    StrokeJoin::Round => LineJoin::Round,
                    StrokeJoin::Bevel => LineJoin::Bevel,
                });
            }
            set_paint_source(cr, stroke, opacity, gradients)?;
            cr.stroke().map_err(cairo_error)?;
        } else {
            cr.new_path();
        }

        Ok(())
    }

    fn render_text(cr: &Context, text: &PaintText, opacity: f64) -> Result<(), PaintRenderError> {
        cr.save().map_err(cairo_error)?;
        cr.select_font_face(
            font_family(text.font_ref.as_deref()).as_str(),
            FontSlant::Normal,
            FontWeight::Normal,
        );
        cr.set_font_size(text.font_size);
        set_source(
            cr,
            parse_css_color(text.fill.as_deref().unwrap_or("#000000")),
            opacity,
        );

        let extents = cr.text_extents(&text.text).map_err(cairo_error)?;
        let x = match text.text_align.as_ref().unwrap_or(&TextAlign::Left) {
            TextAlign::Left => text.x,
            TextAlign::Center => text.x - extents.width() / 2.0,
            TextAlign::Right => text.x - extents.width(),
        };
        cr.move_to(x, text.y);
        cr.show_text(&text.text).map_err(cairo_error)?;
        cr.restore().map_err(cairo_error)
    }

    fn render_glyph_run(
        cr: &Context,
        run: &PaintGlyphRun,
        opacity: f64,
    ) -> Result<(), PaintRenderError> {
        cr.save().map_err(cairo_error)?;
        cr.select_font_face(
            font_family(Some(&run.font_ref)).as_str(),
            FontSlant::Normal,
            FontWeight::Normal,
        );
        cr.set_font_size(run.font_size);
        set_source(
            cr,
            parse_css_color(run.fill.as_deref().unwrap_or("#000000")),
            opacity,
        );
        let glyphs: Vec<Glyph> = run
            .glyphs
            .iter()
            .map(|GlyphPosition { glyph_id, x, y }| Glyph::new(*glyph_id as u64, *x, *y))
            .collect();
        cr.show_glyphs(&glyphs).map_err(cairo_error)?;
        cr.restore().map_err(cairo_error)
    }

    fn render_group(
        cr: &Context,
        group: &PaintGroup,
        opacity: f64,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        let opacity = opacity * group.opacity.unwrap_or(1.0).clamp(0.0, 1.0);
        cr.save().map_err(cairo_error)?;
        if let Some(transform) = group.transform {
            apply_transform(cr, transform);
        }
        for child in &group.children {
            render_instruction(cr, child, opacity, gradients)?;
        }
        cr.restore().map_err(cairo_error)
    }

    fn render_layer(
        cr: &Context,
        layer: &PaintLayer,
        opacity: f64,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        if layer
            .filters
            .as_ref()
            .is_some_and(|filters| !filters.is_empty())
        {
            return Err(render_failed(
                "native Cairo renderer does not implement layer filters yet",
            ));
        }
        if layer.blend_mode.is_some() {
            return Err(render_failed(
                "native Cairo renderer does not implement layer blend modes yet",
            ));
        }

        cr.save().map_err(cairo_error)?;
        if let Some(transform) = layer.transform {
            apply_transform(cr, transform);
        }
        cr.push_group();
        for child in &layer.children {
            render_instruction(cr, child, 1.0, gradients)?;
        }
        cr.pop_group_to_source().map_err(cairo_error)?;
        cr.paint_with_alpha(opacity * layer.opacity.unwrap_or(1.0).clamp(0.0, 1.0))
            .map_err(cairo_error)?;
        cr.restore().map_err(cairo_error)
    }

    fn render_clip(
        cr: &Context,
        clip: &PaintClip,
        opacity: f64,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        cr.save().map_err(cairo_error)?;
        cr.rectangle(clip.x, clip.y, clip.width, clip.height);
        cr.clip();
        for child in &clip.children {
            render_instruction(cr, child, opacity, gradients)?;
        }
        cr.restore().map_err(cairo_error)
    }

    fn render_image(
        cr: &Context,
        image: &PaintImage,
        opacity: f64,
    ) -> Result<(), PaintRenderError> {
        let ImageSrc::Pixels(src) = &image.src else {
            return Err(render_failed(
                "native Cairo renderer only supports ImageSrc::Pixels",
            ));
        };
        if image.width <= 0.0 || image.height <= 0.0 || src.width == 0 || src.height == 0 {
            return Ok(());
        }

        let source = image_surface_from_pixels(src)?;
        cr.save().map_err(cairo_error)?;
        cr.translate(image.x, image.y);
        cr.scale(
            image.width / src.width as f64,
            image.height / src.height as f64,
        );
        cr.set_source_surface(&source, 0.0, 0.0)
            .map_err(cairo_error)?;
        cr.paint_with_alpha(opacity * image.opacity.unwrap_or(1.0).clamp(0.0, 1.0))
            .map_err(cairo_error)?;
        cr.restore().map_err(cairo_error)
    }

    fn append_rounded_rect(
        cr: &Context,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        radius: Option<f64>,
    ) {
        let radius = radius
            .unwrap_or(0.0)
            .max(0.0)
            .min(width / 2.0)
            .min(height / 2.0);
        cr.new_path();
        if radius <= f64::EPSILON {
            cr.rectangle(x, y, width, height);
            return;
        }

        cr.new_sub_path();
        cr.arc(
            x + width - radius,
            y + radius,
            radius,
            -std::f64::consts::FRAC_PI_2,
            0.0,
        );
        cr.arc(
            x + width - radius,
            y + height - radius,
            radius,
            0.0,
            std::f64::consts::FRAC_PI_2,
        );
        cr.arc(
            x + radius,
            y + height - radius,
            radius,
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::PI,
        );
        cr.arc(
            x + radius,
            y + radius,
            radius,
            std::f64::consts::PI,
            std::f64::consts::PI * 1.5,
        );
        cr.close_path();
    }

    fn apply_stroke(
        cr: &Context,
        width: Option<f64>,
        cap: Option<&StrokeCap>,
        dash: Option<&[f64]>,
        dash_offset: Option<f64>,
    ) {
        cr.set_line_width(width.unwrap_or(1.0).max(1.0));
        cr.set_line_cap(match cap.unwrap_or(&StrokeCap::Butt) {
            StrokeCap::Butt => LineCap::Butt,
            StrokeCap::Round => LineCap::Round,
            StrokeCap::Square => LineCap::Square,
        });
        if let Some(dash) = dash {
            cr.set_dash(dash, dash_offset.unwrap_or(0.0));
        } else {
            cr.set_dash(&[], 0.0);
        }
    }

    fn apply_transform(cr: &Context, transform: Transform2D) {
        let [a, b, c, d, e, f] = transform;
        cr.transform(Matrix::new(a, b, c, d, e, f));
    }

    fn set_source(cr: &Context, color: Rgba, opacity: f64) {
        let alpha = (color.a as f64 / 255.0) * opacity.clamp(0.0, 1.0);
        cr.set_source_rgba(
            color.r as f64 / 255.0,
            color.g as f64 / 255.0,
            color.b as f64 / 255.0,
            alpha,
        );
    }

    fn set_paint_source(
        cr: &Context,
        paint: &str,
        opacity: f64,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        if let Some(id) = super::gradient_ref(paint) {
            let Some(gradient) = gradients.get(id) else {
                return Err(render_failed(format!(
                    "gradient reference '{id}' is not defined"
                )));
            };
            set_gradient_source(cr, gradient, opacity)
        } else {
            set_source(cr, parse_css_color(paint), opacity);
            Ok(())
        }
    }

    fn set_gradient_source(
        cr: &Context,
        gradient: &PaintGradient,
        opacity: f64,
    ) -> Result<(), PaintRenderError> {
        let stops = super::gradient_stops(&gradient.stops, opacity);
        if stops.is_empty() {
            set_source(cr, Rgba::TRANSPARENT, 1.0);
            return Ok(());
        }
        if stops.len() == 1 {
            set_source(cr, stops[0].1, 1.0);
            return Ok(());
        }
        match gradient.kind {
            super::GradientKind::Linear { x1, y1, x2, y2 } => {
                let pattern = LinearGradient::new(x1, y1, x2, y2);
                add_stops(&pattern, &stops);
                cr.set_source(&pattern).map_err(cairo_error)
            }
            super::GradientKind::Radial { cx, cy, r } => {
                let pattern = RadialGradient::new(cx, cy, 0.0, cx, cy, r.max(0.0));
                add_stops(&pattern, &stops);
                cr.set_source(&pattern).map_err(cairo_error)
            }
        }
    }

    fn add_stops(pattern: &impl AsRef<cairo::Gradient>, stops: &[(f64, Rgba)]) {
        let pattern = pattern.as_ref();
        for (offset, color) in stops {
            pattern.add_color_stop_rgba(
                *offset,
                color.r as f64 / 255.0,
                color.g as f64 / 255.0,
                color.b as f64 / 255.0,
                color.a as f64 / 255.0,
            );
        }
    }

    fn font_family(font_ref: Option<&str>) -> String {
        let Some(font_ref) = font_ref else {
            return "Sans".to_string();
        };
        let spec = font_ref
            .strip_prefix("canvas:")
            .or_else(|| font_ref.strip_prefix("directwrite:"))
            .unwrap_or(font_ref);
        spec.split('@')
            .next()
            .filter(|family| !family.is_empty())
            .unwrap_or("Sans")
            .to_string()
    }

    fn image_surface_from_pixels(src: &PixelContainer) -> Result<ImageSurface, PaintRenderError> {
        let stride = src.width as usize * 4;
        let mut data = vec![0u8; stride * src.height as usize];
        for y in 0..src.height {
            for x in 0..src.width {
                let (r, g, b, a) = src.pixel_at(x, y);
                let i = (y as usize * stride) + x as usize * 4;
                data[i] = premultiply(b, a);
                data[i + 1] = premultiply(g, a);
                data[i + 2] = premultiply(r, a);
                data[i + 3] = a;
            }
        }
        ImageSurface::create_for_data(
            data,
            Format::ARgb32,
            src.width as i32,
            src.height as i32,
            stride as i32,
        )
        .map_err(cairo_error)
    }

    fn surface_to_pixels(
        mut surface: ImageSurface,
        width: u32,
        height: u32,
    ) -> Result<PixelContainer, PaintRenderError> {
        let stride = surface.stride() as usize;
        let data = surface.data().map_err(cairo_error)?;
        let mut pixels = PixelContainer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let i = y as usize * stride + x as usize * 4;
                let b = data[i];
                let g = data[i + 1];
                let r = data[i + 2];
                let a = data[i + 3];
                pixels.set_pixel(
                    x,
                    y,
                    unpremultiply(r, a),
                    unpremultiply(g, a),
                    unpremultiply(b, a),
                    a,
                );
            }
        }
        Ok(pixels)
    }

    fn premultiply(channel: u8, alpha: u8) -> u8 {
        ((channel as u16 * alpha as u16 + 127) / 255) as u8
    }

    fn unpremultiply(channel: u8, alpha: u8) -> u8 {
        if alpha == 0 {
            return 0;
        }
        ((channel as u16 * 255 + alpha as u16 / 2) / alpha as u16).min(255) as u8
    }

    fn cairo_error(error: impl std::fmt::Display) -> PaintRenderError {
        PaintRenderError::RenderFailed {
            backend: BACKEND_ID,
            message: error.to_string(),
        }
    }

    #[allow(dead_code)]
    fn _assert_text_is_degraded(level: SupportLevel) {
        debug_assert_eq!(level, SupportLevel::Degraded);
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
#[derive(Clone, Debug)]
enum SoftwarePaint {
    Solid(Rgba),
    Linear {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stops: Vec<(f64, Rgba)>,
    },
    Radial {
        cx: f64,
        cy: f64,
        r: f64,
        stops: Vec<(f64, Rgba)>,
    },
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
impl SoftwarePaint {
    fn sample(&self, x: f64, y: f64) -> Rgba {
        match self {
            Self::Solid(color) => *color,
            Self::Linear {
                x1,
                y1,
                x2,
                y2,
                stops,
            } => sample_gradient_stops(stops, linear_gradient_t(x, y, *x1, *y1, *x2, *y2)),
            Self::Radial { cx, cy, r, stops } => {
                let dx = x - *cx;
                let dy = y - *cy;
                let t = if *r <= f64::EPSILON {
                    0.0
                } else {
                    (dx * dx + dy * dy).sqrt() / *r
                };
                sample_gradient_stops(stops, t)
            }
        }
    }

    fn is_transparent(&self) -> bool {
        match self {
            Self::Solid(color) => color.a == 0,
            Self::Linear { stops, .. } | Self::Radial { stops, .. } => {
                stops.iter().all(|(_, color)| color.a == 0)
            }
        }
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
struct SoftwareSurface {
    pixels: PixelContainer,
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
impl SoftwareSurface {
    fn new(width: u32, height: u32) -> Self {
        Self {
            pixels: PixelContainer::new(width, height),
        }
    }

    fn clear(&mut self, color: Rgba) {
        self.pixels.fill(color.r, color.g, color.b, color.a);
    }

    fn into_pixels(self) -> PixelContainer {
        self.pixels
    }

    fn render_instruction(
        &mut self,
        instruction: &PaintInstruction,
        state: RenderState,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        match instruction {
            PaintInstruction::Rect(rect) => self.render_rect(rect, state, gradients),
            PaintInstruction::Line(line) => {
                if let Some(paint) = software_paint(&line.stroke, state.opacity, gradients)? {
                    self.render_line_paint(
                        line.x1,
                        line.y1,
                        line.x2,
                        line.y2,
                        &paint,
                        line.stroke_width.unwrap_or(1.0),
                        state,
                    );
                }
                Ok(())
            }
            PaintInstruction::Ellipse(ellipse) => self.render_ellipse(ellipse, state, gradients),
            PaintInstruction::Path(path) => self.render_path(path, state, gradients),
            PaintInstruction::Text(text) => self.render_text(text, state),
            PaintInstruction::GlyphRun(run) => self.render_glyph_run(run, state),
            PaintInstruction::Group(group) => self.render_group(group, state, gradients),
            PaintInstruction::Layer(layer) => self.render_layer(layer, state, gradients),
            PaintInstruction::Clip(clip) => self.render_clip(clip, state, gradients),
            PaintInstruction::Gradient(_) => Ok(()),
            PaintInstruction::Image(image) => self.render_image(image, state),
        }
    }

    fn render_rect(
        &mut self,
        rect: &PaintRect,
        state: RenderState,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        if let Some(fill) = &rect.fill {
            if let Some(paint) = software_paint(fill, state.opacity, gradients)? {
                self.fill_rect_paint(rect.x, rect.y, rect.width, rect.height, &paint, state.clip);
            }
        }

        if let Some(stroke) = &rect.stroke {
            let Some(paint) = software_paint(stroke, state.opacity, gradients)? else {
                return Ok(());
            };
            let width = rect.stroke_width.unwrap_or(1.0);
            self.stroke_rect_paint(
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                width,
                &paint,
                state,
            );
        }

        Ok(())
    }

    fn render_ellipse(
        &mut self,
        ellipse: &PaintEllipse,
        state: RenderState,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        if ellipse.rx <= 0.0 || ellipse.ry <= 0.0 {
            return Ok(());
        }

        let x0 = (ellipse.cx - ellipse.rx).floor() as i32;
        let y0 = (ellipse.cy - ellipse.ry).floor() as i32;
        let x1 = (ellipse.cx + ellipse.rx).ceil() as i32;
        let y1 = (ellipse.cy + ellipse.ry).ceil() as i32;
        let stroke_width = ellipse.stroke_width.unwrap_or(1.0).max(1.0);
        let stroke_band = (stroke_width / ellipse.rx.max(ellipse.ry)).max(0.01);
        let fill = ellipse
            .fill
            .as_deref()
            .map(|fill| software_paint(fill, state.opacity, gradients))
            .transpose()?
            .flatten();
        let stroke = ellipse
            .stroke
            .as_deref()
            .map(|stroke| software_paint(stroke, state.opacity, gradients))
            .transpose()?
            .flatten();

        for y in y0..y1 {
            for x in x0..x1 {
                let nx = (x as f64 + 0.5 - ellipse.cx) / ellipse.rx;
                let ny = (y as f64 + 0.5 - ellipse.cy) / ellipse.ry;
                let distance = nx * nx + ny * ny;
                if let Some(fill) = &fill {
                    if distance <= 1.0 {
                        self.blend_pixel(
                            x,
                            y,
                            fill.sample(x as f64 + 0.5, y as f64 + 0.5),
                            state.clip,
                        );
                    }
                }
                if let Some(stroke) = &stroke {
                    if (1.0 - stroke_band..=1.0 + stroke_band).contains(&distance) {
                        self.blend_pixel(
                            x,
                            y,
                            stroke.sample(x as f64 + 0.5, y as f64 + 0.5),
                            state.clip,
                        );
                    }
                }
            }
        }

        Ok(())
    }

    fn render_path(
        &mut self,
        path: &PaintPath,
        state: RenderState,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        if path
            .commands
            .iter()
            .any(|command| matches!(command, PathCommand::ArcTo { .. }))
        {
            return Err(render_failed(
                "Cairo smoke renderer cannot lower SVG ArcTo commands yet",
            ));
        }

        let stroke = path
            .stroke
            .as_deref()
            .map(|stroke| software_paint(stroke, state.opacity, gradients))
            .transpose()?
            .flatten();
        let fill = path
            .fill
            .as_deref()
            .map(|fill| software_paint(fill, state.opacity, gradients))
            .transpose()?
            .flatten();

        let mut first = None::<(f64, f64)>;
        let mut cursor = None::<(f64, f64)>;
        let mut points = Vec::new();

        for command in &path.commands {
            match *command {
                PathCommand::MoveTo { x, y } => {
                    first = Some((x, y));
                    cursor = Some((x, y));
                    points.push((x, y));
                }
                PathCommand::LineTo { x, y } => {
                    if let Some((x0, y0)) = cursor {
                        if let Some(paint) = &stroke {
                            self.render_line_paint(
                                x0,
                                y0,
                                x,
                                y,
                                paint,
                                path.stroke_width.unwrap_or(1.0),
                                state,
                            );
                        }
                    }
                    cursor = Some((x, y));
                    points.push((x, y));
                }
                PathCommand::Close => {
                    if let (Some((x0, y0)), Some((x1, y1))) = (cursor, first) {
                        if let Some(paint) = &stroke {
                            self.render_line_paint(
                                x0,
                                y0,
                                x1,
                                y1,
                                paint,
                                path.stroke_width.unwrap_or(1.0),
                                state,
                            );
                        }
                    }
                }
                PathCommand::QuadTo { x, y, .. } | PathCommand::CubicTo { x, y, .. } => {
                    if let Some((x0, y0)) = cursor {
                        if let Some(paint) = &stroke {
                            self.render_line_paint(
                                x0,
                                y0,
                                x,
                                y,
                                paint,
                                path.stroke_width.unwrap_or(1.0),
                                state,
                            );
                        }
                    }
                    cursor = Some((x, y));
                    points.push((x, y));
                }
                PathCommand::ArcTo { .. } => unreachable!("ArcTo rejected before path lowering"),
            }
        }

        if let Some(paint) = fill {
            self.fill_polygon_bounds_paint(&points, &paint, state.clip);
        }

        Ok(())
    }

    fn render_text(
        &mut self,
        text: &PaintText,
        state: RenderState,
    ) -> Result<(), PaintRenderError> {
        let color =
            parse_css_color(text.fill.as_deref().unwrap_or("#000000")).with_alpha(state.opacity);
        let char_width = (text.font_size * 0.56).max(1.0);
        let text_width = char_width * text.text.chars().count() as f64;
        let x = match text.text_align.as_ref().unwrap_or(&TextAlign::Left) {
            TextAlign::Left => text.x,
            TextAlign::Center => text.x - text_width / 2.0,
            TextAlign::Right => text.x - text_width,
        };
        let top = text.y - text.font_size;
        for (index, ch) in text.text.chars().enumerate() {
            if !ch.is_whitespace() {
                self.fill_rect(
                    x + index as f64 * char_width,
                    top,
                    (char_width * 0.7).max(1.0),
                    text.font_size.max(1.0),
                    color,
                    state.clip,
                );
            }
        }
        Ok(())
    }

    fn render_glyph_run(
        &mut self,
        run: &PaintGlyphRun,
        state: RenderState,
    ) -> Result<(), PaintRenderError> {
        let color =
            parse_css_color(run.fill.as_deref().unwrap_or("#000000")).with_alpha(state.opacity);
        for GlyphPosition { glyph_id, x, y } in &run.glyphs {
            if *glyph_id != 0 {
                self.fill_rect(
                    *x,
                    *y - run.font_size,
                    (run.font_size * 0.55).max(1.0),
                    run.font_size.max(1.0),
                    color,
                    state.clip,
                );
            }
        }
        Ok(())
    }

    fn render_group(
        &mut self,
        group: &PaintGroup,
        state: RenderState,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        if group.transform.is_some() {
            return Err(render_failed(
                "Cairo smoke renderer does not implement group transforms yet",
            ));
        }
        let state = RenderState {
            opacity: state.opacity * group.opacity.unwrap_or(1.0).clamp(0.0, 1.0),
            ..state
        };
        for child in &group.children {
            self.render_instruction(child, state, gradients)?;
        }
        Ok(())
    }

    fn render_layer(
        &mut self,
        layer: &PaintLayer,
        state: RenderState,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        if layer.transform.is_some() {
            return Err(render_failed(
                "Cairo smoke renderer does not implement layer transforms yet",
            ));
        }
        if layer
            .filters
            .as_ref()
            .is_some_and(|filters| !filters.is_empty())
        {
            return Err(render_failed(
                "Cairo smoke renderer does not implement layer filters yet",
            ));
        }
        if layer.blend_mode.is_some() {
            return Err(render_failed(
                "Cairo smoke renderer does not implement layer blend modes yet",
            ));
        }
        let state = RenderState {
            opacity: state.opacity * layer.opacity.unwrap_or(1.0).clamp(0.0, 1.0),
            ..state
        };
        for child in &layer.children {
            self.render_instruction(child, state, gradients)?;
        }
        Ok(())
    }

    fn render_clip(
        &mut self,
        clip: &PaintClip,
        state: RenderState,
        gradients: &HashMap<String, PaintGradient>,
    ) -> Result<(), PaintRenderError> {
        let clip_rect = ClipRect {
            x0: clip.x.floor() as i32,
            y0: clip.y.floor() as i32,
            x1: (clip.x + clip.width).ceil() as i32,
            y1: (clip.y + clip.height).ceil() as i32,
        };
        let state = RenderState {
            clip: state.clip.intersect(clip_rect),
            ..state
        };
        for child in &clip.children {
            self.render_instruction(child, state, gradients)?;
        }
        Ok(())
    }

    fn render_image(
        &mut self,
        image: &PaintImage,
        state: RenderState,
    ) -> Result<(), PaintRenderError> {
        let ImageSrc::Pixels(src) = &image.src else {
            return Err(render_failed(
                "Cairo smoke renderer only supports ImageSrc::Pixels",
            ));
        };
        if image.width <= 0.0 || image.height <= 0.0 || src.width == 0 || src.height == 0 {
            return Ok(());
        }

        let opacity = state.opacity * image.opacity.unwrap_or(1.0).clamp(0.0, 1.0);
        let x0 = image.x.floor() as i32;
        let y0 = image.y.floor() as i32;
        let x1 = (image.x + image.width).ceil() as i32;
        let y1 = (image.y + image.height).ceil() as i32;

        for y in y0..y1 {
            for x in x0..x1 {
                let u = ((x as f64 - image.x) / image.width).clamp(0.0, 1.0);
                let v = ((y as f64 - image.y) / image.height).clamp(0.0, 1.0);
                let sx = (u * (src.width.saturating_sub(1)) as f64).round() as u32;
                let sy = (v * (src.height.saturating_sub(1)) as f64).round() as u32;
                let (r, g, b, a) = src.pixel_at(sx, sy);
                self.blend_pixel(x, y, Rgba { r, g, b, a }.with_alpha(opacity), state.clip);
            }
        }

        Ok(())
    }

    fn fill_rect(&mut self, x: f64, y: f64, width: f64, height: f64, color: Rgba, clip: ClipRect) {
        self.fill_rect_paint(x, y, width, height, &SoftwarePaint::Solid(color), clip);
    }

    fn fill_rect_paint(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        paint: &SoftwarePaint,
        clip: ClipRect,
    ) {
        if paint.is_transparent() || width <= 0.0 || height <= 0.0 {
            return;
        }
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = (x + width).ceil() as i32;
        let y1 = (y + height).ceil() as i32;
        for py in y0..y1 {
            for px in x0..x1 {
                self.blend_pixel(px, py, paint.sample(px as f64 + 0.5, py as f64 + 0.5), clip);
            }
        }
    }

    fn stroke_rect_paint(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        stroke_width: f64,
        paint: &SoftwarePaint,
        state: RenderState,
    ) {
        let stroke_width = stroke_width.max(1.0);
        self.fill_rect_paint(x, y, width, stroke_width, paint, state.clip);
        self.fill_rect_paint(
            x,
            y + height - stroke_width,
            width,
            stroke_width,
            paint,
            state.clip,
        );
        self.fill_rect_paint(x, y, stroke_width, height, paint, state.clip);
        self.fill_rect_paint(
            x + width - stroke_width,
            y,
            stroke_width,
            height,
            paint,
            state.clip,
        );
    }

    fn render_line_paint(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        paint: &SoftwarePaint,
        stroke_width: f64,
        state: RenderState,
    ) {
        if paint.is_transparent() {
            return;
        }
        let dx = x1 - x0;
        let dy = y1 - y0;
        let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
        let radius = (stroke_width.max(1.0) / 2.0).ceil() as i32;
        for step in 0..=steps {
            let t = step as f64 / steps as f64;
            let x = (x0 + dx * t).round() as i32;
            let y = (y0 + dy * t).round() as i32;
            for oy in -radius..=radius {
                for ox in -radius..=radius {
                    let px = x + ox;
                    let py = y + oy;
                    self.blend_pixel(
                        px,
                        py,
                        paint.sample(px as f64 + 0.5, py as f64 + 0.5),
                        state.clip,
                    );
                }
            }
        }
    }

    fn fill_polygon_bounds_paint(
        &mut self,
        points: &[(f64, f64)],
        paint: &SoftwarePaint,
        clip: ClipRect,
    ) {
        if points.len() < 3 || paint.is_transparent() {
            return;
        }
        let (mut x0, mut y0) = (f64::MAX, f64::MAX);
        let (mut x1, mut y1) = (f64::MIN, f64::MIN);
        for (x, y) in points {
            x0 = x0.min(*x);
            y0 = y0.min(*y);
            x1 = x1.max(*x);
            y1 = y1.max(*y);
        }
        self.fill_rect_paint(x0, y0, x1 - x0, y1 - y0, paint, clip);
    }

    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba, clip: ClipRect) {
        if color.a == 0 || !clip.contains(x, y) || x < 0 || y < 0 {
            return;
        }
        let x = x as u32;
        let y = y as u32;
        if x >= self.pixels.width || y >= self.pixels.height {
            return;
        }

        let (dr, dg, db, da) = self.pixels.pixel_at(x, y);
        let sa = color.a as f64 / 255.0;
        let da = da as f64 / 255.0;
        let out_a = sa + da * (1.0 - sa);
        if out_a <= f64::EPSILON {
            self.pixels.set_pixel(x, y, 0, 0, 0, 0);
            return;
        }

        let blend_channel = |src: u8, dst: u8| -> u8 {
            (((src as f64 * sa) + (dst as f64 * da * (1.0 - sa))) / out_a)
                .round()
                .clamp(0.0, 255.0) as u8
        };

        self.pixels.set_pixel(
            x,
            y,
            blend_channel(color.r, dr),
            blend_channel(color.g, dg),
            blend_channel(color.b, db),
            (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
        );
    }
}

fn render_failed(message: impl Into<String>) -> PaintRenderError {
    PaintRenderError::RenderFailed {
        backend: BACKEND_ID,
        message: message.into(),
    }
}

fn parse_css_color(s: &str) -> Rgba {
    let s = s.trim();
    if s.eq_ignore_ascii_case("transparent") || s.eq_ignore_ascii_case("none") {
        return Rgba::TRANSPARENT;
    }
    if let Some(inner) = s
        .strip_prefix("rgba(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 4 {
            return Rgba {
                r: parse_css_channel(parts[0]),
                g: parse_css_channel(parts[1]),
                b: parse_css_channel(parts[2]),
                a: (parts[3].parse::<f64>().unwrap_or(1.0).clamp(0.0, 1.0) * 255.0).round() as u8,
            };
        }
    }
    if let Some(inner) = s
        .strip_prefix("rgb(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 3 {
            return Rgba {
                r: parse_css_channel(parts[0]),
                g: parse_css_channel(parts[1]),
                b: parse_css_channel(parts[2]),
                a: 255,
            };
        }
    }

    let hex = s.trim_start_matches('#');
    let expanded = if hex.len() == 3 {
        let mut expanded = String::with_capacity(6);
        for c in hex.chars() {
            expanded.push(c);
            expanded.push(c);
        }
        expanded
    } else {
        hex.to_string()
    };
    if expanded.len() < 6 {
        return Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        };
    }

    Rgba {
        r: u8::from_str_radix(&expanded[0..2], 16).unwrap_or(0),
        g: u8::from_str_radix(&expanded[2..4], 16).unwrap_or(0),
        b: u8::from_str_radix(&expanded[4..6], 16).unwrap_or(0),
        a: if expanded.len() >= 8 {
            u8::from_str_radix(&expanded[6..8], 16).unwrap_or(255)
        } else {
            255
        },
    }
}

fn parse_css_channel(s: &str) -> u8 {
    s.parse::<f64>().unwrap_or(0.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{
        GradientKind, GradientStop, PaintBase, PaintClip, PaintGradient, PaintInstruction,
        PaintRect, PaintText, TextAlign,
    };
    use paint_vm_runtime::{
        PaintBackendPreference, PaintBackendRegistry, PaintFeature, PaintRenderOptions,
    };

    fn transparent_scene(width: f64, height: f64) -> PaintScene {
        PaintScene {
            width,
            height,
            background: "transparent".to_string(),
            instructions: Vec::new(),
            id: None,
            metadata: None,
        }
    }

    #[test]
    fn exposes_tier_one_descriptor() {
        let descriptor = descriptor();
        assert_eq!(descriptor.id, BACKEND_ID);
        assert_eq!(descriptor.family, PaintBackendFamily::Cairo);
        assert_eq!(descriptor.tier, PaintBackendTier::Tier1Smoke);
        assert_eq!(descriptor.capabilities.rect, SupportLevel::Supported);
        assert_eq!(descriptor.capabilities.text, SupportLevel::Degraded);
        assert_eq!(
            descriptor.capabilities.linear_gradient,
            SupportLevel::Supported
        );
        assert_eq!(
            descriptor.capabilities.radial_gradient,
            SupportLevel::Supported
        );
    }

    #[test]
    fn renders_filled_rectangles_to_pixels() {
        let mut scene = transparent_scene(8.0, 8.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                2.0, 2.0, 3.0, 3.0, "#ff0000",
            )));

        let pixels = render(&scene).expect("rect scene renders");

        assert_eq!(pixels.pixel_at(3, 3), (255, 0, 0, 255));
        assert_eq!(pixels.pixel_at(0, 0), (0, 0, 0, 0));
    }

    #[test]
    fn clips_child_instructions() {
        let mut scene = transparent_scene(6.0, 6.0);
        scene.instructions.push(PaintInstruction::Clip(PaintClip {
            base: PaintBase::default(),
            x: 2.0,
            y: 2.0,
            width: 2.0,
            height: 2.0,
            children: vec![PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 6.0, 6.0, "#00ff00",
            ))],
        }));

        let pixels = render(&scene).expect("clip scene renders");

        assert_eq!(pixels.pixel_at(2, 2), (0, 255, 0, 255));
        assert_eq!(pixels.pixel_at(1, 1), (0, 0, 0, 0));
        assert_eq!(pixels.pixel_at(4, 4), (0, 0, 0, 0));
    }

    #[test]
    fn renders_text_as_degraded_visible_glyph_blocks() {
        let mut scene = transparent_scene(64.0, 24.0);
        scene.instructions.push(PaintInstruction::Text(PaintText {
            base: PaintBase::default(),
            x: 32.0,
            y: 18.0,
            text: "Hi".to_string(),
            font_ref: None,
            font_size: 12.0,
            fill: Some("#0000ff".to_string()),
            text_align: Some(TextAlign::Center),
        }));

        let pixels = render(&scene).expect("text smoke scene renders");

        assert!(pixels
            .data
            .chunks_exact(4)
            .any(|pixel| pixel[2] > pixel[0] && pixel[2] > pixel[1] && pixel[3] > 0));
    }

    #[test]
    fn runtime_selects_cairo_for_exact_rect_scenes() {
        let backend = renderer();
        let mut registry = PaintBackendRegistry::new();
        registry.register(&backend);

        let mut scene = transparent_scene(4.0, 4.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 4.0, 4.0, "#111111",
            )));

        let pixels = registry
            .render_auto(&scene, PaintRenderOptions::default())
            .expect("runtime should select Cairo for rect scenes");

        assert_eq!(pixels.pixel_at(1, 1), (17, 17, 17, 255));
    }

    #[test]
    fn renders_linear_gradient_fills() {
        let mut scene = transparent_scene(20.0, 4.0);
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

        let pixels = render(&scene).expect("linear gradient scene renders");
        let (left, _, _, _) = pixels.pixel_at(1, 2);
        let (right, _, _, _) = pixels.pixel_at(18, 2);

        assert!(left < 80, "expected dark left edge, got {left}");
        assert!(right > 170, "expected bright right edge, got {right}");
        assert!(left < right);
    }

    #[test]
    fn renders_radial_gradient_fills() {
        let mut scene = transparent_scene(16.0, 16.0);
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase {
                    id: Some("spot".to_string()),
                    metadata: None,
                },
                kind: GradientKind::Radial {
                    cx: 8.0,
                    cy: 8.0,
                    r: 8.0,
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
            width: 16.0,
            height: 16.0,
            fill: Some("url(#spot)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let pixels = render(&scene).expect("radial gradient scene renders");
        let (center, _, _, _) = pixels.pixel_at(8, 8);
        let (corner, _, _, _) = pixels.pixel_at(0, 0);

        assert!(center < 80, "expected dark center, got {center}");
        assert!(corner > 170, "expected bright corner, got {corner}");
        assert!(center < corner);
    }

    #[test]
    fn runtime_selects_cairo_for_exact_gradient_scenes() {
        let backend = renderer();
        let mut registry = PaintBackendRegistry::new();
        registry.register(&backend);
        let mut scene = transparent_scene(8.0, 2.0);
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
                    x2: 8.0,
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
            width: 8.0,
            height: 2.0,
            fill: Some("url(#fade)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let selected = registry
            .select(
                &scene,
                PaintRenderOptions {
                    preference: PaintBackendPreference::Named(BACKEND_ID.to_string()),
                    ..PaintRenderOptions::default()
                },
            )
            .expect("Cairo should advertise exact gradient support");

        assert_eq!(selected.descriptor().id, BACKEND_ID);
    }

    #[test]
    fn runtime_requires_degraded_text_opt_in() {
        let backend = renderer();
        let mut registry = PaintBackendRegistry::new();
        registry.register(&backend);

        let mut scene = transparent_scene(32.0, 16.0);
        scene.instructions.push(PaintInstruction::Text(PaintText {
            base: PaintBase::default(),
            x: 0.0,
            y: 12.0,
            text: "A".to_string(),
            font_ref: None,
            font_size: 10.0,
            fill: Some("#000000".to_string()),
            text_align: None,
        }));

        let err = match registry.select(&scene, PaintRenderOptions::default()) {
            Ok(_) => panic!("exact text should reject degraded Cairo text"),
            Err(err) => err,
        };
        assert!(matches!(
            err,
            PaintRenderError::NoCompatibleBackend { missing, .. }
                if missing.contains(&PaintFeature::Text)
        ));

        let pixels = registry
            .render_auto(
                &scene,
                PaintRenderOptions {
                    preference: PaintBackendPreference::Named(BACKEND_ID.to_string()),
                    allow_degraded: true,
                    require_antialiasing: false,
                    require_exact_text: false,
                },
            )
            .expect("degraded text opt-in should render");
        assert!(pixels.data.chunks_exact(4).any(|pixel| pixel[3] != 0));
    }
}
