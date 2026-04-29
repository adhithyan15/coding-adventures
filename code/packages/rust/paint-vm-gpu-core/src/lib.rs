//! Shared GPU render-plan core for Paint VM backends.
//!
//! `paint-vm-gpu-core` lowers [`PaintScene`] into a backend-neutral plan that
//! Vulkan, OpenGL, WGPU, Mesa, and compute experiments can consume. It keeps
//! geometry interpretation in one place so backend crates can focus on API
//! plumbing, resource uploads, render passes, and readback.

use std::collections::HashMap;

use paint_instructions::{
    BlendMode, FillRule, GradientKind, GradientStop, ImageSrc, PaintClip, PaintEllipse,
    PaintGlyphRun, PaintGradient, PaintGroup, PaintImage, PaintInstruction, PaintLayer, PaintLine,
    PaintPath, PaintRect, PaintScene, PaintText, PathCommand, Transform2D, IDENTITY_TRANSFORM,
};

pub const VERSION: &str = "0.1.0";
const GRADIENT_RAMP_WIDTH: u32 = 1024;
const RADIAL_GRADIENT_TEXTURE_SIZE: u32 = 256;

#[derive(Clone, Debug, PartialEq)]
pub struct GpuPaintPlan {
    pub width: u32,
    pub height: u32,
    pub background: GpuColor,
    pub commands: Vec<GpuCommand>,
    pub meshes: Vec<GpuMesh>,
    pub images: Vec<GpuImageUpload>,
    pub diagnostics: Vec<GpuPlanDiagnostic>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GpuCommand {
    DrawMesh { mesh_id: usize },
    DrawText(GpuTextRun),
    DrawGlyphRun(GpuGlyphRun),
    PushClip { rect: GpuRect },
    PopClip,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GpuMesh {
    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u32>,
    pub texture_id: Option<usize>,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuVertex {
    pub position: GpuPoint,
    pub uv: [f32; 2],
    pub color: GpuColor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GpuImageUpload {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub filter: GpuTextureFilter,
    pub kind: GpuTextureKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuTextureFilter {
    Nearest,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuTextureKind {
    Image,
    LinearGradient,
    RadialGradient,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GpuTextRun {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub font_ref: Option<String>,
    pub font_size: f32,
    pub color: GpuColor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GpuGlyphRun {
    pub glyphs: Vec<GpuGlyphInstance>,
    pub font_ref: String,
    pub font_size: f32,
    pub color: GpuColor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GpuGlyphInstance {
    pub glyph_id: u32,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuPlanDiagnostic {
    pub severity: GpuPlanSeverity,
    pub feature: &'static str,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuPlanSeverity {
    Info,
    Degraded,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuPlanOptions {
    pub ellipse_segments: usize,
    pub curve_segments: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuApiFamily {
    Vulkan,
    OpenGl,
    Mesa,
    OpenCl,
    Wgpu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuRenderPath {
    GraphicsPipeline,
    ComputeRaster,
    DriverProfile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuReadbackStrategy {
    TextureCopyToBuffer,
    FramebufferReadPixels,
    StorageBufferReadback,
    DelegatedToProfile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuBackendProfile {
    pub id: &'static str,
    pub family: GpuApiFamily,
    pub render_path: GpuRenderPath,
    pub shader_model: &'static str,
    pub readback: GpuReadbackStrategy,
    pub supports_indexed_meshes: bool,
    pub supports_scissor_clips: bool,
    pub supports_texture_sampling: bool,
    pub supports_linear_gradients: bool,
    pub supports_radial_gradients: bool,
    pub supports_glyph_atlas: bool,
    pub accepts_degraded_solid_gradients: bool,
}

impl GpuBackendProfile {
    pub const fn tier1_solid(
        id: &'static str,
        family: GpuApiFamily,
        render_path: GpuRenderPath,
        shader_model: &'static str,
        readback: GpuReadbackStrategy,
    ) -> Self {
        Self {
            id,
            family,
            render_path,
            shader_model,
            readback,
            supports_indexed_meshes: true,
            supports_scissor_clips: true,
            supports_texture_sampling: false,
            supports_linear_gradients: false,
            supports_radial_gradients: false,
            supports_glyph_atlas: false,
            accepts_degraded_solid_gradients: true,
        }
    }
}

impl Default for GpuPlanOptions {
    fn default() -> Self {
        Self {
            ellipse_segments: 48,
            curve_segments: 16,
        }
    }
}

pub fn plan_scene(scene: &PaintScene) -> GpuPaintPlan {
    plan_scene_with_options(scene, GpuPlanOptions::default())
}

pub fn unsupported_plan_features(
    profile: GpuBackendProfile,
    plan: &GpuPaintPlan,
) -> Vec<&'static str> {
    let mut unsupported = Vec::new();
    for diagnostic in &plan.diagnostics {
        match diagnostic.severity {
            GpuPlanSeverity::Unsupported => push_unique(&mut unsupported, diagnostic.feature),
            GpuPlanSeverity::Degraded if !profile.accepts_degraded_solid_gradients => {
                push_unique(&mut unsupported, diagnostic.feature)
            }
            GpuPlanSeverity::Info | GpuPlanSeverity::Degraded => {}
        }
    }
    for command in &plan.commands {
        match command {
            GpuCommand::DrawMesh { .. } if !profile.supports_indexed_meshes => {
                push_unique(&mut unsupported, "mesh")
            }
            GpuCommand::PushClip { .. } | GpuCommand::PopClip
                if !profile.supports_scissor_clips =>
            {
                push_unique(&mut unsupported, "clip")
            }
            GpuCommand::DrawText(_) | GpuCommand::DrawGlyphRun(_)
                if !profile.supports_glyph_atlas =>
            {
                push_unique(&mut unsupported, "text")
            }
            _ => {}
        }
    }
    for image in &plan.images {
        match image.kind {
            GpuTextureKind::Image if !profile.supports_texture_sampling => {
                push_unique(&mut unsupported, "image")
            }
            GpuTextureKind::LinearGradient
                if !profile.supports_texture_sampling || !profile.supports_linear_gradients =>
            {
                push_unique(&mut unsupported, "gradient.linear")
            }
            GpuTextureKind::RadialGradient
                if !profile.supports_texture_sampling || !profile.supports_radial_gradients =>
            {
                push_unique(&mut unsupported, "gradient.radial")
            }
            _ => {}
        }
    }
    unsupported
}

fn push_unique(features: &mut Vec<&'static str>, feature: &'static str) {
    if !features.contains(&feature) {
        features.push(feature);
    }
}

pub fn plan_scene_with_options(scene: &PaintScene, options: GpuPlanOptions) -> GpuPaintPlan {
    let mut builder = PlanBuilder {
        options,
        gradients: collect_gradients(&scene.instructions),
        plan: GpuPaintPlan {
            width: scene.width.max(0.0).ceil() as u32,
            height: scene.height.max(0.0).ceil() as u32,
            background: parse_color(&scene.background),
            commands: Vec::new(),
            meshes: Vec::new(),
            images: Vec::new(),
            diagnostics: Vec::new(),
        },
    };
    builder.plan_instructions(&scene.instructions, IDENTITY_TRANSFORM, 1.0);
    builder.plan
}

struct PlanBuilder {
    options: GpuPlanOptions,
    gradients: HashMap<String, PaintGradient>,
    plan: GpuPaintPlan,
}

#[derive(Clone, Copy, Debug)]
enum PaintBrush {
    Solid(GpuColor),
    LinearGradient {
        texture_id: usize,
        start: GpuPoint,
        end: GpuPoint,
    },
    RadialGradient {
        texture_id: usize,
        center: GpuPoint,
        axis_x: GpuPoint,
        axis_y: GpuPoint,
    },
}

impl PaintBrush {
    fn vertex(self, position: GpuPoint) -> GpuVertex {
        match self {
            PaintBrush::Solid(color) => vertex(position, color),
            PaintBrush::LinearGradient {
                texture_id: _,
                start,
                end,
            } => vertex_uv(
                position,
                [linear_gradient_t(position, start, end), 0.5],
                GpuColor::white(),
            ),
            PaintBrush::RadialGradient {
                texture_id: _,
                center,
                axis_x,
                axis_y,
            } => vertex_uv(
                position,
                radial_gradient_uv(position, center, axis_x, axis_y),
                GpuColor::white(),
            ),
        }
    }

    fn texture_id(self) -> Option<usize> {
        match self {
            PaintBrush::Solid(_) => None,
            PaintBrush::LinearGradient { texture_id, .. } => Some(texture_id),
            PaintBrush::RadialGradient { texture_id, .. } => Some(texture_id),
        }
    }

    fn is_transparent(self) -> bool {
        matches!(self, PaintBrush::Solid(color) if color.a == 0.0)
    }
}

impl PlanBuilder {
    fn plan_instructions(
        &mut self,
        instructions: &[PaintInstruction],
        transform: Transform2D,
        opacity: f32,
    ) {
        for instruction in instructions {
            match instruction {
                PaintInstruction::Rect(rect) => self.plan_rect(rect, transform, opacity),
                PaintInstruction::Ellipse(ellipse) => {
                    self.plan_ellipse(ellipse, transform, opacity)
                }
                PaintInstruction::Path(path) => self.plan_path(path, transform, opacity),
                PaintInstruction::Text(text) => self.plan_text(text, transform, opacity),
                PaintInstruction::GlyphRun(run) => self.plan_glyph_run(run, transform, opacity),
                PaintInstruction::Group(group) => self.plan_group(group, transform, opacity),
                PaintInstruction::Layer(layer) => self.plan_layer(layer, transform, opacity),
                PaintInstruction::Line(line) => self.plan_line(line, transform, opacity),
                PaintInstruction::Clip(clip) => self.plan_clip(clip, transform, opacity),
                PaintInstruction::Gradient(_) => {}
                PaintInstruction::Image(image) => self.plan_image(image, transform, opacity),
            }
        }
    }

    fn plan_rect(&mut self, rect: &PaintRect, transform: Transform2D, opacity: f32) {
        if rect.corner_radius.unwrap_or(0.0) > 0.0 {
            self.diagnostic(
                GpuPlanSeverity::Degraded,
                "rect.corner_radius",
                "rounded rectangles are currently lowered as sharp rectangles",
            );
        }
        if let Some(brush) = self.paint_brush(rect.fill.as_deref(), opacity, transform) {
            self.add_rect_mesh(
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                transform,
                brush,
                "rect",
            );
        }
        if let Some(color) = self.stroke_color(rect.stroke.as_deref(), opacity) {
            self.warn_stroke_dash(rect.stroke_dash.as_deref());
            let w = rect.stroke_width.unwrap_or(1.0).max(0.0);
            if w > 0.0 {
                self.add_rect_stroke(rect.x, rect.y, rect.width, rect.height, w, transform, color);
            }
        }
    }

    fn plan_line(&mut self, line: &PaintLine, transform: Transform2D, opacity: f32) {
        self.warn_stroke_dash(line.stroke_dash.as_deref());
        let color = parse_color_with_opacity(&line.stroke, opacity);
        if color.a == 0.0 {
            return;
        }
        self.add_line_quad(
            point(line.x1, line.y1),
            point(line.x2, line.y2),
            line.stroke_width.unwrap_or(1.0).max(1.0) as f32,
            transform,
            color,
            "line",
        );
    }

    fn plan_ellipse(&mut self, ellipse: &PaintEllipse, transform: Transform2D, opacity: f32) {
        if let Some(brush) = self.paint_brush(ellipse.fill.as_deref(), opacity, transform) {
            let mut vertices = Vec::with_capacity(self.options.ellipse_segments + 1);
            vertices.push(brush.vertex(apply_transform(point(ellipse.cx, ellipse.cy), transform)));
            for i in 0..self.options.ellipse_segments {
                let t = i as f32 / self.options.ellipse_segments as f32 * std::f32::consts::TAU;
                vertices.push(brush.vertex(apply_transform(
                    GpuPoint {
                        x: ellipse.cx as f32 + ellipse.rx as f32 * t.cos(),
                        y: ellipse.cy as f32 + ellipse.ry as f32 * t.sin(),
                    },
                    transform,
                )));
            }
            let mut indices = Vec::with_capacity(self.options.ellipse_segments * 3);
            for i in 1..=self.options.ellipse_segments {
                indices.push(0);
                indices.push(i as u32);
                indices.push(if i == self.options.ellipse_segments {
                    1
                } else {
                    i as u32 + 1
                });
            }
            self.add_mesh(vertices, indices, brush.texture_id(), "ellipse.fill");
        }

        if let Some(color) = self.stroke_color(ellipse.stroke.as_deref(), opacity) {
            self.warn_stroke_dash(ellipse.stroke_dash.as_deref());
            let stroke_width = ellipse.stroke_width.unwrap_or(1.0).max(1.0) as f32;
            let mut vertices = Vec::with_capacity(self.options.ellipse_segments * 2);
            let mut indices = Vec::with_capacity(self.options.ellipse_segments * 6);
            let outer_rx = ellipse.rx as f32 + stroke_width / 2.0;
            let outer_ry = ellipse.ry as f32 + stroke_width / 2.0;
            let inner_rx = (ellipse.rx as f32 - stroke_width / 2.0).max(0.0);
            let inner_ry = (ellipse.ry as f32 - stroke_width / 2.0).max(0.0);
            for i in 0..self.options.ellipse_segments {
                let t = i as f32 / self.options.ellipse_segments as f32 * std::f32::consts::TAU;
                vertices.push(vertex(
                    apply_transform(
                        GpuPoint {
                            x: ellipse.cx as f32 + outer_rx * t.cos(),
                            y: ellipse.cy as f32 + outer_ry * t.sin(),
                        },
                        transform,
                    ),
                    color,
                ));
                vertices.push(vertex(
                    apply_transform(
                        GpuPoint {
                            x: ellipse.cx as f32 + inner_rx * t.cos(),
                            y: ellipse.cy as f32 + inner_ry * t.sin(),
                        },
                        transform,
                    ),
                    color,
                ));
            }
            for i in 0..self.options.ellipse_segments {
                let next = (i + 1) % self.options.ellipse_segments;
                let outer0 = (i * 2) as u32;
                let inner0 = outer0 + 1;
                let outer1 = (next * 2) as u32;
                let inner1 = outer1 + 1;
                indices.extend_from_slice(&[outer0, inner0, outer1, outer1, inner0, inner1]);
            }
            self.add_mesh(vertices, indices, None, "ellipse.stroke");
        }
    }

    fn plan_path(&mut self, path: &PaintPath, transform: Transform2D, opacity: f32) {
        let contours = self.flatten_path(path);
        if path.fill_rule == Some(FillRule::EvenOdd) {
            self.diagnostic(
                GpuPlanSeverity::Degraded,
                "path.fill_rule",
                "evenodd path filling is not exact in the simple GPU tessellator",
            );
        }
        if let Some(brush) = self.paint_brush(path.fill.as_deref(), opacity, transform) {
            for contour in &contours {
                if contour.points.len() >= 3 {
                    let base = contour.points[0];
                    let mut vertices = Vec::with_capacity(contour.points.len());
                    vertices.push(brush.vertex(apply_transform(base, transform)));
                    for point in contour.points.iter().skip(1) {
                        vertices.push(brush.vertex(apply_transform(*point, transform)));
                    }
                    let mut indices = Vec::new();
                    for i in 1..vertices.len().saturating_sub(1) {
                        indices.extend_from_slice(&[0, i as u32, i as u32 + 1]);
                    }
                    self.add_mesh(vertices, indices, brush.texture_id(), "path.fill");
                }
            }
        }
        if let Some(color) = self.stroke_color(path.stroke.as_deref(), opacity) {
            self.warn_stroke_dash(path.stroke_dash.as_deref());
            let stroke_width = path.stroke_width.unwrap_or(1.0).max(1.0) as f32;
            for contour in &contours {
                for segment in contour.points.windows(2) {
                    self.add_line_quad(
                        segment[0],
                        segment[1],
                        stroke_width,
                        transform,
                        color,
                        "path.stroke",
                    );
                }
                if contour.closed && contour.points.len() > 2 {
                    self.add_line_quad(
                        *contour.points.last().unwrap(),
                        contour.points[0],
                        stroke_width,
                        transform,
                        color,
                        "path.stroke",
                    );
                }
            }
        }
    }

    fn plan_text(&mut self, text: &PaintText, transform: Transform2D, opacity: f32) {
        let position = apply_transform(point(text.x, text.y), transform);
        self.plan.commands.push(GpuCommand::DrawText(GpuTextRun {
            text: text.text.clone(),
            x: position.x,
            y: position.y,
            font_ref: text.font_ref.clone(),
            font_size: text.font_size as f32,
            color: parse_color_with_opacity(text.fill.as_deref().unwrap_or("#000000"), opacity),
        }));
        self.diagnostic(
            GpuPlanSeverity::Info,
            "text",
            "text is preserved for backend glyph atlas/shaping rather than tessellated",
        );
    }

    fn plan_glyph_run(&mut self, run: &PaintGlyphRun, transform: Transform2D, opacity: f32) {
        let glyphs = run
            .glyphs
            .iter()
            .map(|glyph| {
                let p = apply_transform(point(glyph.x, glyph.y), transform);
                GpuGlyphInstance {
                    glyph_id: glyph.glyph_id,
                    x: p.x,
                    y: p.y,
                }
            })
            .collect();
        self.plan
            .commands
            .push(GpuCommand::DrawGlyphRun(GpuGlyphRun {
                glyphs,
                font_ref: run.font_ref.clone(),
                font_size: run.font_size as f32,
                color: parse_color_with_opacity(run.fill.as_deref().unwrap_or("#000000"), opacity),
            }));
    }

    fn plan_group(&mut self, group: &PaintGroup, transform: Transform2D, opacity: f32) {
        let next_transform = group
            .transform
            .map_or(transform, |local| multiply_transform(transform, local));
        let next_opacity = opacity * group.opacity.unwrap_or(1.0).clamp(0.0, 1.0) as f32;
        self.plan_instructions(&group.children, next_transform, next_opacity);
    }

    fn plan_layer(&mut self, layer: &PaintLayer, transform: Transform2D, opacity: f32) {
        if layer
            .filters
            .as_ref()
            .is_some_and(|filters| !filters.is_empty())
        {
            self.diagnostic(
                GpuPlanSeverity::Unsupported,
                "layer.filters",
                "GPU core preserves no filter graph yet",
            );
        }
        if !matches!(layer.blend_mode.as_ref(), None | Some(BlendMode::Normal)) {
            self.diagnostic(
                GpuPlanSeverity::Unsupported,
                "layer.blend_mode",
                "non-normal blend modes require backend render-pass support",
            );
        }
        let next_transform = layer
            .transform
            .map_or(transform, |local| multiply_transform(transform, local));
        let next_opacity = opacity * layer.opacity.unwrap_or(1.0).clamp(0.0, 1.0) as f32;
        self.plan_instructions(&layer.children, next_transform, next_opacity);
    }

    fn plan_clip(&mut self, clip: &PaintClip, transform: Transform2D, opacity: f32) {
        let rect = transformed_rect(clip.x, clip.y, clip.width, clip.height, transform);
        self.plan.commands.push(GpuCommand::PushClip { rect });
        self.plan_instructions(&clip.children, transform, opacity);
        self.plan.commands.push(GpuCommand::PopClip);
    }

    fn plan_image(&mut self, image: &PaintImage, transform: Transform2D, opacity: f32) {
        let ImageSrc::Pixels(pixels) = &image.src else {
            self.diagnostic(
                GpuPlanSeverity::Unsupported,
                "image.uri",
                "GPU core cannot decode ImageSrc::Uri; pass decoded pixels first",
            );
            return;
        };
        if pixels.width == 0 || pixels.height == 0 {
            return;
        }
        let texture_id = self.plan.images.len();
        self.plan.images.push(GpuImageUpload {
            width: pixels.width,
            height: pixels.height,
            data: pixels.data.clone(),
            filter: GpuTextureFilter::Nearest,
            kind: GpuTextureKind::Image,
        });
        let color = GpuColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: opacity * image.opacity.unwrap_or(1.0).clamp(0.0, 1.0) as f32,
        };
        let p0 = apply_transform(point(image.x, image.y), transform);
        let p1 = apply_transform(point(image.x + image.width, image.y), transform);
        let p2 = apply_transform(
            point(image.x + image.width, image.y + image.height),
            transform,
        );
        let p3 = apply_transform(point(image.x, image.y + image.height), transform);
        let vertices = vec![
            vertex_uv(p0, [0.0, 0.0], color),
            vertex_uv(p1, [1.0, 0.0], color),
            vertex_uv(p2, [1.0, 1.0], color),
            vertex_uv(p3, [0.0, 1.0], color),
        ];
        self.add_mesh(vertices, vec![0, 1, 2, 0, 2, 3], Some(texture_id), "image");
    }

    fn paint_brush(
        &mut self,
        paint: Option<&str>,
        opacity: f32,
        transform: Transform2D,
    ) -> Option<PaintBrush> {
        let paint = paint?;
        if paint.trim().eq_ignore_ascii_case("none") {
            return None;
        }
        if let Some(id) = gradient_ref(paint) {
            return self.gradient_brush(id, opacity, transform);
        }
        let color = parse_color_with_opacity(paint, opacity);
        (color.a > 0.0).then_some(PaintBrush::Solid(color))
    }

    fn gradient_brush(
        &mut self,
        id: &str,
        opacity: f32,
        transform: Transform2D,
    ) -> Option<PaintBrush> {
        let Some(gradient) = self.gradients.get(id).cloned() else {
            self.diagnostic(
                GpuPlanSeverity::Unsupported,
                "gradient",
                format!("gradient reference '{id}' does not resolve to a PaintGradient"),
            );
            return None;
        };
        if gradient.stops.is_empty() {
            self.diagnostic(
                GpuPlanSeverity::Unsupported,
                "gradient",
                format!("gradient reference '{id}' has no usable PaintGradient stop"),
            );
            return None;
        }
        match gradient.kind {
            GradientKind::Linear { x1, y1, x2, y2 } => {
                let start = apply_transform(point(x1, y1), transform);
                let end = apply_transform(point(x2, y2), transform);
                if same_point(start, end) {
                    self.diagnostic(
                        GpuPlanSeverity::Degraded,
                        "gradient.linear",
                        "zero-length linear gradient is lowered to its first stop color",
                    );
                    return gradient.stops.first().map(|stop| {
                        PaintBrush::Solid(parse_color_with_opacity(&stop.color, opacity))
                    });
                }
                let texture_id = self.plan.images.len();
                self.plan.images.push(GpuImageUpload {
                    width: GRADIENT_RAMP_WIDTH,
                    height: 1,
                    data: build_gradient_ramp(&gradient.stops, opacity),
                    filter: GpuTextureFilter::Linear,
                    kind: GpuTextureKind::LinearGradient,
                });
                Some(PaintBrush::LinearGradient {
                    texture_id,
                    start,
                    end,
                })
            }
            GradientKind::Radial { cx, cy, r } => {
                if r <= f64::EPSILON {
                    self.diagnostic(
                        GpuPlanSeverity::Degraded,
                        "gradient.radial",
                        "zero-radius radial gradient is lowered to its first stop color",
                    );
                    return gradient.stops.first().map(|stop| {
                        PaintBrush::Solid(parse_color_with_opacity(&stop.color, opacity))
                    });
                }
                let center = apply_transform(point(cx, cy), transform);
                let right = apply_transform(point(cx + r, cy), transform);
                let bottom = apply_transform(point(cx, cy + r), transform);
                let axis_x = GpuPoint {
                    x: right.x - center.x,
                    y: right.y - center.y,
                };
                let axis_y = GpuPoint {
                    x: bottom.x - center.x,
                    y: bottom.y - center.y,
                };
                if radial_basis_is_degenerate(axis_x, axis_y) {
                    self.diagnostic(
                        GpuPlanSeverity::Degraded,
                        "gradient.radial",
                        "degenerate transformed radial gradient is lowered to its first stop color",
                    );
                    return gradient.stops.first().map(|stop| {
                        PaintBrush::Solid(parse_color_with_opacity(&stop.color, opacity))
                    });
                }
                let texture_id = self.plan.images.len();
                self.plan.images.push(GpuImageUpload {
                    width: RADIAL_GRADIENT_TEXTURE_SIZE,
                    height: RADIAL_GRADIENT_TEXTURE_SIZE,
                    data: build_radial_gradient_texture(&gradient.stops, opacity),
                    filter: GpuTextureFilter::Linear,
                    kind: GpuTextureKind::RadialGradient,
                });
                Some(PaintBrush::RadialGradient {
                    texture_id,
                    center,
                    axis_x,
                    axis_y,
                })
            }
        }
    }

    fn paint_color(&mut self, paint: Option<&str>, opacity: f32) -> Option<GpuColor> {
        let paint = paint?;
        if paint.trim().eq_ignore_ascii_case("none") {
            return None;
        }
        if let Some(id) = gradient_ref(paint) {
            let Some(first_stop_color) = self
                .gradients
                .get(id)
                .and_then(|gradient| gradient.stops.first())
                .map(|stop| stop.color.clone())
            else {
                self.diagnostic(
                    GpuPlanSeverity::Unsupported,
                    "gradient",
                    format!("gradient reference '{id}' has no usable PaintGradient stop"),
                );
                return None;
            };
            self.diagnostic(
                GpuPlanSeverity::Degraded,
                "gradient.stroke",
                "gradient strokes are currently lowered to their first stop color",
            );
            return Some(parse_color_with_opacity(&first_stop_color, opacity));
        }
        let color = parse_color_with_opacity(paint, opacity);
        (color.a > 0.0).then_some(color)
    }

    fn stroke_color(&mut self, stroke: Option<&str>, opacity: f32) -> Option<GpuColor> {
        self.paint_color(stroke, opacity)
    }

    fn add_rect_stroke(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        stroke_width: f64,
        transform: Transform2D,
        color: GpuColor,
    ) {
        let w = stroke_width;
        let brush = PaintBrush::Solid(color);
        self.add_rect_mesh(x, y, width, w, transform, brush, "rect.stroke");
        self.add_rect_mesh(x, y + height - w, width, w, transform, brush, "rect.stroke");
        self.add_rect_mesh(x, y, w, height, transform, brush, "rect.stroke");
        self.add_rect_mesh(x + width - w, y, w, height, transform, brush, "rect.stroke");
    }

    fn add_rect_mesh(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        transform: Transform2D,
        brush: PaintBrush,
        label: &'static str,
    ) {
        if width <= 0.0 || height <= 0.0 || brush.is_transparent() {
            return;
        }
        let p0 = apply_transform(point(x, y), transform);
        let p1 = apply_transform(point(x + width, y), transform);
        let p2 = apply_transform(point(x + width, y + height), transform);
        let p3 = apply_transform(point(x, y + height), transform);
        self.add_mesh(
            vec![
                brush.vertex(p0),
                brush.vertex(p1),
                brush.vertex(p2),
                brush.vertex(p3),
            ],
            vec![0, 1, 2, 0, 2, 3],
            brush.texture_id(),
            label,
        );
    }

    fn add_line_quad(
        &mut self,
        p0: GpuPoint,
        p1: GpuPoint,
        width: f32,
        transform: Transform2D,
        color: GpuColor,
        label: &'static str,
    ) {
        let dx = p1.x - p0.x;
        let dy = p1.y - p0.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= f32::EPSILON || color.a == 0.0 {
            return;
        }
        let nx = -dy / len * width / 2.0;
        let ny = dx / len * width / 2.0;
        let a = apply_transform(
            GpuPoint {
                x: p0.x + nx,
                y: p0.y + ny,
            },
            transform,
        );
        let b = apply_transform(
            GpuPoint {
                x: p1.x + nx,
                y: p1.y + ny,
            },
            transform,
        );
        let c = apply_transform(
            GpuPoint {
                x: p1.x - nx,
                y: p1.y - ny,
            },
            transform,
        );
        let d = apply_transform(
            GpuPoint {
                x: p0.x - nx,
                y: p0.y - ny,
            },
            transform,
        );
        self.add_mesh(
            vec![
                vertex(a, color),
                vertex(b, color),
                vertex(c, color),
                vertex(d, color),
            ],
            vec![0, 1, 2, 0, 2, 3],
            None,
            label,
        );
    }

    fn add_mesh(
        &mut self,
        vertices: Vec<GpuVertex>,
        indices: Vec<u32>,
        texture_id: Option<usize>,
        label: &'static str,
    ) {
        let mesh_id = self.plan.meshes.len();
        self.plan.meshes.push(GpuMesh {
            vertices,
            indices,
            texture_id,
            label,
        });
        self.plan.commands.push(GpuCommand::DrawMesh { mesh_id });
    }

    fn flatten_path(&mut self, path: &PaintPath) -> Vec<Contour> {
        let mut contours = Vec::new();
        let mut current = Vec::new();
        let mut current_point = GpuPoint { x: 0.0, y: 0.0 };
        let mut contour_start = current_point;
        let mut closed = false;

        for command in &path.commands {
            match *command {
                PathCommand::MoveTo { x, y } => {
                    push_contour(&mut contours, &mut current, closed);
                    closed = false;
                    current_point = point(x, y);
                    contour_start = current_point;
                    current.push(current_point);
                }
                PathCommand::LineTo { x, y } => {
                    current_point = point(x, y);
                    current.push(current_point);
                }
                PathCommand::QuadTo { cx, cy, x, y } => {
                    let start = current_point;
                    let control = point(cx, cy);
                    let end = point(x, y);
                    for i in 1..=self.options.curve_segments {
                        let t = i as f32 / self.options.curve_segments as f32;
                        current.push(quad_point(start, control, end, t));
                    }
                    current_point = end;
                }
                PathCommand::CubicTo {
                    cx1,
                    cy1,
                    cx2,
                    cy2,
                    x,
                    y,
                } => {
                    let start = current_point;
                    let c1 = point(cx1, cy1);
                    let c2 = point(cx2, cy2);
                    let end = point(x, y);
                    for i in 1..=self.options.curve_segments {
                        let t = i as f32 / self.options.curve_segments as f32;
                        current.push(cubic_point(start, c1, c2, end, t));
                    }
                    current_point = end;
                }
                PathCommand::ArcTo { x, y, .. } => {
                    self.diagnostic(
                        GpuPlanSeverity::Degraded,
                        "path.arc_to",
                        "ArcTo is currently lowered to a straight line in GPU core",
                    );
                    current_point = point(x, y);
                    current.push(current_point);
                }
                PathCommand::Close => {
                    if current.last().copied() != Some(contour_start) {
                        current.push(contour_start);
                    }
                    closed = true;
                }
            }
        }
        push_contour(&mut contours, &mut current, closed);
        contours
    }

    fn warn_stroke_dash(&mut self, dash: Option<&[f64]>) {
        if dash.is_some_and(|dash| !dash.is_empty()) {
            self.diagnostic(
                GpuPlanSeverity::Degraded,
                "stroke_dash",
                "dashed strokes are currently lowered as solid strokes",
            );
        }
    }

    fn diagnostic(
        &mut self,
        severity: GpuPlanSeverity,
        feature: &'static str,
        message: impl Into<String>,
    ) {
        self.plan.diagnostics.push(GpuPlanDiagnostic {
            severity,
            feature,
            message: message.into(),
        });
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Contour {
    points: Vec<GpuPoint>,
    closed: bool,
}

fn push_contour(contours: &mut Vec<Contour>, current: &mut Vec<GpuPoint>, closed: bool) {
    if current.len() >= 2 {
        contours.push(Contour {
            points: std::mem::take(current),
            closed,
        });
    } else {
        current.clear();
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

fn build_gradient_ramp(stops: &[GradientStop], opacity: f32) -> Vec<u8> {
    let stops = normalized_gradient_stops(stops, opacity);

    let mut data = Vec::with_capacity(GRADIENT_RAMP_WIDTH as usize * 4);
    for i in 0..GRADIENT_RAMP_WIDTH {
        let t = if GRADIENT_RAMP_WIDTH <= 1 {
            0.0
        } else {
            i as f32 / (GRADIENT_RAMP_WIDTH - 1) as f32
        };
        let color = sample_gradient_stops(&stops, t);
        data.extend_from_slice(&color_to_rgba8(color));
    }
    data
}

fn build_radial_gradient_texture(stops: &[GradientStop], opacity: f32) -> Vec<u8> {
    let stops = normalized_gradient_stops(stops, opacity);
    let size = RADIAL_GRADIENT_TEXTURE_SIZE;
    let mut data = Vec::with_capacity(size as usize * size as usize * 4);
    for y in 0..size {
        for x in 0..size {
            let u = (x as f32 + 0.5) / size as f32;
            let v = (y as f32 + 0.5) / size as f32;
            let dx = u - 0.5;
            let dy = v - 0.5;
            let t = (dx * dx + dy * dy).sqrt() * 2.0;
            data.extend_from_slice(&color_to_rgba8(sample_gradient_stops(&stops, t)));
        }
    }
    data
}

fn normalized_gradient_stops(stops: &[GradientStop], opacity: f32) -> Vec<(f32, GpuColor)> {
    let mut stops: Vec<(f32, GpuColor)> = stops
        .iter()
        .map(|stop| {
            (
                stop.offset.clamp(0.0, 1.0) as f32,
                parse_color_with_opacity(&stop.color, opacity),
            )
        })
        .collect();
    stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    stops
}

fn sample_gradient_stops(stops: &[(f32, GpuColor)], t: f32) -> GpuColor {
    if stops.is_empty() {
        return GpuColor::transparent();
    }
    if t <= stops[0].0 {
        return stops[0].1;
    }
    for pair in stops.windows(2) {
        let (left_offset, left_color) = pair[0];
        let (right_offset, right_color) = pair[1];
        if t <= right_offset {
            let width = (right_offset - left_offset).max(f32::EPSILON);
            return mix_color(left_color, right_color, (t - left_offset) / width);
        }
    }
    stops
        .last()
        .map(|(_, color)| *color)
        .unwrap_or_else(GpuColor::transparent)
}

fn mix_color(a: GpuColor, b: GpuColor, t: f32) -> GpuColor {
    let t = t.clamp(0.0, 1.0);
    GpuColor {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

fn color_to_rgba8(color: GpuColor) -> [u8; 4] {
    [
        float_to_u8(color.r),
        float_to_u8(color.g),
        float_to_u8(color.b),
        float_to_u8(color.a),
    ]
}

fn float_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn linear_gradient_t(position: GpuPoint, start: GpuPoint, end: GpuPoint) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len2 = dx * dx + dy * dy;
    if len2 <= f32::EPSILON {
        return 0.0;
    }
    (((position.x - start.x) * dx + (position.y - start.y) * dy) / len2).clamp(0.0, 1.0)
}

fn radial_gradient_uv(
    position: GpuPoint,
    center: GpuPoint,
    axis_x: GpuPoint,
    axis_y: GpuPoint,
) -> [f32; 2] {
    let det = radial_basis_determinant(axis_x, axis_y);
    if det.abs() <= f32::EPSILON {
        return [0.5, 0.5];
    }
    let px = position.x - center.x;
    let py = position.y - center.y;
    let local_x = (px * axis_y.y - py * axis_y.x) / det;
    let local_y = (axis_x.x * py - axis_x.y * px) / det;
    [0.5 + local_x * 0.5, 0.5 + local_y * 0.5]
}

fn radial_basis_is_degenerate(axis_x: GpuPoint, axis_y: GpuPoint) -> bool {
    radial_basis_determinant(axis_x, axis_y).abs() <= f32::EPSILON
}

fn radial_basis_determinant(axis_x: GpuPoint, axis_y: GpuPoint) -> f32 {
    axis_x.x * axis_y.y - axis_x.y * axis_y.x
}

fn same_point(a: GpuPoint, b: GpuPoint) -> bool {
    (a.x - b.x).abs() <= f32::EPSILON && (a.y - b.y).abs() <= f32::EPSILON
}

fn parse_color_with_opacity(color: &str, opacity: f32) -> GpuColor {
    let mut parsed = parse_color(color);
    parsed.a *= opacity.clamp(0.0, 1.0);
    parsed
}

fn parse_color(color: &str) -> GpuColor {
    let s = color.trim();
    if s.eq_ignore_ascii_case("transparent") || s.eq_ignore_ascii_case("none") {
        return GpuColor::transparent();
    }
    if let Some(inner) = s.strip_prefix("rgba(").and_then(|v| v.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 4 {
            return GpuColor {
                r: parse_css_channel(parts[0]) as f32,
                g: parse_css_channel(parts[1]) as f32,
                b: parse_css_channel(parts[2]) as f32,
                a: parts[3].parse::<f32>().unwrap_or(1.0).clamp(0.0, 1.0),
            };
        }
    }
    if let Some(inner) = s.strip_prefix("rgb(").and_then(|v| v.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 3 {
            return GpuColor {
                r: parse_css_channel(parts[0]) as f32,
                g: parse_css_channel(parts[1]) as f32,
                b: parse_css_channel(parts[2]) as f32,
                a: 1.0,
            };
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
        return GpuColor {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
    }
    GpuColor {
        r: u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0,
        g: u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0,
        b: u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0,
        a: if hex.len() >= 8 {
            u8::from_str_radix(&hex[6..8], 16).unwrap_or(255) as f32 / 255.0
        } else {
            1.0
        },
    }
}

fn parse_css_channel(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or(0.0).clamp(0.0, 255.0) / 255.0
}

impl GpuColor {
    pub const fn transparent() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
    }

    pub const fn white() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
}

fn point(x: f64, y: f64) -> GpuPoint {
    GpuPoint {
        x: x as f32,
        y: y as f32,
    }
}

fn vertex(position: GpuPoint, color: GpuColor) -> GpuVertex {
    vertex_uv(position, [0.0, 0.0], color)
}

fn vertex_uv(position: GpuPoint, uv: [f32; 2], color: GpuColor) -> GpuVertex {
    GpuVertex {
        position,
        uv,
        color,
    }
}

fn apply_transform(point: GpuPoint, transform: Transform2D) -> GpuPoint {
    GpuPoint {
        x: transform[0] as f32 * point.x + transform[2] as f32 * point.y + transform[4] as f32,
        y: transform[1] as f32 * point.x + transform[3] as f32 * point.y + transform[5] as f32,
    }
}

fn multiply_transform(parent: Transform2D, local: Transform2D) -> Transform2D {
    [
        parent[0] * local[0] + parent[2] * local[1],
        parent[1] * local[0] + parent[3] * local[1],
        parent[0] * local[2] + parent[2] * local[3],
        parent[1] * local[2] + parent[3] * local[3],
        parent[0] * local[4] + parent[2] * local[5] + parent[4],
        parent[1] * local[4] + parent[3] * local[5] + parent[5],
    ]
}

fn transformed_rect(x: f64, y: f64, width: f64, height: f64, transform: Transform2D) -> GpuRect {
    let points = [
        apply_transform(point(x, y), transform),
        apply_transform(point(x + width, y), transform),
        apply_transform(point(x + width, y + height), transform),
        apply_transform(point(x, y + height), transform),
    ];
    let min_x = points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let min_y = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max_x = points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let max_y = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
    GpuRect {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    }
}

fn quad_point(p0: GpuPoint, p1: GpuPoint, p2: GpuPoint, t: f32) -> GpuPoint {
    let mt = 1.0 - t;
    GpuPoint {
        x: mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x,
        y: mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y,
    }
}

fn cubic_point(p0: GpuPoint, p1: GpuPoint, p2: GpuPoint, p3: GpuPoint, t: f32) -> GpuPoint {
    let mt = 1.0 - t;
    GpuPoint {
        x: mt.powi(3) * p0.x
            + 3.0 * mt.powi(2) * t * p1.x
            + 3.0 * mt * t.powi(2) * p2.x
            + t.powi(3) * p3.x,
        y: mt.powi(3) * p0.y
            + 3.0 * mt.powi(2) * t * p1.y
            + 3.0 * mt * t.powi(2) * p2.y
            + t.powi(3) * p3.y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{
        GlyphPosition, GradientKind, GradientStop, PaintBase, PaintGradient, PaintGroup,
        PaintImage, PaintInstruction, PaintRect, PixelContainer,
    };

    #[test]
    fn plans_rect_as_indexed_mesh() {
        let mut scene = PaintScene::new(20.0, 10.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                1.0, 2.0, 8.0, 4.0, "#ff0000",
            )));

        let plan = plan_scene(&scene);
        assert_eq!((plan.width, plan.height), (20, 10));
        assert_eq!(plan.meshes.len(), 1);
        assert_eq!(plan.meshes[0].vertices.len(), 4);
        assert_eq!(plan.meshes[0].indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(plan.commands, vec![GpuCommand::DrawMesh { mesh_id: 0 }]);
    }

    #[test]
    fn folds_group_transform_into_vertices() {
        let mut scene = PaintScene::new(40.0, 20.0);
        scene.instructions.push(PaintInstruction::Group(PaintGroup {
            base: PaintBase::default(),
            children: vec![PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 10.0, 10.0, "#000000",
            ))],
            transform: Some([1.0, 0.0, 0.0, 1.0, 12.0, 5.0]),
            opacity: Some(0.5),
        }));

        let plan = plan_scene(&scene);
        assert_eq!(
            plan.meshes[0].vertices[0].position,
            GpuPoint { x: 12.0, y: 5.0 }
        );
        assert_eq!(plan.meshes[0].vertices[0].color.a, 0.5);
    }

    #[test]
    fn emits_clip_push_and_pop() {
        let mut scene = PaintScene::new(40.0, 40.0);
        scene.instructions.push(PaintInstruction::Clip(PaintClip {
            base: PaintBase::default(),
            x: 5.0,
            y: 6.0,
            width: 10.0,
            height: 12.0,
            children: vec![PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 40.0, 40.0, "#000000",
            ))],
        }));

        let plan = plan_scene(&scene);
        assert!(matches!(
            plan.commands.first(),
            Some(GpuCommand::PushClip { .. })
        ));
        assert!(matches!(plan.commands.last(), Some(GpuCommand::PopClip)));
    }

    #[test]
    fn lowers_line_to_quad_mesh() {
        let mut scene = PaintScene::new(20.0, 20.0);
        scene.instructions.push(PaintInstruction::Line(PaintLine {
            base: PaintBase::default(),
            x1: 2.0,
            y1: 10.0,
            x2: 18.0,
            y2: 10.0,
            stroke: "#000000".to_string(),
            stroke_width: Some(4.0),
            stroke_cap: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);
        assert_eq!(plan.meshes[0].vertices.len(), 4);
        assert_eq!(plan.meshes[0].label, "line");
    }

    #[test]
    fn uploads_pixel_images_and_draws_textured_quad() {
        let mut pixels = PixelContainer::new(1, 1);
        pixels.set_pixel(0, 0, 12, 34, 56, 255);
        let mut scene = PaintScene::new(10.0, 10.0);
        scene.instructions.push(PaintInstruction::Image(PaintImage {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            src: ImageSrc::Pixels(pixels),
            opacity: Some(0.75),
        }));

        let plan = plan_scene(&scene);
        assert_eq!(plan.images.len(), 1);
        assert_eq!(plan.images[0].kind, GpuTextureKind::Image);
        assert_eq!(plan.images[0].filter, GpuTextureFilter::Nearest);
        assert_eq!(plan.meshes[0].texture_id, Some(0));
        assert_eq!(plan.meshes[0].vertices[0].color.a, 0.75);
    }

    #[test]
    fn preserves_text_and_glyph_commands() {
        let mut scene = PaintScene::new(100.0, 40.0);
        scene.instructions.push(PaintInstruction::Text(PaintText {
            base: PaintBase::default(),
            x: 10.0,
            y: 20.0,
            text: "GPU".to_string(),
            font_ref: None,
            font_size: 16.0,
            fill: Some("#000000".to_string()),
            text_align: None,
        }));
        scene
            .instructions
            .push(PaintInstruction::GlyphRun(PaintGlyphRun {
                base: PaintBase::default(),
                glyphs: vec![GlyphPosition {
                    glyph_id: 42,
                    x: 12.0,
                    y: 22.0,
                }],
                font_ref: "canvas:system-ui@16:400".to_string(),
                font_size: 16.0,
                fill: Some("#000000".to_string()),
            }));

        let plan = plan_scene(&scene);
        assert!(matches!(plan.commands[0], GpuCommand::DrawText(_)));
        assert!(matches!(plan.commands[1], GpuCommand::DrawGlyphRun(_)));
    }

    #[test]
    fn flattens_cubic_path() {
        let mut scene = PaintScene::new(100.0, 100.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 0.0, y: 0.0 },
                PathCommand::CubicTo {
                    cx1: 10.0,
                    cy1: 20.0,
                    cx2: 30.0,
                    cy2: 40.0,
                    x: 50.0,
                    y: 60.0,
                },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(1.0),
            stroke_cap: None,
            stroke_join: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan_scene_with_options(
            &scene,
            GpuPlanOptions {
                curve_segments: 4,
                ellipse_segments: 12,
            },
        );
        assert_eq!(plan.meshes.len(), 4);
    }

    #[test]
    fn lowers_linear_gradient_to_texture_ramp() {
        let mut scene = PaintScene::new(10.0, 10.0);
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
                    x2: 10.0,
                    y2: 0.0,
                },
                stops: vec![
                    GradientStop {
                        offset: 0.0,
                        color: "#ff0000".to_string(),
                    },
                    GradientStop {
                        offset: 1.0,
                        color: "#0000ff".to_string(),
                    },
                ],
            }));
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            fill: Some("url(#fade)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);
        assert_eq!(plan.images.len(), 1);
        assert_eq!(plan.images[0].kind, GpuTextureKind::LinearGradient);
        assert_eq!(plan.images[0].filter, GpuTextureFilter::Linear);
        assert_eq!(plan.meshes[0].texture_id, Some(0));
        assert_eq!(plan.meshes[0].vertices[0].uv[0], 0.0);
        assert_eq!(plan.meshes[0].vertices[1].uv[0], 1.0);
        assert!(plan.diagnostics.is_empty());
    }

    #[test]
    fn lowers_radial_gradient_to_texture() {
        let mut scene = PaintScene::new(10.0, 10.0);
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase {
                    id: Some("fade".to_string()),
                    metadata: None,
                },
                kind: GradientKind::Radial {
                    cx: 5.0,
                    cy: 5.0,
                    r: 5.0,
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
            width: 10.0,
            height: 10.0,
            fill: Some("url(#fade)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);
        assert_eq!(plan.images.len(), 1);
        assert_eq!(plan.images[0].kind, GpuTextureKind::RadialGradient);
        assert_eq!(plan.images[0].filter, GpuTextureFilter::Linear);
        assert_eq!(plan.images[0].width, RADIAL_GRADIENT_TEXTURE_SIZE);
        assert_eq!(plan.images[0].height, RADIAL_GRADIENT_TEXTURE_SIZE);
        assert_eq!(plan.meshes[0].texture_id, Some(0));
        assert_eq!(plan.meshes[0].vertices[0].uv, [0.0, 0.0]);
        assert_eq!(plan.meshes[0].vertices[2].uv, [1.0, 1.0]);
        let center_index = ((RADIAL_GRADIENT_TEXTURE_SIZE / 2 * RADIAL_GRADIENT_TEXTURE_SIZE
            + RADIAL_GRADIENT_TEXTURE_SIZE / 2)
            * 4) as usize;
        assert!(plan.images[0].data[center_index] < 5);
        assert!(plan.images[0].data[0] > 240);
        assert!(plan.diagnostics.is_empty());
    }

    #[test]
    fn degrades_zero_radius_radial_gradient_to_first_stop_with_diagnostic() {
        let mut scene = PaintScene::new(10.0, 10.0);
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase {
                    id: Some("fade".to_string()),
                    metadata: None,
                },
                kind: GradientKind::Radial {
                    cx: 5.0,
                    cy: 5.0,
                    r: 0.0,
                },
                stops: vec![GradientStop {
                    offset: 0.0,
                    color: "#ff0000".to_string(),
                }],
            }));
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            fill: Some("url(#fade)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);
        assert_eq!(plan.images.len(), 0);
        assert_eq!(plan.meshes[0].vertices[0].color.r, 1.0);
        assert!(plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.feature == "gradient.radial"));
    }

    #[test]
    fn tier1_solid_profile_accepts_basic_mesh_plan() {
        let mut scene = PaintScene::new(20.0, 10.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                1.0, 2.0, 8.0, 4.0, "#ff0000",
            )));
        let profile = GpuBackendProfile::tier1_solid(
            "paint-vm-test-gpu",
            GpuApiFamily::Wgpu,
            GpuRenderPath::GraphicsPipeline,
            "test-shader",
            GpuReadbackStrategy::TextureCopyToBuffer,
        );

        let plan = plan_scene(&scene);

        assert!(unsupported_plan_features(profile, &plan).is_empty());
    }

    #[test]
    fn tier1_solid_profile_rejects_gradient_textures() {
        let mut scene = PaintScene::new(10.0, 10.0);
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase {
                    id: Some("fade".to_string()),
                    metadata: None,
                },
                kind: GradientKind::Radial {
                    cx: 5.0,
                    cy: 5.0,
                    r: 5.0,
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
            width: 10.0,
            height: 10.0,
            fill: Some("url(#fade)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
        let profile = GpuBackendProfile::tier1_solid(
            "paint-vm-test-gpu",
            GpuApiFamily::Wgpu,
            GpuRenderPath::GraphicsPipeline,
            "test-shader",
            GpuReadbackStrategy::TextureCopyToBuffer,
        );

        let plan = plan_scene(&scene);
        let unsupported = unsupported_plan_features(profile, &plan);

        assert_eq!(unsupported, vec!["gradient.radial"]);
    }

    #[test]
    fn tier1_solid_profile_rejects_textures_and_text() {
        let mut pixels = PixelContainer::new(1, 1);
        pixels.set_pixel(0, 0, 12, 34, 56, 255);
        let mut scene = PaintScene::new(20.0, 20.0);
        scene.instructions.push(PaintInstruction::Image(PaintImage {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            src: ImageSrc::Pixels(pixels),
            opacity: None,
        }));
        scene.instructions.push(PaintInstruction::Text(PaintText {
            base: PaintBase::default(),
            x: 1.0,
            y: 12.0,
            text: "GPU".to_string(),
            font_ref: None,
            font_size: 12.0,
            fill: Some("#000000".to_string()),
            text_align: None,
        }));
        let profile = GpuBackendProfile::tier1_solid(
            "paint-vm-test-gpu",
            GpuApiFamily::Wgpu,
            GpuRenderPath::GraphicsPipeline,
            "test-shader",
            GpuReadbackStrategy::TextureCopyToBuffer,
        );

        let plan = plan_scene(&scene);
        let unsupported = unsupported_plan_features(profile, &plan);

        assert!(unsupported.contains(&"image"));
        assert!(unsupported.contains(&"text"));
    }
}
