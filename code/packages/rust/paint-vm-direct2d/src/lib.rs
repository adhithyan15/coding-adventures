//! # paint-vm-direct2d
//!
//! Direct2D GPU renderer for the paint-instructions scene model (P2D06).
//!
//! This crate takes a [`PaintScene`] (backend-neutral 2D paint instructions)
//! and renders it to a [`PixelContainer`] using Microsoft's Direct2D API —
//! a GPU-accelerated 2D rendering API available on Windows Vista and later.
//!
//! Direct2D is the modern renderer in the paint-* stack on Windows. It uses
//! the GPU for hardware-accelerated rendering with antialiasing. For a
//! simpler CPU-based fallback, see `paint-vm-gdi`.
//!
//! ## Current instruction support
//!
//! | Instruction       | Status                                              |
//! |-------------------|-----------------------------------------------------|
//! | `PaintRect`       | Fully implemented — solid-colour filled rects       |
//! | `PaintLine`       | Fully implemented — DrawLine with stroke width      |
//! | `PaintGroup`      | Fully implemented — recurses into children          |
//! | `PaintClip`       | Fully implemented — PushAxisAlignedClip / Pop       |
//! | `PaintGlyphRun`   | Planned — IDWriteFactory + DrawGlyphRun             |
//! | `PaintEllipse`    | Planned — FillEllipse                               |
//! | `PaintPath`       | Planned — ID2D1PathGeometry                         |
//! | `PaintLayer`      | Planned — PushLayer / PopLayer                      |
//! | `PaintGradient`   | Planned — CreateLinearGradientBrush                 |
//! | `PaintImage`      | Planned — ID2D1Bitmap from PixelContainer           |
//!
//! ## Direct2D pipeline (offscreen, no HWND)
//!
//! ```text
//! PaintScene
//!   │
//!   ├── 1. CoInitializeEx() — COM single-threaded apartment
//!   ├── 2. D2D1CreateFactory() → ID2D1Factory
//!   ├── 3. CoCreateInstance(CLSID_WICImagingFactory) → IWICImagingFactory
//!   ├── 4. CreateBitmap() → IWICBitmap (offscreen RGBA target)
//!   ├── 5. CreateWicBitmapRenderTarget() → ID2D1RenderTarget
//!   ├── 6. BeginDraw → Clear(background) → dispatch instructions → EndDraw
//!   ├── 7. Lock WIC bitmap → read premultiplied BGRA pixels
//!   └── 8. Convert pBGRA → RGBA → PixelContainer
//! ```
//!
//! ## Coordinate system
//!
//! `PaintScene` uses a **top-left origin** with Y increasing downward
//! (same as SVG, HTML Canvas, and CSS).
//!
//! Direct2D also uses a top-left origin with Y increasing downward by
//! default — so no coordinate conversion is needed (unlike Metal which
//! requires NDC conversion).
//!
//! ```text
//!  Scene coordinates:       Direct2D coordinates:
//!  (0,0)──────(w,0)        (0,0)──────(w,0)
//!    │              │           │              │
//!    │              │           │              │
//!    │              │           │              │
//!  (0,h)──────(w,h)        (0,h)──────(w,h)
//! ```
//!
//! ## Premultiplied alpha
//!
//! Direct2D renders to premultiplied BGRA (pBGRA). In premultiplied alpha,
//! each colour channel is pre-scaled by the alpha value:
//!
//! ```text
//! Straight:       (R, G, B, A) = (255, 0, 0, 128) — half-transparent red
//! Premultiplied:  (R, G, B, A) = (128, 0, 0, 128) — R scaled by A/255
//! ```
//!
//! When reading back pixels, we un-premultiply and swap BGRA→RGBA.

pub const VERSION: &str = "0.1.0";

use paint_instructions::{
    FillRule, ImageSrc, PaintClip, PaintEllipse, PaintGlyphRun, PaintGroup, PaintImage,
    PaintInstruction, PaintLayer, PaintLine, PaintPath, PaintRect, PaintScene, PathCommand,
    PixelContainer,
};
#[cfg(target_os = "windows")]
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Platform gate
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
compile_error!(
    "paint-vm-direct2d requires Windows. Use paint-metal on macOS or paint-vm-cairo on Linux."
);

// ---------------------------------------------------------------------------
// Windows API imports
// ---------------------------------------------------------------------------
//
// Direct2D is a COM-based API. All interactions go through COM interfaces:
//   - ID2D1Factory: creates render targets and geometry objects
//   - ID2D1RenderTarget: the drawing surface (backed by a WIC bitmap here)
//   - ID2D1SolidColorBrush: a brush that paints a single colour
//   - IWICImagingFactory: creates WIC bitmaps for offscreen rendering
//   - IWICBitmap: a CPU-accessible bitmap that Direct2D can render into

#[cfg(target_os = "windows")]
use windows::core::{GUID, Interface, PCWSTR};
#[cfg(target_os = "windows")]
use windows::Foundation::Numerics::Matrix3x2;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{BOOL, FALSE, HWND, RECT};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_ALPHA_MODE_UNKNOWN, D2D1_BEZIER_SEGMENT, D2D1_COLOR_F,
    D2D1_FIGURE_BEGIN_FILLED, D2D1_FIGURE_END_CLOSED, D2D1_FIGURE_END_OPEN,
    D2D1_FILL_MODE_ALTERNATE, D2D1_FILL_MODE_WINDING, D2D1_PIXEL_FORMAT, D2D_POINT_2F, D2D_RECT_F,
    D2D_SIZE_F, D2D_SIZE_U,
};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Factory, ID2D1RenderTarget, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
    D2D1_ARC_SEGMENT, D2D1_ARC_SIZE_LARGE, D2D1_ARC_SIZE_SMALL,
    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, D2D1_BITMAP_PROPERTIES, D2D1_ELLIPSE,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_LAYER_OPTIONS_NONE,
    D2D1_LAYER_PARAMETERS, D2D1_PRESENT_OPTIONS_NONE, D2D1_QUADRATIC_BEZIER_SEGMENT,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
    D2D1_ROUNDED_RECT, D2D1_SWEEP_DIRECTION_CLOCKWISE, D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE,
};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteFontCollection, IDWriteFontFace,
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH, DWRITE_FONT_STRETCH_CONDENSED,
    DWRITE_FONT_STRETCH_EXPANDED, DWRITE_FONT_STRETCH_EXTRA_CONDENSED,
    DWRITE_FONT_STRETCH_EXTRA_EXPANDED, DWRITE_FONT_STRETCH_NORMAL,
    DWRITE_FONT_STRETCH_SEMI_CONDENSED, DWRITE_FONT_STRETCH_SEMI_EXPANDED,
    DWRITE_FONT_STRETCH_ULTRA_CONDENSED, DWRITE_FONT_STRETCH_ULTRA_EXPANDED, DWRITE_FONT_STYLE,
    DWRITE_FONT_STYLE_ITALIC, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STYLE_OBLIQUE,
    DWRITE_FONT_WEIGHT, DWRITE_GLYPH_OFFSET, DWRITE_GLYPH_RUN, DWRITE_MEASURING_MODE_NATURAL,
};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_UNKNOWN};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, IWICBitmap, IWICImagingFactory, WICBitmapCacheOnLoad,
    WICBitmapLockRead,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    COINIT_MULTITHREADED,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

#[cfg(target_os = "windows")]
use window_core::{LogicalSize, SurfacePreference, WindowAttributes};
#[cfg(target_os = "windows")]
use window_win32::Win32Backend;

// ---------------------------------------------------------------------------
// Colour parsing
// ---------------------------------------------------------------------------

/// Parse a hex colour string to RGBA floats in the range 0.0–1.0.
///
/// Supported formats:
/// - `"#rrggbb"`   → (r, g, b, 1.0)
/// - `"#rrggbbaa"` → (r, g, b, a)
/// - `"#rgb"`      → expanded to `#rrggbb`
/// - `"transparent"` / anything else → (0.0, 0.0, 0.0, 0.0)
fn parse_css_color(s: &str) -> (f64, f64, f64, f64) {
    let s = s.trim();
    if s == "transparent" {
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

#[allow(dead_code)]
fn parse_hex_color(s: &str) -> (f64, f64, f64, f64) {
    parse_css_color(s)
}

/// Convert RGBA floats to a Direct2D [`D2D1_COLOR_F`].
///
/// Direct2D uses float4 colours in the range 0.0–1.0, same as our parsed values.
#[cfg(target_os = "windows")]
fn to_d2d_color(r: f64, g: f64, b: f64, a: f64) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: r as f32,
        g: g as f32,
        b: b as f32,
        a: a as f32,
    }
}

// ---------------------------------------------------------------------------
// Instruction dispatch — PaintInstruction → Direct2D calls
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
struct RenderContext {
    factory: ID2D1Factory,
    font_collection: IDWriteFontCollection,
    font_cache: HashMap<String, IDWriteFontFace>,
    scene_bounds: D2D_RECT_F,
}

#[cfg(target_os = "windows")]
impl RenderContext {
    unsafe fn new(factory: ID2D1Factory, width: f32, height: f32) -> Self {
        let dwrite_factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)
            .expect("Failed to create DWrite factory");
        let mut font_collection = None;
        dwrite_factory
            .GetSystemFontCollection(&mut font_collection, FALSE)
            .expect("Failed to get system font collection");
        Self {
            factory,
            font_collection: font_collection.expect("system font collection"),
            font_cache: HashMap::new(),
            scene_bounds: D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: width,
                bottom: height,
            },
        }
    }

    unsafe fn font_face_for_ref(&mut self, font_ref: &str) -> Option<IDWriteFontFace> {
        if let Some(face) = self.font_cache.get(font_ref) {
            return Some(face.clone());
        }

        let spec = parse_directwrite_font_ref(font_ref).unwrap_or_else(|| DWriteFontRef {
            family: "Segoe UI".to_string(),
            weight: 400,
            style: DWRITE_FONT_STYLE_NORMAL,
            stretch: DWRITE_FONT_STRETCH_NORMAL,
        });
        let face = self.resolve_font_face(&spec).or_else(|| {
            self.resolve_font_face(&DWriteFontRef {
                family: "Segoe UI".to_string(),
                weight: 400,
                style: DWRITE_FONT_STYLE_NORMAL,
                stretch: DWRITE_FONT_STRETCH_NORMAL,
            })
        })?;
        self.font_cache.insert(font_ref.to_string(), face.clone());
        Some(face)
    }

    unsafe fn resolve_font_face(&self, spec: &DWriteFontRef) -> Option<IDWriteFontFace> {
        let family_w = wide_null(&spec.family);
        let mut index = 0u32;
        let mut exists = BOOL(0);
        self.font_collection
            .FindFamilyName(PCWSTR(family_w.as_ptr()), &mut index, &mut exists)
            .ok()?;
        if !exists.as_bool() {
            return None;
        }
        let family = self.font_collection.GetFontFamily(index).ok()?;
        let font = family
            .GetFirstMatchingFont(
                DWRITE_FONT_WEIGHT(spec.weight as i32),
                spec.stretch,
                spec.style,
            )
            .ok()?;
        font.CreateFontFace().ok()
    }
}

#[cfg(target_os = "windows")]
struct DWriteFontRef {
    family: String,
    weight: u16,
    style: DWRITE_FONT_STYLE,
    stretch: DWRITE_FONT_STRETCH,
}

/// Render a list of [`PaintInstruction`]s into a Direct2D render target.
///
/// This is the core dispatch loop. It recursively handles Group and Clip
/// nodes, and dispatches Rect and Line to their respective D2D calls.
#[cfg(target_os = "windows")]
unsafe fn render_instructions(
    ctx: &mut RenderContext,
    rt: &ID2D1RenderTarget,
    instructions: &[PaintInstruction],
) {
    for instr in instructions {
        match instr {
            PaintInstruction::Rect(rect) => render_rect(rt, rect),
            PaintInstruction::Line(line) => render_line(rt, line),
            PaintInstruction::Group(group) => {
                // PaintGroup: render children directly into the same target.
                // Transform support (SetTransform) is deferred — for barcodes,
                // groups are purely logical containers.
                render_group(ctx, rt, group);
            }
            PaintInstruction::Clip(clip) => render_clip(ctx, rt, clip),
            PaintInstruction::GlyphRun(run) => render_glyph_run(ctx, rt, run),
            PaintInstruction::Ellipse(ellipse) => render_ellipse(rt, ellipse),
            PaintInstruction::Path(path) => render_path(ctx, rt, path),
            PaintInstruction::Layer(layer) => render_layer(ctx, rt, layer),
            PaintInstruction::Image(image) => render_image(rt, image),
            PaintInstruction::Gradient(_) => {}
        }
    }
}

/// Render a [`PaintRect`] as a filled rectangle.
///
/// Direct2D's `FillRectangle` takes a `D2D_RECT_F` (left, top, right, bottom)
/// and a brush. We create a temporary `ID2D1SolidColorBrush` for each rect.
///
/// ```text
/// (left, top) ────── (right, top)
///      │                    │
///      │   FillRectangle    │
///      │                    │
/// (left, bottom) ── (right, bottom)
/// ```
#[cfg(target_os = "windows")]
unsafe fn render_rect(rt: &ID2D1RenderTarget, rect: &PaintRect) {
    let d2d_rect = D2D_RECT_F {
        left: rect.x as f32,
        top: rect.y as f32,
        right: (rect.x + rect.width) as f32,
        bottom: (rect.y + rect.height) as f32,
    };
    let radius = rect.corner_radius.unwrap_or(0.0).max(0.0) as f32;

    if let Some(fill) = rect.fill.as_deref() {
        if let Some(brush) = solid_brush(rt, fill) {
            if radius > 0.0 {
                let rounded = D2D1_ROUNDED_RECT {
                    rect: d2d_rect,
                    radiusX: radius,
                    radiusY: radius,
                };
                rt.FillRoundedRectangle(&rounded, &brush);
            } else {
                rt.FillRectangle(&d2d_rect, &brush);
            }
        }
    }

    if let Some(stroke) = rect.stroke.as_deref() {
        if let Some(brush) = solid_brush(rt, stroke) {
            let stroke_width = rect.stroke_width.unwrap_or(1.0).max(0.0) as f32;
            if radius > 0.0 {
                let rounded = D2D1_ROUNDED_RECT {
                    rect: d2d_rect,
                    radiusX: radius,
                    radiusY: radius,
                };
                rt.DrawRoundedRectangle(&rounded, &brush, stroke_width, None);
            } else {
                rt.DrawRectangle(&d2d_rect, &brush, stroke_width, None);
            }
        }
    }
}

/// Render a [`PaintLine`] using Direct2D's `DrawLine`.
///
/// `DrawLine` takes two `D2D_POINT_2F` endpoints, a brush, and a stroke width.
/// Direct2D handles the perpendicular expansion internally (unlike paint-metal
/// which manually constructs a thin rectangle from triangle vertices).
#[cfg(target_os = "windows")]
unsafe fn render_line(rt: &ID2D1RenderTarget, line: &PaintLine) {
    if let Some(brush) = solid_brush(rt, &line.stroke) {
        let p0 = D2D_POINT_2F {
            x: line.x1 as f32,
            y: line.y1 as f32,
        };
        let p1 = D2D_POINT_2F {
            x: line.x2 as f32,
            y: line.y2 as f32,
        };
        let stroke_width = line.stroke_width.unwrap_or(1.0) as f32;

        rt.DrawLine(p0, p1, &brush, stroke_width, None);
    }
}

/// Render a [`PaintClip`] using Direct2D's axis-aligned clip.
///
/// Direct2D clip flow:
/// 1. `PushAxisAlignedClip()` — restricts drawing to the clip rectangle
/// 2. Render children
/// 3. `PopAxisAlignedClip()` — restores the previous clip
///
/// Nested clips are intersected automatically by Direct2D.
#[cfg(target_os = "windows")]
unsafe fn render_clip(ctx: &mut RenderContext, rt: &ID2D1RenderTarget, clip: &PaintClip) {
    let clip_rect = D2D_RECT_F {
        left: clip.x as f32,
        top: clip.y as f32,
        right: (clip.x + clip.width) as f32,
        bottom: (clip.y + clip.height) as f32,
    };

    rt.PushAxisAlignedClip(&clip_rect, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
    render_instructions(ctx, rt, &clip.children);
    rt.PopAxisAlignedClip();
}

#[cfg(target_os = "windows")]
unsafe fn render_group(ctx: &mut RenderContext, rt: &ID2D1RenderTarget, group: &PaintGroup) {
    with_transform(rt, group.transform.as_ref(), || {
        let opacity = group.opacity.unwrap_or(1.0).clamp(0.0, 1.0) as f32;
        if opacity < 1.0 {
            with_layer(ctx, rt, opacity, |ctx, rt| {
                render_instructions(ctx, rt, &group.children);
            });
        } else {
            render_instructions(ctx, rt, &group.children);
        }
    });
}

#[cfg(target_os = "windows")]
unsafe fn render_layer(ctx: &mut RenderContext, rt: &ID2D1RenderTarget, layer: &PaintLayer) {
    with_transform(rt, layer.transform.as_ref(), || {
        let opacity = layer.opacity.unwrap_or(1.0).clamp(0.0, 1.0) as f32;
        with_layer(ctx, rt, opacity, |ctx, rt| {
            render_instructions(ctx, rt, &layer.children);
        });
    });
}

#[cfg(target_os = "windows")]
unsafe fn render_ellipse(rt: &ID2D1RenderTarget, ellipse: &PaintEllipse) {
    let d2d_ellipse = D2D1_ELLIPSE {
        point: D2D_POINT_2F {
            x: ellipse.cx as f32,
            y: ellipse.cy as f32,
        },
        radiusX: ellipse.rx as f32,
        radiusY: ellipse.ry as f32,
    };
    if let Some(fill) = ellipse.fill.as_deref() {
        if let Some(brush) = solid_brush(rt, fill) {
            rt.FillEllipse(&d2d_ellipse, &brush);
        }
    }
    if let Some(stroke) = ellipse.stroke.as_deref() {
        if let Some(brush) = solid_brush(rt, stroke) {
            rt.DrawEllipse(
                &d2d_ellipse,
                &brush,
                ellipse.stroke_width.unwrap_or(1.0) as f32,
                None,
            );
        }
    }
}

#[cfg(target_os = "windows")]
unsafe fn render_path(ctx: &RenderContext, rt: &ID2D1RenderTarget, path: &PaintPath) {
    let fill_mode = match path.fill_rule.as_ref().unwrap_or(&FillRule::NonZero) {
        FillRule::NonZero => D2D1_FILL_MODE_WINDING,
        FillRule::EvenOdd => D2D1_FILL_MODE_ALTERNATE,
    };
    let geometry = match ctx.factory.CreatePathGeometry() {
        Ok(g) => g,
        Err(_) => return,
    };
    let sink = match geometry.Open() {
        Ok(s) => s,
        Err(_) => return,
    };
    sink.SetFillMode(fill_mode);

    let mut figure_open = false;
    for command in &path.commands {
        match *command {
            PathCommand::MoveTo { x, y } => {
                if figure_open {
                    sink.EndFigure(D2D1_FIGURE_END_OPEN);
                }
                sink.BeginFigure(point(x, y), D2D1_FIGURE_BEGIN_FILLED);
                figure_open = true;
            }
            PathCommand::LineTo { x, y } => {
                ensure_figure(&sink, &mut figure_open, x, y);
                sink.AddLine(point(x, y));
            }
            PathCommand::QuadTo { cx, cy, x, y } => {
                ensure_figure(&sink, &mut figure_open, x, y);
                let segment = D2D1_QUADRATIC_BEZIER_SEGMENT {
                    point1: point(cx, cy),
                    point2: point(x, y),
                };
                sink.AddQuadraticBezier(&segment);
            }
            PathCommand::CubicTo {
                cx1,
                cy1,
                cx2,
                cy2,
                x,
                y,
            } => {
                ensure_figure(&sink, &mut figure_open, x, y);
                let segment = D2D1_BEZIER_SEGMENT {
                    point1: point(cx1, cy1),
                    point2: point(cx2, cy2),
                    point3: point(x, y),
                };
                sink.AddBezier(&segment);
            }
            PathCommand::ArcTo {
                rx,
                ry,
                x_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => {
                ensure_figure(&sink, &mut figure_open, x, y);
                let segment = D2D1_ARC_SEGMENT {
                    point: point(x, y),
                    size: D2D_SIZE_F {
                        width: rx as f32,
                        height: ry as f32,
                    },
                    rotationAngle: x_rotation as f32,
                    sweepDirection: if sweep {
                        D2D1_SWEEP_DIRECTION_CLOCKWISE
                    } else {
                        D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE
                    },
                    arcSize: if large_arc {
                        D2D1_ARC_SIZE_LARGE
                    } else {
                        D2D1_ARC_SIZE_SMALL
                    },
                };
                sink.AddArc(&segment);
            }
            PathCommand::Close => {
                if figure_open {
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    figure_open = false;
                }
            }
        }
    }
    if figure_open {
        sink.EndFigure(D2D1_FIGURE_END_OPEN);
    }
    if sink.Close().is_err() {
        return;
    }

    if let Some(fill) = path.fill.as_deref() {
        if let Some(brush) = solid_brush(rt, fill) {
            rt.FillGeometry(&geometry, &brush, None);
        }
    }
    if let Some(stroke) = path.stroke.as_deref() {
        if let Some(brush) = solid_brush(rt, stroke) {
            rt.DrawGeometry(
                &geometry,
                &brush,
                path.stroke_width.unwrap_or(1.0) as f32,
                None,
            );
        }
    }
}

#[cfg(target_os = "windows")]
unsafe fn render_glyph_run(ctx: &mut RenderContext, rt: &ID2D1RenderTarget, run: &PaintGlyphRun) {
    if run.glyphs.is_empty() {
        return;
    }
    let Some(face) = ctx.font_face_for_ref(&run.font_ref) else {
        return;
    };
    let Some(brush) = solid_brush(rt, run.fill.as_deref().unwrap_or("#000000")) else {
        return;
    };

    for glyph in &run.glyphs {
        let glyph_index = glyph.glyph_id as u16;
        let glyph_indices = [glyph_index];
        let glyph_advances = [0.0f32];
        let glyph_offsets = [DWRITE_GLYPH_OFFSET {
            advanceOffset: 0.0,
            ascenderOffset: 0.0,
        }];
        let baseline = D2D_POINT_2F {
            x: glyph.x as f32,
            y: glyph.y as f32,
        };

        let glyph_run = DWRITE_GLYPH_RUN {
            fontFace: std::mem::ManuallyDrop::new(Some(face.clone())),
            fontEmSize: run.font_size as f32,
            glyphCount: 1,
            glyphIndices: glyph_indices.as_ptr(),
            glyphAdvances: glyph_advances.as_ptr(),
            glyphOffsets: glyph_offsets.as_ptr(),
            isSideways: FALSE,
            bidiLevel: 0,
        };
        rt.DrawGlyphRun(baseline, &glyph_run, &brush, DWRITE_MEASURING_MODE_NATURAL);
    }
}

#[cfg(target_os = "windows")]
unsafe fn render_image(rt: &ID2D1RenderTarget, image: &PaintImage) {
    let ImageSrc::Pixels(pixels) = &image.src else {
        return;
    };
    if pixels.width == 0
        || pixels.height == 0
        || pixels.data.len() < pixels.width as usize * pixels.height as usize * 4
    {
        return;
    }

    let mut pbgra = Vec::with_capacity(pixels.data.len());
    for rgba in pixels.data.chunks_exact(4) {
        let a = rgba[3] as u16;
        pbgra.push(((rgba[2] as u16 * a + 127) / 255) as u8);
        pbgra.push(((rgba[1] as u16 * a + 127) / 255) as u8);
        pbgra.push(((rgba[0] as u16 * a + 127) / 255) as u8);
        pbgra.push(rgba[3]);
    }

    let props = D2D1_BITMAP_PROPERTIES {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
    };
    let bitmap = match rt.CreateBitmap(
        D2D_SIZE_U {
            width: pixels.width,
            height: pixels.height,
        },
        Some(pbgra.as_ptr() as *const _),
        pixels.width * 4,
        &props,
    ) {
        Ok(bitmap) => bitmap,
        Err(_) => return,
    };
    let dest = D2D_RECT_F {
        left: image.x as f32,
        top: image.y as f32,
        right: (image.x + image.width) as f32,
        bottom: (image.y + image.height) as f32,
    };
    rt.DrawBitmap(
        &bitmap,
        Some(&dest),
        image.opacity.unwrap_or(1.0).clamp(0.0, 1.0) as f32,
        D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
        None,
    );
}

#[cfg(target_os = "windows")]
unsafe fn with_layer<F>(ctx: &mut RenderContext, rt: &ID2D1RenderTarget, opacity: f32, f: F)
where
    F: FnOnce(&mut RenderContext, &ID2D1RenderTarget),
{
    let layer_size = D2D_SIZE_F {
        width: (ctx.scene_bounds.right - ctx.scene_bounds.left).max(1.0),
        height: (ctx.scene_bounds.bottom - ctx.scene_bounds.top).max(1.0),
    };
    let Ok(layer) = rt.CreateLayer(Some(&layer_size)) else {
        f(ctx, rt);
        return;
    };
    let mut params = D2D1_LAYER_PARAMETERS::default();
    params.contentBounds = ctx.scene_bounds;
    params.maskAntialiasMode = D2D1_ANTIALIAS_MODE_PER_PRIMITIVE;
    params.maskTransform = identity_matrix();
    params.opacity = opacity;
    params.layerOptions = D2D1_LAYER_OPTIONS_NONE;
    rt.PushLayer(&params, &layer);
    f(ctx, rt);
    rt.PopLayer();
}

#[cfg(target_os = "windows")]
unsafe fn with_transform<F>(rt: &ID2D1RenderTarget, transform: Option<&[f64; 6]>, f: F)
where
    F: FnOnce(),
{
    let Some(transform) = transform else {
        f();
        return;
    };
    let mut previous = identity_matrix();
    rt.GetTransform(&mut previous);
    let local = matrix_from_transform(transform);
    let combined = multiply_matrix(previous, local);
    rt.SetTransform(&combined);
    f();
    rt.SetTransform(&previous);
}

#[cfg(target_os = "windows")]
unsafe fn solid_brush(
    rt: &ID2D1RenderTarget,
    color: &str,
) -> Option<windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush> {
    let (r, g, b, a) = parse_css_color(color);
    if a <= 0.0 {
        return None;
    }
    let color = to_d2d_color(r, g, b, a);
    rt.CreateSolidColorBrush(&color, None).ok()
}

#[cfg(target_os = "windows")]
fn point(x: f64, y: f64) -> D2D_POINT_2F {
    D2D_POINT_2F {
        x: x as f32,
        y: y as f32,
    }
}

#[cfg(target_os = "windows")]
unsafe fn ensure_figure(
    sink: &windows::Win32::Graphics::Direct2D::ID2D1GeometrySink,
    figure_open: &mut bool,
    x: f64,
    y: f64,
) {
    if !*figure_open {
        sink.BeginFigure(point(x, y), D2D1_FIGURE_BEGIN_FILLED);
        *figure_open = true;
    }
}

#[cfg(target_os = "windows")]
fn identity_matrix() -> Matrix3x2 {
    Matrix3x2 {
        M11: 1.0,
        M12: 0.0,
        M21: 0.0,
        M22: 1.0,
        M31: 0.0,
        M32: 0.0,
    }
}

#[cfg(target_os = "windows")]
fn matrix_from_transform(t: &[f64; 6]) -> Matrix3x2 {
    Matrix3x2 {
        M11: t[0] as f32,
        M12: t[1] as f32,
        M21: t[2] as f32,
        M22: t[3] as f32,
        M31: t[4] as f32,
        M32: t[5] as f32,
    }
}

#[cfg(target_os = "windows")]
fn multiply_matrix(a: Matrix3x2, b: Matrix3x2) -> Matrix3x2 {
    Matrix3x2 {
        M11: a.M11 * b.M11 + a.M12 * b.M21,
        M12: a.M11 * b.M12 + a.M12 * b.M22,
        M21: a.M21 * b.M11 + a.M22 * b.M21,
        M22: a.M21 * b.M12 + a.M22 * b.M22,
        M31: a.M31 * b.M11 + a.M32 * b.M21 + b.M31,
        M32: a.M31 * b.M12 + a.M32 * b.M22 + b.M32,
    }
}

#[cfg(target_os = "windows")]
fn parse_directwrite_font_ref(font_ref: &str) -> Option<DWriteFontRef> {
    let body = font_ref.strip_prefix("directwrite:")?;
    let (family_part, rest) = body.split_once('@')?;
    let mut weight = 400u16;
    let mut style = DWRITE_FONT_STYLE_NORMAL;
    let mut stretch_rank = 5u8;
    for part in rest.split(';').skip(1) {
        if let Some(value) = part.strip_prefix("w=") {
            weight = value.parse().unwrap_or(weight);
        } else if let Some(value) = part.strip_prefix("style=") {
            style = match value {
                "italic" => DWRITE_FONT_STYLE_ITALIC,
                "oblique" => DWRITE_FONT_STYLE_OBLIQUE,
                _ => DWRITE_FONT_STYLE_NORMAL,
            };
        } else if let Some(value) = part.strip_prefix("stretch=") {
            stretch_rank = value.parse().unwrap_or(stretch_rank);
        }
    }
    Some(DWriteFontRef {
        family: unescape_ref_component(family_part),
        weight,
        style,
        stretch: stretch_from_rank(stretch_rank),
    })
}

#[cfg(target_os = "windows")]
fn stretch_from_rank(rank: u8) -> DWRITE_FONT_STRETCH {
    match rank {
        1 => DWRITE_FONT_STRETCH_ULTRA_CONDENSED,
        2 => DWRITE_FONT_STRETCH_EXTRA_CONDENSED,
        3 => DWRITE_FONT_STRETCH_CONDENSED,
        4 => DWRITE_FONT_STRETCH_SEMI_CONDENSED,
        6 => DWRITE_FONT_STRETCH_SEMI_EXPANDED,
        7 => DWRITE_FONT_STRETCH_EXPANDED,
        8 => DWRITE_FONT_STRETCH_EXTRA_EXPANDED,
        9 => DWRITE_FONT_STRETCH_ULTRA_EXPANDED,
        _ => DWRITE_FONT_STRETCH_NORMAL,
    }
}

#[cfg(target_os = "windows")]
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

#[cfg(target_os = "windows")]
fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Render a [`PaintScene`] to a [`PixelContainer`] using Direct2D.
///
/// This is the main entry point for the `paint-vm-direct2d` crate.
///
/// ## Pipeline
///
/// 1. Initialize COM (CoInitializeEx)
/// 2. Create ID2D1Factory and IWICImagingFactory
/// 3. Create an offscreen WIC bitmap (premultiplied BGRA)
/// 4. Create a WIC bitmap render target
/// 5. BeginDraw → Clear → dispatch instructions → EndDraw
/// 6. Lock the WIC bitmap and read premultiplied BGRA pixels
/// 7. Un-premultiply and convert BGRA→RGBA
/// 8. Return as `PixelContainer`
///
/// ## Requires
///
/// - Windows Vista or later (Direct2D is not available on Windows XP)
/// - A GPU is used if available; falls back to WARP software rasteriser
///
/// ## Chaining with a codec
///
/// ```rust,ignore
/// let scene = barcode_2d::layout(&grid, &config);
/// let pixels = paint_vm_direct2d::render(&scene);
/// let png = paint_codec_png::encode_png(&pixels);
/// std::fs::write("barcode.png", png).unwrap();
/// ```
#[cfg(target_os = "windows")]
pub fn render(scene: &PaintScene) -> PixelContainer {
    // Validate that dimensions are finite, non-negative, and within u32 range
    // before casting. NaN, negative, or infinite values would produce nonsensical
    // results from `as u32` (saturating cast in release mode).
    if scene.width < 0.0
        || scene.height < 0.0
        || !scene.width.is_finite()
        || !scene.height.is_finite()
    {
        return PixelContainer::new(0, 0);
    }

    let width = scene.width as u32;
    let height = scene.height as u32;

    if width == 0 || height == 0 {
        return PixelContainer::new(width, height);
    }

    const MAX_DIMENSION: u32 = 16384;
    assert!(
        width <= MAX_DIMENSION && height <= MAX_DIMENSION,
        "Scene dimensions {}x{} exceed maximum {}x{}",
        width,
        height,
        MAX_DIMENSION,
        MAX_DIMENSION
    );

    unsafe { render_unsafe(scene, width, height) }
}

/// Render a [`PaintScene`] directly into an existing Win32 HWND.
#[cfg(target_os = "windows")]
pub unsafe fn render_to_hwnd(hwnd: HWND, scene: &PaintScene) -> windows::core::Result<()> {
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect)?;
    let width = (rect.right - rect.left).max(1) as u32;
    let height = (rect.bottom - rect.top).max(1) as u32;

    let d2d_factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
    let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
        r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_UNKNOWN,
            alphaMode: D2D1_ALPHA_MODE_UNKNOWN,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        usage: D2D1_RENDER_TARGET_USAGE_NONE,
        minLevel: Default::default(),
    };
    let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
        hwnd,
        pixelSize: D2D_SIZE_U { width, height },
        presentOptions: D2D1_PRESENT_OPTIONS_NONE,
    };
    let hwnd_target = d2d_factory.CreateHwndRenderTarget(&rt_props, &hwnd_props)?;
    let render_target: ID2D1RenderTarget = hwnd_target.cast()?;
    let mut ctx = RenderContext::new(d2d_factory, scene.width as f32, scene.height as f32);
    let (bg_r, bg_g, bg_b, bg_a) = parse_css_color(&scene.background);
    let bg_color = to_d2d_color(bg_r, bg_g, bg_b, bg_a);

    render_target.BeginDraw();
    render_target.Clear(Some(&bg_color));
    render_instructions(&mut ctx, &render_target, &scene.instructions);
    render_target.EndDraw(None, None)?;
    Ok(())
}

/// Open a simple Win32 window and paint the scene through the Direct2D backend.
#[cfg(target_os = "windows")]
pub unsafe fn show_scene_in_window(scene: &PaintScene, title: &str) {
    let scene_size = (scene.width.max(200.0), scene.height.max(100.0));
    let scene = scene.clone();
    let scene_ptr = Box::into_raw(Box::new(scene)) as isize;
    let mut backend = Win32Backend::new();
    let mut attributes = WindowAttributes::default();
    attributes.title = title.to_string();
    attributes.initial_size = LogicalSize::new(scene_size.0, scene_size.1);
    attributes.preferred_surface = SurfacePreference::Direct2D;

    if let Err(err) = backend.create_native_window(
        attributes,
        Some(render_paint_callback),
        scene_ptr,
    ) {
        drop(unsafe { Box::from_raw(scene_ptr as *mut PaintScene) });
        panic!("failed to create native Direct2D window: {err}");
    }
    let run_result = backend.run();

    drop(unsafe { Box::from_raw(scene_ptr as *mut PaintScene) });

    if let Err(err) = run_result {
        panic!("failed to run Win32 message loop: {err}");
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn render_paint_callback(
    hwnd: HWND,
    user_data: isize,
) {
    let scene_ptr = user_data as *const PaintScene;
    if scene_ptr.is_null() {
        return;
    }

    let scene = unsafe { scene_ptr.as_ref().expect("scene pointer provided by show_scene_in_window") };
    let _ = render_to_hwnd(hwnd, scene);
}

/// The actual rendering logic, wrapped in `unsafe` for COM/D2D FFI calls.
///
/// ## WIC bitmap pixel format
///
/// We use `GUID_WICPixelFormat32bppPBGRA` — premultiplied BGRA. This is
/// the native format Direct2D renders to. After reading back the pixels,
/// we un-premultiply the alpha and swap BGRA→RGBA for PixelContainer.
///
/// ### Premultiplied → straight alpha conversion
///
/// ```text
/// if alpha > 0:
///     R_straight = R_premul * 255 / alpha
///     G_straight = G_premul * 255 / alpha
///     B_straight = B_premul * 255 / alpha
/// else:
///     R = G = B = 0  (fully transparent pixel)
/// ```
#[cfg(target_os = "windows")]
unsafe fn render_unsafe(scene: &PaintScene, width: u32, height: u32) -> PixelContainer {
    // ── Step 1: Initialize COM ───────────────────────────────────────────
    //
    // COM must be initialized on every thread that uses COM objects.
    // COINIT_MULTITHREADED is used because we don't need a message loop.
    // If COM is already initialized, this returns S_FALSE (which is OK).
    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

    // ── Step 2: Create factories ─────────────────────────────────────────
    //
    // ID2D1Factory: creates render targets, geometries, etc.
    // IWICImagingFactory: creates WIC bitmaps for offscreen rendering.
    let d2d_factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)
        .expect("Failed to create D2D1 factory");

    let wic_factory: IWICImagingFactory =
        CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)
            .expect("Failed to create WIC imaging factory");

    // ── Step 3: Create offscreen WIC bitmap ──────────────────────────────
    //
    // The WIC bitmap is our offscreen render surface. Direct2D renders
    // into it, and we read back pixels from it when done.
    //
    // GUID_WICPixelFormat32bppPBGRA = premultiplied BGRA, 32 bits per pixel
    let pixel_format_guid = GUID::from_u128(0x6fddc324_4e03_4bfe_b185_3d77768dc910);
    let wic_bitmap: IWICBitmap = wic_factory
        .CreateBitmap(width, height, &pixel_format_guid, WICBitmapCacheOnLoad)
        .expect("Failed to create WIC bitmap");

    // ── Step 4: Create render target ─────────────────────────────────────
    //
    // A WIC bitmap render target draws into the WIC bitmap. This is an
    // offscreen rendering path — no window or HWND is involved.
    let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
        r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        usage: D2D1_RENDER_TARGET_USAGE_NONE,
        minLevel: Default::default(),
    };

    let render_target: ID2D1RenderTarget = d2d_factory
        .CreateWicBitmapRenderTarget(&wic_bitmap, &rt_props as *const _)
        .expect("Failed to create WIC bitmap render target");

    // ── Step 5: Render ───────────────────────────────────────────────────
    //
    // BeginDraw/EndDraw bracket all drawing operations. Clear sets the
    // entire surface to the background colour. Then we dispatch each
    // PaintInstruction to the appropriate D2D call.
    let (bg_r, bg_g, bg_b, bg_a) = parse_css_color(&scene.background);
    let bg_color = to_d2d_color(bg_r, bg_g, bg_b, bg_a);
    let mut ctx = RenderContext::new(d2d_factory.clone(), width as f32, height as f32);

    render_target.BeginDraw();
    render_target.Clear(Some(&bg_color as *const _));
    render_instructions(&mut ctx, &render_target, &scene.instructions);
    render_target.EndDraw(None, None).expect("EndDraw failed");

    // ── Step 6: Read back pixels ─────────────────────────────────────────
    //
    // Lock the WIC bitmap to get a pointer to the pixel data. The pixels
    // are in premultiplied BGRA format. We convert to straight RGBA.
    let lock = wic_bitmap
        .Lock(std::ptr::null(), WICBitmapLockRead.0 as u32)
        .expect("Failed to lock WIC bitmap");

    let stride = lock.GetStride().expect("Failed to get stride");

    let mut data_size: u32 = 0;
    let mut data_ptr: *mut u8 = std::ptr::null_mut();
    lock.GetDataPointer(&mut data_size, &mut data_ptr)
        .expect("Failed to get data pointer");

    // Validate that the returned buffer is large enough for our pixel loop.
    // Without this check, an unusual stride or truncated buffer could cause
    // an out-of-bounds read in the BGRA→RGBA conversion loop below.
    let required_size = (height as usize - 1) * stride as usize + width as usize * 4;
    assert!(
        data_size as usize >= required_size,
        "WIC bitmap data too small: got {} bytes, need {} bytes",
        data_size,
        required_size
    );

    let pbgra_slice = std::slice::from_raw_parts(data_ptr, data_size as usize);
    let total_pixels = (width as usize) * (height as usize);
    let mut rgba_data = vec![0u8; total_pixels * 4];

    // ── Step 7: Convert premultiplied BGRA → straight RGBA ──────────────
    //
    // WIC bitmap rows may have padding (stride > width*4). We must read
    // row by row using the stride, not assume contiguous pixel data.
    for row in 0..height as usize {
        let row_start = row * stride as usize;
        for col in 0..width as usize {
            let src_offset = row_start + col * 4;
            let dst_offset = (row * width as usize + col) * 4;

            let pb = pbgra_slice[src_offset]; // premultiplied B
            let pg = pbgra_slice[src_offset + 1]; // premultiplied G
            let pr = pbgra_slice[src_offset + 2]; // premultiplied R
            let a = pbgra_slice[src_offset + 3]; // alpha

            // Un-premultiply: straight_channel = premul_channel * 255 / alpha
            if a == 0 {
                rgba_data[dst_offset] = 0;
                rgba_data[dst_offset + 1] = 0;
                rgba_data[dst_offset + 2] = 0;
                rgba_data[dst_offset + 3] = 0;
            } else if a == 255 {
                // Fully opaque — no division needed, just swap B↔R
                rgba_data[dst_offset] = pr; // R
                rgba_data[dst_offset + 1] = pg; // G
                rgba_data[dst_offset + 2] = pb; // B
                rgba_data[dst_offset + 3] = 255; // A
            } else {
                // General case: un-premultiply
                let a_f = a as f32;
                rgba_data[dst_offset] = ((pr as f32 * 255.0 / a_f).round() as u8).min(255);
                rgba_data[dst_offset + 1] = ((pg as f32 * 255.0 / a_f).round() as u8).min(255);
                rgba_data[dst_offset + 2] = ((pb as f32 * 255.0 / a_f).round() as u8).min(255);
                rgba_data[dst_offset + 3] = a;
            }
        }
    }

    PixelContainer::from_data(width, height, rgba_data)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{
        GlyphPosition, PaintBase, PaintGlyphRun, PaintGroup, PaintInstruction, PaintRect,
        PaintScene,
    };

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ─── Colour parser tests ────────────────────────────────────────────────

    #[test]
    fn parse_hex_color_6_digit() {
        let (r, g, b, a) = parse_hex_color("#ff0000");
        assert!((r - 1.0).abs() < 0.01, "r should be 1.0");
        assert!((g - 0.0).abs() < 0.01, "g should be 0.0");
        assert!((b - 0.0).abs() < 0.01, "b should be 0.0");
        assert!((a - 1.0).abs() < 0.01, "a should be 1.0");
    }

    #[test]
    fn parse_hex_color_8_digit_with_alpha() {
        let (_r, _g, _b, a) = parse_hex_color("#00ff0080");
        assert!((a - 0.502).abs() < 0.01, "alpha should be ~0.502");
    }

    #[test]
    fn parse_hex_color_3_digit() {
        let (r, g, b, a) = parse_hex_color("#f00");
        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 0.0).abs() < 0.01);
        assert!((b - 0.0).abs() < 0.01);
        assert!((a - 1.0).abs() < 0.01);
    }

    #[test]
    fn parse_transparent() {
        let (r, g, b, a) = parse_hex_color("transparent");
        assert_eq!(a, 0.0);
        assert_eq!(r, 0.0);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    // ─── D2D colour conversion tests ────────────────────────────────────────

    #[cfg(target_os = "windows")]
    #[test]
    fn d2d_color_red() {
        let c = to_d2d_color(1.0, 0.0, 0.0, 1.0);
        assert!((c.r - 1.0).abs() < 0.001);
        assert!((c.g - 0.0).abs() < 0.001);
        assert!((c.b - 0.0).abs() < 0.001);
        assert!((c.a - 1.0).abs() < 0.001);
    }

    // ─── Rendering tests (Windows only) ─────────────────────────────────────

    #[cfg(target_os = "windows")]
    #[test]
    fn empty_scene_returns_empty_pixel_container() {
        let scene = PaintScene::new(0.0, 0.0);
        let pixels = render(&scene);
        assert_eq!(pixels.width, 0);
        assert_eq!(pixels.height, 0);
        assert!(pixels.data.is_empty());
    }

    /// Render a scene with a red rectangle on a white background.
    #[cfg(target_os = "windows")]
    #[test]
    fn render_red_rect_on_white() {
        let mut scene = PaintScene::new(100.0, 100.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                10.0, 10.0, 80.0, 80.0, "#ff0000",
            )));

        let pixels = render(&scene);
        assert_eq!(pixels.width, 100);
        assert_eq!(pixels.height, 100);

        // Centre of the red rectangle should be red
        let (r, g, b, a) = pixels.pixel_at(50, 50);
        assert_eq!(r, 255, "red channel at centre");
        assert_eq!(g, 0, "green channel at centre");
        assert_eq!(b, 0, "blue channel at centre");
        assert_eq!(a, 255, "alpha at centre");

        // Top-left corner is outside the rect → white background
        let (r, g, b, a) = pixels.pixel_at(2, 2);
        assert_eq!(r, 255, "red channel at corner (background)");
        assert_eq!(g, 255, "green channel at corner (background)");
        assert_eq!(b, 255, "blue channel at corner (background)");
        assert_eq!(a, 255, "alpha at corner (background)");
    }

    /// Render a solid green background with no instructions.
    #[cfg(target_os = "windows")]
    #[test]
    fn render_solid_green_background() {
        let mut scene = PaintScene::new(50.0, 50.0);
        scene.background = "#00ff00".to_string();

        let pixels = render(&scene);
        let (r, g, b, _a) = pixels.pixel_at(25, 25);
        assert_eq!(r, 0, "green bg: r should be 0");
        assert_eq!(g, 255, "green bg: g should be 255");
        assert_eq!(b, 0, "green bg: b should be 0");
    }

    /// Transparent rects should not draw anything.
    #[cfg(target_os = "windows")]
    #[test]
    fn transparent_rect_is_invisible() {
        let mut scene = PaintScene::new(50.0, 50.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                0.0,
                0.0,
                50.0,
                50.0,
                "transparent",
            )));

        let pixels = render(&scene);
        let (r, g, b, _a) = pixels.pixel_at(25, 25);
        assert_eq!(r, 255);
        assert_eq!(g, 255);
        assert_eq!(b, 255);
    }

    /// Group should recurse into children and render both rects.
    #[cfg(target_os = "windows")]
    #[test]
    fn group_recurses_into_children() {
        let mut scene = PaintScene::new(100.0, 50.0);
        let group = PaintInstruction::Group(PaintGroup {
            base: PaintBase::default(),
            children: vec![
                PaintInstruction::Rect(PaintRect::filled(0.0, 0.0, 50.0, 50.0, "#ff0000")),
                PaintInstruction::Rect(PaintRect::filled(50.0, 0.0, 50.0, 50.0, "#0000ff")),
            ],
            transform: None,
            opacity: None,
        });
        scene.instructions.push(group);

        let pixels = render(&scene);

        let (r, g, b, _a) = pixels.pixel_at(25, 25);
        assert_eq!(r, 255, "left half should be red");
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        let (r, g, b, _a) = pixels.pixel_at(75, 25);
        assert_eq!(r, 0, "right half should be blue");
        assert_eq!(g, 0);
        assert_eq!(b, 255);
    }

    /// Clip should restrict drawing to the clip rectangle.
    #[cfg(target_os = "windows")]
    #[test]
    fn clip_restricts_drawing() {
        let mut scene = PaintScene::new(100.0, 100.0);
        let clip = PaintInstruction::Clip(PaintClip {
            base: PaintBase::default(),
            x: 25.0,
            y: 25.0,
            width: 50.0,
            height: 50.0,
            children: vec![PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 100.0, 100.0, "#ff0000",
            ))],
        });
        scene.instructions.push(clip);

        let pixels = render(&scene);

        // Centre should be red (inside clip)
        let (r, g, b, _a) = pixels.pixel_at(50, 50);
        assert_eq!(r, 255, "centre should be red (inside clip)");
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        // Corner should be white (outside clip)
        let (r, g, b, _a) = pixels.pixel_at(5, 5);
        assert_eq!(r, 255, "corner should be white background");
        assert_eq!(g, 255);
        assert_eq!(b, 255);
    }

    /// Barcode-style pattern: alternating black/white vertical bars.
    #[cfg(target_os = "windows")]
    #[test]
    fn render_barcode_pattern() {
        let mut scene = PaintScene::new(200.0, 100.0);
        for i in 0..20u32 {
            if i % 2 == 0 {
                scene
                    .instructions
                    .push(PaintInstruction::Rect(PaintRect::filled(
                        i as f64 * 10.0,
                        0.0,
                        10.0,
                        80.0,
                        "#000000",
                    )));
            }
        }

        let pixels = render(&scene);
        assert_eq!(pixels.width, 200);
        assert_eq!(pixels.height, 100);

        let (r, g, b, _a) = pixels.pixel_at(5, 40);
        assert_eq!(r, 0, "black bar pixel should be black");
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        let (r, g, b, _a) = pixels.pixel_at(15, 40);
        assert_eq!(r, 255, "white gap pixel should be white");
        assert_eq!(g, 255);
        assert_eq!(b, 255);

        let (r, g, b, _a) = pixels.pixel_at(5, 90);
        assert_eq!(r, 255, "below bars should be white background");
        assert_eq!(g, 255);
        assert_eq!(b, 255);
    }

    /// QR-like checkerboard pattern (same as paint-metal's test).
    #[cfg(target_os = "windows")]
    #[test]
    fn render_black_modules_on_white() {
        let module_size = 4.0_f64;
        let mut scene = PaintScene::new(40.0, 40.0);

        for row in 0..4u32 {
            for col in 0..4u32 {
                if (row + col) % 2 == 0 {
                    scene
                        .instructions
                        .push(PaintInstruction::Rect(PaintRect::filled(
                            col as f64 * module_size,
                            row as f64 * module_size,
                            module_size,
                            module_size,
                            "#000000",
                        )));
                }
            }
        }

        let pixels = render(&scene);
        assert_eq!(pixels.width, 40);
        assert_eq!(pixels.height, 40);

        let (r, g, b, _a) = pixels.pixel_at(2, 2);
        assert_eq!(r, 0, "black module should have r=0");
        assert_eq!(g, 0, "black module should have g=0");
        assert_eq!(b, 0, "black module should have b=0");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn render_directwrite_glyph_run_draws_text_pixels() {
        use text_interfaces::{FontQuery, FontResolver, ShapeOptions, TextShaper};
        use text_native::{NativeResolver, NativeShaper};

        let resolver = NativeResolver::new();
        let shaper = NativeShaper::new();
        let handle = resolver.resolve(&FontQuery::named("Segoe UI")).unwrap();
        let shaped = shaper
            .shape("Hi", &handle, 32.0, &ShapeOptions::default())
            .unwrap();
        let run = &shaped.runs[0];

        let mut x = 12.0;
        let glyphs: Vec<GlyphPosition> = run
            .glyphs
            .iter()
            .map(|g| {
                let positioned = GlyphPosition {
                    glyph_id: g.glyph_id,
                    x,
                    y: 46.0,
                };
                x += g.x_advance as f64;
                positioned
            })
            .collect();

        let mut scene = PaintScene::new(100.0, 64.0);
        scene
            .instructions
            .push(PaintInstruction::GlyphRun(PaintGlyphRun {
                base: PaintBase::default(),
                glyphs,
                font_ref: run.font_ref.clone(),
                font_size: 32.0,
                fill: Some("#000000".to_string()),
            }));

        let pixels = render(&scene);
        let dark_pixels = pixels
            .data
            .chunks_exact(4)
            .filter(|px| px[0] < 64 && px[1] < 64 && px[2] < 64 && px[3] > 0)
            .count();
        assert!(dark_pixels > 20, "expected visible glyph pixels");
    }
}
