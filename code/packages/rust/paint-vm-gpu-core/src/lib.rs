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
    PaintPath, PaintRect, PaintScene, PaintText, PathCommand, StrokeCap, StrokeJoin, Transform2D,
    IDENTITY_TRANSFORM,
};

pub const VERSION: &str = "0.1.0";
const GRADIENT_RAMP_WIDTH: u32 = 1024;
const RADIAL_GRADIENT_TEXTURE_SIZE: u32 = 256;
const DEFAULT_MITER_LIMIT: f32 = 4.0;

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

    pub const fn tier1_textured(
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
            supports_texture_sampling: true,
            supports_linear_gradients: true,
            supports_radial_gradients: true,
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
            let w = rect.stroke_width.unwrap_or(1.0).max(0.0);
            if w > 0.0 {
                if let Some(dash) = normalized_dash_pattern(rect.stroke_dash.as_deref()) {
                    self.add_dashed_rect_stroke(
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        w,
                        &dash,
                        rect.stroke_dash_offset.unwrap_or(0.0) as f32,
                        transform,
                        color,
                    );
                } else {
                    self.add_rect_stroke(
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        w,
                        transform,
                        color,
                    );
                }
            }
        }
    }

    fn plan_line(&mut self, line: &PaintLine, transform: Transform2D, opacity: f32) {
        let color = parse_color_with_opacity(&line.stroke, opacity);
        if color.a == 0.0 {
            return;
        }
        self.add_stroked_line(
            point(line.x1, line.y1),
            point(line.x2, line.y2),
            line.stroke_width.unwrap_or(1.0).max(1.0) as f32,
            line.stroke_cap.as_ref(),
            line.stroke_dash.as_deref(),
            line.stroke_dash_offset.unwrap_or(0.0) as f32,
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
            let stroke_width = ellipse.stroke_width.unwrap_or(1.0).max(1.0) as f32;
            if let Some(dash) = normalized_dash_pattern(ellipse.stroke_dash.as_deref()) {
                self.add_dashed_ellipse_stroke(
                    ellipse,
                    stroke_width,
                    &dash,
                    ellipse.stroke_dash_offset.unwrap_or(0.0) as f32,
                    transform,
                    color,
                );
                return;
            }
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
            let stroke_width = path.stroke_width.unwrap_or(1.0).max(1.0) as f32;
            let dash = normalized_dash_pattern(path.stroke_dash.as_deref());
            let cap = path.stroke_cap.as_ref().unwrap_or(&StrokeCap::Butt);
            let join = path.stroke_join.as_ref().unwrap_or(&StrokeJoin::Miter);
            for contour in &contours {
                if let Some(dash) = dash.as_deref() {
                    self.add_dashed_path_contour(
                        contour,
                        stroke_width,
                        dash,
                        path.stroke_dash_offset.unwrap_or(0.0) as f32,
                        cap,
                        join,
                        DEFAULT_MITER_LIMIT,
                        transform,
                        color,
                    );
                } else {
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
                    if !contour.closed {
                        self.add_open_contour_caps(contour, stroke_width, cap, transform, color);
                    }
                    self.add_contour_joins(
                        contour,
                        stroke_width,
                        join,
                        DEFAULT_MITER_LIMIT,
                        transform,
                        color,
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

    fn add_dashed_rect_stroke(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        stroke_width: f64,
        dash: &[f32],
        dash_offset: f32,
        transform: Transform2D,
        color: GpuColor,
    ) {
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let (mut dash_index, mut dash_offset) = dash_start(dash, dash_offset);
        let sides = [
            (point(x, y), point(x + width, y)),
            (point(x + width, y), point(x + width, y + height)),
            (point(x + width, y + height), point(x, y + height)),
            (point(x, y + height), point(x, y)),
        ];
        for (p0, p1) in sides {
            self.add_dashed_line_segment(
                p0,
                p1,
                stroke_width as f32,
                dash,
                &mut dash_index,
                &mut dash_offset,
                &StrokeCap::Butt,
                transform,
                color,
                "rect.stroke.dash",
            );
        }
    }

    fn add_dashed_ellipse_stroke(
        &mut self,
        ellipse: &PaintEllipse,
        stroke_width: f32,
        dash: &[f32],
        dash_offset: f32,
        transform: Transform2D,
        color: GpuColor,
    ) {
        if ellipse.rx <= 0.0 || ellipse.ry <= 0.0 {
            return;
        }
        let (mut dash_index, mut dash_offset) = dash_start(dash, dash_offset);
        let segments = self.options.ellipse_segments.max(4);
        let mut previous = ellipse_point(ellipse, 0.0);
        for i in 1..=segments {
            let t = i as f32 / segments as f32 * std::f32::consts::TAU;
            let next = ellipse_point(ellipse, t);
            self.add_dashed_line_segment(
                previous,
                next,
                stroke_width,
                dash,
                &mut dash_index,
                &mut dash_offset,
                &StrokeCap::Butt,
                transform,
                color,
                "ellipse.stroke.dash",
            );
            previous = next;
        }
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

    fn add_stroked_line(
        &mut self,
        p0: GpuPoint,
        p1: GpuPoint,
        width: f32,
        cap: Option<&StrokeCap>,
        dash: Option<&[f64]>,
        dash_offset: f32,
        transform: Transform2D,
        color: GpuColor,
        label: &'static str,
    ) {
        let cap = cap.unwrap_or(&StrokeCap::Butt);
        let Some(pattern) = normalized_dash_pattern(dash) else {
            self.add_capped_line_segment(p0, p1, width, cap, transform, color, label);
            return;
        };
        let (mut dash_index, mut dash_offset) = dash_start(&pattern, dash_offset);
        self.add_dashed_line_quads(
            p0,
            p1,
            width,
            &pattern,
            &mut dash_index,
            &mut dash_offset,
            cap,
            transform,
            color,
            "line.dash",
        );
    }

    fn add_dashed_line_quads(
        &mut self,
        p0: GpuPoint,
        p1: GpuPoint,
        width: f32,
        dash: &[f32],
        dash_index: &mut usize,
        dash_offset: &mut f32,
        cap: &StrokeCap,
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

        let ux = dx / len;
        let uy = dy / len;
        let mut distance = 0.0f32;
        while distance < len {
            let remaining_dash = dash[*dash_index] - *dash_offset;
            let run = remaining_dash.min(len - distance);
            if (*dash_index).is_multiple_of(2) && run > f32::EPSILON {
                let start = GpuPoint {
                    x: p0.x + ux * distance,
                    y: p0.y + uy * distance,
                };
                let end = GpuPoint {
                    x: p0.x + ux * (distance + run),
                    y: p0.y + uy * (distance + run),
                };
                self.add_capped_line_segment(start, end, width, cap, transform, color, label);
            }
            distance += run;
            if run < remaining_dash {
                *dash_offset += run;
            } else {
                *dash_index = (*dash_index + 1) % dash.len();
                *dash_offset = 0.0;
            }
        }
    }

    fn add_capped_line_segment(
        &mut self,
        p0: GpuPoint,
        p1: GpuPoint,
        width: f32,
        cap: &StrokeCap,
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
        let ux = dx / len;
        let uy = dy / len;
        let half_width = width / 2.0;
        match cap {
            StrokeCap::Butt => self.add_line_quad(p0, p1, width, transform, color, label),
            StrokeCap::Square => {
                let start = GpuPoint {
                    x: p0.x - ux * half_width,
                    y: p0.y - uy * half_width,
                };
                let end = GpuPoint {
                    x: p1.x + ux * half_width,
                    y: p1.y + uy * half_width,
                };
                self.add_line_quad(start, end, width, transform, color, label);
            }
            StrokeCap::Round => {
                self.add_line_quad(p0, p1, width, transform, color, label);
                self.add_round_line_cap(p0, -ux, -uy, half_width, transform, color, label);
                self.add_round_line_cap(p1, ux, uy, half_width, transform, color, label);
            }
        }
    }

    fn add_round_line_cap(
        &mut self,
        center: GpuPoint,
        out_x: f32,
        out_y: f32,
        radius: f32,
        transform: Transform2D,
        color: GpuColor,
        label: &'static str,
    ) {
        if radius <= f32::EPSILON || color.a == 0.0 {
            return;
        }
        let segments = (self.options.ellipse_segments / 4).max(6);
        let perp_x = -out_y;
        let perp_y = out_x;
        let mut vertices = Vec::with_capacity(segments + 2);
        vertices.push(vertex(apply_transform(center, transform), color));
        for i in 0..=segments {
            let theta =
                -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / segments as f32;
            let point = GpuPoint {
                x: center.x + out_x * theta.cos() * radius + perp_x * theta.sin() * radius,
                y: center.y + out_y * theta.cos() * radius + perp_y * theta.sin() * radius,
            };
            vertices.push(vertex(apply_transform(point, transform), color));
        }
        let mut indices = Vec::with_capacity(segments * 3);
        for i in 1..=segments {
            indices.extend_from_slice(&[0, i as u32, i as u32 + 1]);
        }
        self.add_mesh(vertices, indices, None, label);
    }

    fn add_open_contour_caps(
        &mut self,
        contour: &Contour,
        stroke_width: f32,
        cap: &StrokeCap,
        transform: Transform2D,
        color: GpuColor,
    ) {
        if contour.points.len() < 2 || matches!(cap, StrokeCap::Butt) {
            return;
        }

        let start = contour.points[0];
        let after_start = contour.points[1];
        self.add_line_endpoint_cap(
            start,
            after_start,
            stroke_width,
            true,
            cap,
            transform,
            color,
            "path.stroke",
        );

        let end = *contour.points.last().unwrap();
        let before_end = contour.points[contour.points.len() - 2];
        self.add_line_endpoint_cap(
            before_end,
            end,
            stroke_width,
            false,
            cap,
            transform,
            color,
            "path.stroke",
        );
    }

    fn add_line_endpoint_cap(
        &mut self,
        p0: GpuPoint,
        p1: GpuPoint,
        stroke_width: f32,
        at_start: bool,
        cap: &StrokeCap,
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
        let ux = dx / len;
        let uy = dy / len;
        let half_width = stroke_width / 2.0;
        let (center, out_x, out_y) = if at_start {
            (p0, -ux, -uy)
        } else {
            (p1, ux, uy)
        };

        match cap {
            StrokeCap::Butt => {}
            StrokeCap::Square => {
                let outside = GpuPoint {
                    x: center.x + out_x * half_width,
                    y: center.y + out_y * half_width,
                };
                self.add_line_quad(center, outside, stroke_width, transform, color, label);
            }
            StrokeCap::Round => {
                self.add_round_line_cap(center, out_x, out_y, half_width, transform, color, label)
            }
        }
    }

    fn add_dashed_path_contour(
        &mut self,
        contour: &Contour,
        stroke_width: f32,
        dash: &[f32],
        dash_offset: f32,
        cap: &StrokeCap,
        join: &StrokeJoin,
        miter_limit: f32,
        transform: Transform2D,
        color: GpuColor,
    ) {
        if contour.points.len() < 2 || stroke_width <= f32::EPSILON || color.a == 0.0 {
            return;
        }

        let (mut dash_index, mut dash_offset) = dash_start(dash, dash_offset);
        let segment_count = contour.points.len().saturating_sub(1);
        for segment_index in 0..segment_count {
            let start = contour.points[segment_index];
            let end = contour.points[segment_index + 1];
            let is_open_start = !contour.closed && segment_index == 0;
            let is_open_end = !contour.closed && segment_index + 1 == segment_count;
            self.add_dashed_path_segment(
                start,
                end,
                stroke_width,
                dash,
                &mut dash_index,
                &mut dash_offset,
                cap,
                is_open_start,
                is_open_end,
                transform,
                color,
            );

            if segment_index + 1 < segment_count
                && dash_cursor_inside_visible_run(dash_index, dash_offset)
            {
                self.add_line_join(
                    start,
                    end,
                    contour.points[segment_index + 2],
                    stroke_width,
                    join,
                    miter_limit,
                    transform,
                    color,
                    "path.stroke.dash.join",
                );
            }
        }
    }

    fn add_dashed_path_segment(
        &mut self,
        p0: GpuPoint,
        p1: GpuPoint,
        width: f32,
        dash: &[f32],
        dash_index: &mut usize,
        dash_offset: &mut f32,
        cap: &StrokeCap,
        is_open_start: bool,
        is_open_end: bool,
        transform: Transform2D,
        color: GpuColor,
    ) {
        let dx = p1.x - p0.x;
        let dy = p1.y - p0.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= f32::EPSILON || color.a == 0.0 {
            return;
        }

        let ux = dx / len;
        let uy = dy / len;
        let mut distance = 0.0f32;
        while distance < len {
            let remaining_dash = dash[*dash_index] - *dash_offset;
            let run = remaining_dash.min(len - distance);
            if (*dash_index).is_multiple_of(2) && run > f32::EPSILON {
                let start = GpuPoint {
                    x: p0.x + ux * distance,
                    y: p0.y + uy * distance,
                };
                let end = GpuPoint {
                    x: p0.x + ux * (distance + run),
                    y: p0.y + uy * (distance + run),
                };
                let starts_at_segment_start = distance <= f32::EPSILON;
                let ends_at_segment_end = distance + run >= len - f32::EPSILON;
                let starts_at_dash_boundary = *dash_offset <= f32::EPSILON;
                let ends_at_dash_boundary = run >= remaining_dash - f32::EPSILON;
                let start_cap =
                    starts_at_dash_boundary || (is_open_start && starts_at_segment_start);
                let end_cap = ends_at_dash_boundary || (is_open_end && ends_at_segment_end);
                self.add_partially_capped_line_segment(
                    start,
                    end,
                    width,
                    cap,
                    start_cap,
                    end_cap,
                    transform,
                    color,
                    "path.stroke.dash",
                );
            }
            distance += run;
            if run < remaining_dash {
                *dash_offset += run;
            } else {
                *dash_index = (*dash_index + 1) % dash.len();
                *dash_offset = 0.0;
            }
        }
    }

    fn add_partially_capped_line_segment(
        &mut self,
        p0: GpuPoint,
        p1: GpuPoint,
        width: f32,
        cap: &StrokeCap,
        start_cap: bool,
        end_cap: bool,
        transform: Transform2D,
        color: GpuColor,
        label: &'static str,
    ) {
        self.add_line_quad(p0, p1, width, transform, color, label);
        if start_cap {
            self.add_line_endpoint_cap(p0, p1, width, true, cap, transform, color, label);
        }
        if end_cap {
            self.add_line_endpoint_cap(p0, p1, width, false, cap, transform, color, label);
        }
    }

    fn add_contour_joins(
        &mut self,
        contour: &Contour,
        stroke_width: f32,
        join: &StrokeJoin,
        miter_limit: f32,
        transform: Transform2D,
        color: GpuColor,
    ) {
        if stroke_width <= f32::EPSILON || color.a == 0.0 {
            return;
        }

        let logical_len = if contour.closed
            && contour.points.len() > 1
            && same_point(contour.points[0], *contour.points.last().unwrap())
        {
            contour.points.len() - 1
        } else {
            contour.points.len()
        };
        if logical_len < 3 {
            return;
        }

        if contour.closed {
            for index in 0..logical_len {
                let previous = contour.points[(index + logical_len - 1) % logical_len];
                let current = contour.points[index];
                let next = contour.points[(index + 1) % logical_len];
                self.add_line_join(
                    previous,
                    current,
                    next,
                    stroke_width,
                    join,
                    miter_limit,
                    transform,
                    color,
                    "path.stroke.join",
                );
            }
        } else {
            for index in 1..logical_len - 1 {
                self.add_line_join(
                    contour.points[index - 1],
                    contour.points[index],
                    contour.points[index + 1],
                    stroke_width,
                    join,
                    miter_limit,
                    transform,
                    color,
                    "path.stroke.join",
                );
            }
        }
    }

    fn add_line_join(
        &mut self,
        previous: GpuPoint,
        current: GpuPoint,
        next: GpuPoint,
        stroke_width: f32,
        join: &StrokeJoin,
        miter_limit: f32,
        transform: Transform2D,
        color: GpuColor,
        label: &'static str,
    ) {
        let Some(in_dir) = normalized_vector(previous, current) else {
            return;
        };
        let Some(out_dir) = normalized_vector(current, next) else {
            return;
        };
        let turn = cross(in_dir, out_dir);
        if turn.abs() <= f32::EPSILON {
            return;
        }

        let half_width = stroke_width / 2.0;
        let side = if turn > 0.0 { 1.0 } else { -1.0 };
        let in_normal = scale_point(left_normal(in_dir), side);
        let out_normal = scale_point(left_normal(out_dir), side);
        let outer0 = add_points(current, scale_point(in_normal, half_width));
        let outer1 = add_points(current, scale_point(out_normal, half_width));

        match join {
            StrokeJoin::Bevel => {
                self.add_join_triangle(current, outer0, outer1, transform, color, label);
            }
            StrokeJoin::Round => self.add_round_line_join(
                current, in_normal, out_normal, turn, half_width, transform, color, label,
            ),
            StrokeJoin::Miter => {
                let miter = line_intersection(outer0, in_dir, outer1, out_dir);
                if let Some(miter) = miter
                    .filter(|point| distance(*point, current) <= half_width * miter_limit.max(1.0))
                {
                    self.add_join_triangle(outer0, miter, outer1, transform, color, label);
                } else {
                    self.add_join_triangle(current, outer0, outer1, transform, color, label);
                }
            }
        }
    }

    fn add_join_triangle(
        &mut self,
        p0: GpuPoint,
        p1: GpuPoint,
        p2: GpuPoint,
        transform: Transform2D,
        color: GpuColor,
        label: &'static str,
    ) {
        self.add_mesh(
            vec![
                vertex(apply_transform(p0, transform), color),
                vertex(apply_transform(p1, transform), color),
                vertex(apply_transform(p2, transform), color),
            ],
            vec![0, 1, 2],
            None,
            label,
        );
    }

    fn add_round_line_join(
        &mut self,
        center: GpuPoint,
        in_normal: GpuPoint,
        out_normal: GpuPoint,
        turn: f32,
        radius: f32,
        transform: Transform2D,
        color: GpuColor,
        label: &'static str,
    ) {
        if radius <= f32::EPSILON || color.a == 0.0 {
            return;
        }

        let start_angle = in_normal.y.atan2(in_normal.x);
        let end_angle = out_normal.y.atan2(out_normal.x);
        let sweep = if turn > 0.0 {
            positive_angle_delta(start_angle, end_angle)
        } else {
            -positive_angle_delta(end_angle, start_angle)
        };
        let segments = ((sweep.abs() / (std::f32::consts::PI / 8.0)).ceil() as usize).max(2);
        let mut vertices = Vec::with_capacity(segments + 2);
        vertices.push(vertex(apply_transform(center, transform), color));
        for index in 0..=segments {
            let theta = start_angle + sweep * index as f32 / segments as f32;
            vertices.push(vertex(
                apply_transform(
                    GpuPoint {
                        x: center.x + theta.cos() * radius,
                        y: center.y + theta.sin() * radius,
                    },
                    transform,
                ),
                color,
            ));
        }
        let mut indices = Vec::with_capacity(segments * 3);
        for index in 1..=segments {
            indices.extend_from_slice(&[0, index as u32, index as u32 + 1]);
        }
        self.add_mesh(vertices, indices, None, label);
    }

    fn add_dashed_line_segment(
        &mut self,
        p0: GpuPoint,
        p1: GpuPoint,
        width: f32,
        dash: &[f32],
        dash_index: &mut usize,
        dash_offset: &mut f32,
        cap: &StrokeCap,
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

        let ux = dx / len;
        let uy = dy / len;
        let mut distance = 0.0f32;
        while distance < len {
            let remaining_dash = dash[*dash_index] - *dash_offset;
            let run = remaining_dash.min(len - distance);
            if (*dash_index).is_multiple_of(2) && run > f32::EPSILON {
                let start = GpuPoint {
                    x: p0.x + ux * distance,
                    y: p0.y + uy * distance,
                };
                let end = GpuPoint {
                    x: p0.x + ux * (distance + run),
                    y: p0.y + uy * (distance + run),
                };
                self.add_capped_line_segment(start, end, width, cap, transform, color, label);
            }
            distance += run;
            if run < remaining_dash {
                *dash_offset += run;
            } else {
                *dash_index = (*dash_index + 1) % dash.len();
                *dash_offset = 0.0;
            }
        }
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

fn normalized_dash_pattern(dash: Option<&[f64]>) -> Option<Vec<f32>> {
    let dash = dash?;
    let mut pattern = dash
        .iter()
        .filter_map(|value| {
            let value = *value as f32;
            (value.is_finite() && value > f32::EPSILON).then_some(value)
        })
        .collect::<Vec<_>>();
    if pattern.is_empty() {
        return None;
    }
    if pattern.len() % 2 == 1 {
        let repeated = pattern.clone();
        pattern.extend(repeated);
    }
    Some(pattern)
}

fn dash_start(dash: &[f32], dash_offset: f32) -> (usize, f32) {
    let cycle = dash.iter().sum::<f32>();
    let mut index = 0usize;
    let mut offset = dash_offset.rem_euclid(cycle);
    while offset >= dash[index] {
        offset -= dash[index];
        index = (index + 1) % dash.len();
    }
    (index, offset)
}

fn dash_cursor_inside_visible_run(dash_index: usize, dash_offset: f32) -> bool {
    dash_index.is_multiple_of(2) && dash_offset > f32::EPSILON
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

fn normalized_vector(from: GpuPoint, to: GpuPoint) -> Option<GpuPoint> {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    (len > f32::EPSILON).then_some(GpuPoint {
        x: dx / len,
        y: dy / len,
    })
}

fn left_normal(vector: GpuPoint) -> GpuPoint {
    GpuPoint {
        x: -vector.y,
        y: vector.x,
    }
}

fn scale_point(point: GpuPoint, scale: f32) -> GpuPoint {
    GpuPoint {
        x: point.x * scale,
        y: point.y * scale,
    }
}

fn add_points(a: GpuPoint, b: GpuPoint) -> GpuPoint {
    GpuPoint {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

fn cross(a: GpuPoint, b: GpuPoint) -> f32 {
    a.x * b.y - a.y * b.x
}

fn distance(a: GpuPoint, b: GpuPoint) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

fn line_intersection(
    origin0: GpuPoint,
    direction0: GpuPoint,
    origin1: GpuPoint,
    direction1: GpuPoint,
) -> Option<GpuPoint> {
    let denom = cross(direction0, direction1);
    if denom.abs() <= f32::EPSILON {
        return None;
    }
    let delta = GpuPoint {
        x: origin1.x - origin0.x,
        y: origin1.y - origin0.y,
    };
    let t = cross(delta, direction1) / denom;
    Some(GpuPoint {
        x: origin0.x + direction0.x * t,
        y: origin0.y + direction0.y * t,
    })
}

fn positive_angle_delta(from: f32, to: f32) -> f32 {
    (to - from).rem_euclid(std::f32::consts::TAU)
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

fn ellipse_point(ellipse: &PaintEllipse, t: f32) -> GpuPoint {
    GpuPoint {
        x: ellipse.cx as f32 + ellipse.rx as f32 * t.cos(),
        y: ellipse.cy as f32 + ellipse.ry as f32 * t.sin(),
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
        GlyphPosition, GradientKind, GradientStop, PaintBase, PaintEllipse, PaintGradient,
        PaintGroup, PaintImage, PaintInstruction, PaintRect, PixelContainer,
    };

    fn mesh_count(plan: &GpuPaintPlan, label: &str) -> usize {
        plan.meshes
            .iter()
            .filter(|mesh| mesh.label == label)
            .count()
    }

    fn mesh_has_vertex(mesh: &GpuMesh, x: f32, y: f32) -> bool {
        mesh.vertices.iter().any(|vertex| {
            (vertex.position.x - x).abs() <= f32::EPSILON
                && (vertex.position.y - y).abs() <= f32::EPSILON
        })
    }

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
    fn lowers_dashed_rect_stroke_around_perimeter() {
        let mut scene = PaintScene::new(24.0, 16.0);
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 2.0,
            y: 2.0,
            width: 8.0,
            height: 4.0,
            fill: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(2.0),
            corner_radius: None,
            stroke_dash: Some(vec![4.0, 4.0]),
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);

        assert_eq!(plan.meshes.len(), 3);
        assert!(plan
            .meshes
            .iter()
            .all(|mesh| mesh.label == "rect.stroke.dash"));
        assert_eq!(plan.meshes[0].vertices[0].position.x, 2.0);
        assert_eq!(plan.meshes[0].vertices[1].position.x, 6.0);
        assert_eq!(plan.meshes[1].vertices[0].position.y, 2.0);
        assert_eq!(plan.meshes[1].vertices[1].position.y, 6.0);
        assert_eq!(plan.meshes[2].vertices[0].position.x, 6.0);
        assert_eq!(plan.meshes[2].vertices[1].position.x, 2.0);
        assert!(!plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.feature == "stroke_dash"));
    }

    #[test]
    fn lowers_dashed_ellipse_stroke_to_segments() {
        let mut scene = PaintScene::new(20.0, 20.0);
        scene
            .instructions
            .push(PaintInstruction::Ellipse(PaintEllipse {
                base: PaintBase::default(),
                cx: 10.0,
                cy: 10.0,
                rx: 6.0,
                ry: 4.0,
                fill: None,
                stroke: Some("#000000".to_string()),
                stroke_width: Some(2.0),
                stroke_dash: Some(vec![4.0, 4.0]),
                stroke_dash_offset: None,
            }));

        let plan = plan_scene_with_options(
            &scene,
            GpuPlanOptions {
                ellipse_segments: 8,
                curve_segments: 4,
            },
        );

        assert!(plan.meshes.len() > 1);
        assert!(plan
            .meshes
            .iter()
            .all(|mesh| mesh.label == "ellipse.stroke.dash"));
        assert!(!plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.feature == "stroke_dash"));
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
    fn lowers_square_line_caps_by_extending_segment() {
        let mut scene = PaintScene::new(20.0, 12.0);
        scene.instructions.push(PaintInstruction::Line(PaintLine {
            base: PaintBase::default(),
            x1: 4.0,
            y1: 6.0,
            x2: 12.0,
            y2: 6.0,
            stroke: "#000000".to_string(),
            stroke_width: Some(4.0),
            stroke_cap: Some(StrokeCap::Square),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);

        assert_eq!(plan.meshes.len(), 1);
        assert_eq!(plan.meshes[0].vertices[0].position.x, 2.0);
        assert_eq!(plan.meshes[0].vertices[1].position.x, 14.0);
    }

    #[test]
    fn lowers_round_line_caps_to_endpoint_fans() {
        let mut scene = PaintScene::new(20.0, 12.0);
        scene.instructions.push(PaintInstruction::Line(PaintLine {
            base: PaintBase::default(),
            x1: 4.0,
            y1: 6.0,
            x2: 12.0,
            y2: 6.0,
            stroke: "#000000".to_string(),
            stroke_width: Some(4.0),
            stroke_cap: Some(StrokeCap::Round),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan_scene_with_options(
            &scene,
            GpuPlanOptions {
                ellipse_segments: 8,
                curve_segments: 4,
            },
        );

        assert_eq!(plan.meshes.len(), 3);
        assert_eq!(plan.meshes[0].vertices.len(), 4);
        assert!(plan.meshes[1].vertices.len() > 4);
        assert!(plan.meshes[2].vertices.len() > 4);
        assert!(plan.meshes.iter().all(|mesh| mesh.label == "line"));
    }

    #[test]
    fn lowers_dashed_line_to_segment_meshes() {
        let mut scene = PaintScene::new(24.0, 8.0);
        scene.instructions.push(PaintInstruction::Line(PaintLine {
            base: PaintBase::default(),
            x1: 0.0,
            y1: 4.0,
            x2: 20.0,
            y2: 4.0,
            stroke: "#000000".to_string(),
            stroke_width: Some(2.0),
            stroke_cap: None,
            stroke_dash: Some(vec![4.0, 4.0]),
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);

        assert_eq!(plan.meshes.len(), 3);
        assert!(plan
            .meshes
            .iter()
            .all(|mesh| mesh.label == "line.dash" && mesh.vertices.len() == 4));
        assert_eq!(plan.meshes[0].vertices[0].position.x, 0.0);
        assert_eq!(plan.meshes[0].vertices[1].position.x, 4.0);
        assert_eq!(plan.meshes[1].vertices[0].position.x, 8.0);
        assert_eq!(plan.meshes[1].vertices[1].position.x, 12.0);
        assert!(!plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.feature == "stroke_dash"));
    }

    #[test]
    fn lowers_dashed_line_with_offset() {
        let mut scene = PaintScene::new(24.0, 8.0);
        scene.instructions.push(PaintInstruction::Line(PaintLine {
            base: PaintBase::default(),
            x1: 0.0,
            y1: 4.0,
            x2: 20.0,
            y2: 4.0,
            stroke: "#000000".to_string(),
            stroke_width: Some(2.0),
            stroke_cap: None,
            stroke_dash: Some(vec![4.0, 4.0]),
            stroke_dash_offset: Some(2.0),
        }));

        let plan = plan_scene(&scene);

        assert_eq!(plan.meshes.len(), 3);
        assert_eq!(plan.meshes[0].vertices[0].position.x, 0.0);
        assert_eq!(plan.meshes[0].vertices[1].position.x, 2.0);
        assert_eq!(plan.meshes[1].vertices[0].position.x, 6.0);
        assert_eq!(plan.meshes[1].vertices[1].position.x, 10.0);
        assert_eq!(plan.meshes[2].vertices[0].position.x, 14.0);
        assert_eq!(plan.meshes[2].vertices[1].position.x, 18.0);
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
        assert_eq!(mesh_count(&plan, "path.stroke"), 4);
        assert_eq!(mesh_count(&plan, "path.stroke.join"), 3);
    }

    #[test]
    fn lowers_bevel_path_joins_on_open_contours() {
        let mut scene = PaintScene::new(20.0, 20.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 2.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 2.0 },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(4.0),
            stroke_cap: None,
            stroke_join: Some(StrokeJoin::Bevel),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);
        let join = plan
            .meshes
            .iter()
            .find(|mesh| mesh.label == "path.stroke.join")
            .unwrap();

        assert_eq!(mesh_count(&plan, "path.stroke"), 2);
        assert_eq!(mesh_count(&plan, "path.stroke.join"), 1);
        assert_eq!(join.vertices.len(), 3);
        assert!(mesh_has_vertex(join, 10.0, 10.0));
    }

    #[test]
    fn lowers_round_path_joins_to_arc_fans() {
        let mut scene = PaintScene::new(20.0, 20.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 2.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 2.0 },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(4.0),
            stroke_cap: None,
            stroke_join: Some(StrokeJoin::Round),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan_scene_with_options(
            &scene,
            GpuPlanOptions {
                ellipse_segments: 8,
                curve_segments: 4,
            },
        );
        let join = plan
            .meshes
            .iter()
            .find(|mesh| mesh.label == "path.stroke.join")
            .unwrap();

        assert_eq!(mesh_count(&plan, "path.stroke"), 2);
        assert_eq!(mesh_count(&plan, "path.stroke.join"), 1);
        assert!(join.vertices.len() > 3);
        assert!(mesh_has_vertex(join, 10.0, 10.0));
    }

    #[test]
    fn lowers_miter_path_joins_to_outer_corners() {
        let mut scene = PaintScene::new(20.0, 20.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 2.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 2.0 },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(2.0),
            stroke_cap: None,
            stroke_join: Some(StrokeJoin::Miter),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);
        let join = plan
            .meshes
            .iter()
            .find(|mesh| mesh.label == "path.stroke.join")
            .unwrap();

        assert_eq!(mesh_count(&plan, "path.stroke"), 2);
        assert_eq!(mesh_count(&plan, "path.stroke.join"), 1);
        assert_eq!(join.vertices.len(), 3);
        assert!(mesh_has_vertex(join, 9.0, 9.0));
    }

    #[test]
    fn lowers_closed_path_joins_at_each_corner() {
        let mut scene = PaintScene::new(20.0, 20.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 2.0, y: 2.0 },
                PathCommand::LineTo { x: 10.0, y: 2.0 },
                PathCommand::LineTo { x: 6.0, y: 10.0 },
                PathCommand::Close,
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(2.0),
            stroke_cap: Some(StrokeCap::Round),
            stroke_join: Some(StrokeJoin::Bevel),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);

        assert_eq!(mesh_count(&plan, "path.stroke"), 3);
        assert_eq!(mesh_count(&plan, "path.stroke.join"), 3);
    }

    #[test]
    fn lowers_round_dashed_path_joins_when_dash_run_crosses_corner() {
        let mut scene = PaintScene::new(20.0, 20.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 0.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 0.0 },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(4.0),
            stroke_cap: Some(StrokeCap::Round),
            stroke_join: Some(StrokeJoin::Round),
            stroke_dash: Some(vec![24.0, 8.0]),
            stroke_dash_offset: None,
        }));

        let plan = plan_scene_with_options(
            &scene,
            GpuPlanOptions {
                ellipse_segments: 8,
                curve_segments: 4,
            },
        );
        let join = plan
            .meshes
            .iter()
            .find(|mesh| mesh.label == "path.stroke.dash.join")
            .unwrap();

        assert_eq!(mesh_count(&plan, "path.stroke.dash"), 4);
        assert_eq!(mesh_count(&plan, "path.stroke.dash.join"), 1);
        assert!(join.vertices.len() > 3);
        assert!(mesh_has_vertex(join, 10.0, 10.0));
    }

    #[test]
    fn lowers_miter_dashed_path_joins_when_dash_run_crosses_corner() {
        let mut scene = PaintScene::new(20.0, 20.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 2.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 2.0 },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(2.0),
            stroke_cap: Some(StrokeCap::Butt),
            stroke_join: Some(StrokeJoin::Miter),
            stroke_dash: Some(vec![24.0, 8.0]),
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);
        let join = plan
            .meshes
            .iter()
            .find(|mesh| mesh.label == "path.stroke.dash.join")
            .unwrap();

        assert_eq!(mesh_count(&plan, "path.stroke.dash"), 2);
        assert_eq!(mesh_count(&plan, "path.stroke.dash.join"), 1);
        assert_eq!(join.vertices.len(), 3);
        assert!(mesh_has_vertex(join, 9.0, 9.0));
    }

    #[test]
    fn skips_dashed_path_join_when_dash_boundary_lands_on_corner() {
        let mut scene = PaintScene::new(20.0, 20.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 0.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 0.0 },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(4.0),
            stroke_cap: Some(StrokeCap::Round),
            stroke_join: Some(StrokeJoin::Round),
            stroke_dash: Some(vec![10.0, 10.0]),
            stroke_dash_offset: None,
        }));

        let plan = plan_scene_with_options(
            &scene,
            GpuPlanOptions {
                ellipse_segments: 8,
                curve_segments: 4,
            },
        );

        assert_eq!(mesh_count(&plan, "path.stroke.dash"), 3);
        assert_eq!(mesh_count(&plan, "path.stroke.dash.join"), 0);
    }

    #[test]
    fn skips_dashed_path_joins_on_straight_continuations() {
        let mut scene = PaintScene::new(24.0, 12.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 0.0, y: 6.0 },
                PathCommand::LineTo { x: 10.0, y: 6.0 },
                PathCommand::LineTo { x: 20.0, y: 6.0 },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(2.0),
            stroke_cap: Some(StrokeCap::Round),
            stroke_join: Some(StrokeJoin::Round),
            stroke_dash: Some(vec![4.0, 4.0]),
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);

        assert_eq!(mesh_count(&plan, "path.stroke.dash.join"), 0);
        assert!(plan
            .meshes
            .iter()
            .all(|mesh| mesh.label == "path.stroke.dash"));
    }

    #[test]
    fn lowers_square_path_caps_on_open_contours() {
        let mut scene = PaintScene::new(20.0, 12.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 4.0, y: 6.0 },
                PathCommand::LineTo { x: 12.0, y: 6.0 },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(4.0),
            stroke_cap: Some(StrokeCap::Square),
            stroke_join: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);
        let min_x = plan
            .meshes
            .iter()
            .flat_map(|mesh| mesh.vertices.iter())
            .map(|vertex| vertex.position.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = plan
            .meshes
            .iter()
            .flat_map(|mesh| mesh.vertices.iter())
            .map(|vertex| vertex.position.x)
            .fold(f32::NEG_INFINITY, f32::max);

        assert_eq!(plan.meshes.len(), 3);
        assert_eq!(min_x, 2.0);
        assert_eq!(max_x, 14.0);
        assert!(plan.meshes.iter().all(|mesh| mesh.label == "path.stroke"));
    }

    #[test]
    fn lowers_round_path_caps_on_open_contours() {
        let mut scene = PaintScene::new(20.0, 12.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 4.0, y: 6.0 },
                PathCommand::LineTo { x: 12.0, y: 6.0 },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(4.0),
            stroke_cap: Some(StrokeCap::Round),
            stroke_join: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan_scene_with_options(
            &scene,
            GpuPlanOptions {
                ellipse_segments: 8,
                curve_segments: 4,
            },
        );

        assert_eq!(plan.meshes.len(), 3);
        assert_eq!(plan.meshes[0].vertices.len(), 4);
        assert!(plan.meshes[1].vertices.len() > 4);
        assert!(plan.meshes[2].vertices.len() > 4);
        assert!(plan.meshes.iter().all(|mesh| mesh.label == "path.stroke"));
    }

    #[test]
    fn lowers_dashed_path_caps_on_dash_segments() {
        let mut scene = PaintScene::new(20.0, 12.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 0.0, y: 6.0 },
                PathCommand::LineTo { x: 10.0, y: 6.0 },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(4.0),
            stroke_cap: Some(StrokeCap::Round),
            stroke_join: None,
            stroke_dash: Some(vec![4.0, 4.0]),
            stroke_dash_offset: None,
        }));

        let plan = plan_scene_with_options(
            &scene,
            GpuPlanOptions {
                ellipse_segments: 8,
                curve_segments: 4,
            },
        );

        assert_eq!(plan.meshes.len(), 6);
        assert!(plan
            .meshes
            .iter()
            .all(|mesh| mesh.label == "path.stroke.dash"));
        assert_eq!(
            plan.meshes
                .iter()
                .filter(|mesh| mesh.vertices.len() == 4)
                .count(),
            2
        );
        assert_eq!(
            plan.meshes
                .iter()
                .filter(|mesh| mesh.vertices.len() > 4)
                .count(),
            4
        );
    }

    #[test]
    fn lowers_dashed_path_stroke_across_segments() {
        let mut scene = PaintScene::new(24.0, 12.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 0.0, y: 6.0 },
                PathCommand::LineTo { x: 10.0, y: 6.0 },
                PathCommand::LineTo { x: 20.0, y: 6.0 },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(2.0),
            stroke_cap: None,
            stroke_join: None,
            stroke_dash: Some(vec![4.0, 4.0]),
            stroke_dash_offset: None,
        }));

        let plan = plan_scene(&scene);

        assert_eq!(plan.meshes.len(), 4);
        assert!(plan
            .meshes
            .iter()
            .all(|mesh| mesh.label == "path.stroke.dash"));
        assert_eq!(plan.meshes[0].vertices[0].position.x, 0.0);
        assert_eq!(plan.meshes[0].vertices[1].position.x, 4.0);
        assert_eq!(plan.meshes[1].vertices[0].position.x, 8.0);
        assert_eq!(plan.meshes[1].vertices[1].position.x, 10.0);
        assert_eq!(plan.meshes[2].vertices[0].position.x, 10.0);
        assert_eq!(plan.meshes[2].vertices[1].position.x, 12.0);
        assert_eq!(plan.meshes[3].vertices[0].position.x, 16.0);
        assert_eq!(plan.meshes[3].vertices[1].position.x, 20.0);
        assert!(!plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.feature == "stroke_dash"));
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
    fn tier1_textured_profile_accepts_images_and_gradient_textures() {
        let mut pixels = PixelContainer::new(1, 1);
        pixels.set_pixel(0, 0, 12, 34, 56, 255);
        let mut scene = PaintScene::new(24.0, 12.0);
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase {
                    id: Some("linear".to_string()),
                    metadata: None,
                },
                kind: GradientKind::Linear {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 12.0,
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
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase {
                    id: Some("radial".to_string()),
                    metadata: None,
                },
                kind: GradientKind::Radial {
                    cx: 18.0,
                    cy: 6.0,
                    r: 6.0,
                },
                stops: vec![
                    GradientStop {
                        offset: 0.0,
                        color: "#ffffff".to_string(),
                    },
                    GradientStop {
                        offset: 1.0,
                        color: "#000000".to_string(),
                    },
                ],
            }));
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 12.0,
            fill: Some("url(#linear)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 8.0,
            y: 0.0,
            width: 8.0,
            height: 12.0,
            fill: Some("url(#radial)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
        scene.instructions.push(PaintInstruction::Image(PaintImage {
            base: PaintBase::default(),
            x: 16.0,
            y: 0.0,
            width: 8.0,
            height: 12.0,
            src: ImageSrc::Pixels(pixels),
            opacity: None,
        }));
        let profile = GpuBackendProfile::tier1_textured(
            "paint-vm-test-gpu",
            GpuApiFamily::Wgpu,
            GpuRenderPath::GraphicsPipeline,
            "test-shader",
            GpuReadbackStrategy::TextureCopyToBuffer,
        );

        let plan = plan_scene(&scene);
        let unsupported = unsupported_plan_features(profile, &plan);

        assert_eq!(
            plan.images
                .iter()
                .map(|image| image.kind)
                .collect::<Vec<_>>(),
            vec![
                GpuTextureKind::LinearGradient,
                GpuTextureKind::RadialGradient,
                GpuTextureKind::Image
            ]
        );
        assert!(unsupported.is_empty());
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
