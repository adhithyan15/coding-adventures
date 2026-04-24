//! # paint-metal
//!
//! Metal GPU renderer for the paint-instructions scene model (P2D01).
//!
//! This crate takes a [`PaintScene`] (backend-neutral 2D paint instructions)
//! and renders it to a [`PixelContainer`] using Apple's Metal GPU API plus a
//! CoreText overlay for `PaintText` instructions.
//!
//! ## Current instruction support
//!
//! | Instruction       | Status                                                      |
//! |-------------------|-------------------------------------------------------------|
//! | `PaintRect`       | Fully implemented — solid-colour filled rects               |
//! | `PaintLine`       | Fully implemented — rendered as thin rectangles             |
//! | `PaintGroup`      | Fully implemented — recurses into children                  |
//! | `PaintClip`       | Partial — clips but no stencil                              |
//! | `PaintEllipse`    | Implemented — fan tessellation (64 triangles) + stroke ring |
//! | `PaintPath`       | Implemented — fan fill + segment stroke + Bézier approx     |
//! | `PaintText`       | Implemented — CoreText CTLine overlay into CG bitmap        |
//! | `PaintGlyphRun`   | Implemented — CoreText CTFontDrawGlyphs overlay             |
//! | `PaintLayer`      | Planned — offscreen texture + compose                       |
//! | `PaintGradient`   | Planned — MSL gradient shader                               |
//! | `PaintImage`      | Planned — texture from PixelContainer or URI                |
//!
//! ## Metal pipeline
//!
//! ```text
//! PaintScene
//!   │
//!   ├── 1. Create Metal device (MTLCreateSystemDefaultDevice)
//!   ├── 2. Create offscreen RGBA8 texture (width × height)
//!   ├── 3. Compile rect shader (solid-color triangles)
//!   ├── 4. Build render pipeline state
//!   ├── 5. Collect PaintRect / PaintLine / PaintEllipse / PaintPath → triangle vertex buffers
//!   │       (PaintText collected separately for CoreText overlay)
//!   ├── 6. Encode render commands into command buffer
//!   ├── 7. Commit and wait for GPU completion
//!   ├── 8. Read back RGBA8 pixels → PixelContainer
//!   ├── 9. CoreText overlay: draw PaintText via CTLine into CGBitmapContext
//!   └── 10. CoreText overlay: draw PaintGlyphRun via CTFontDrawGlyphs
//! ```
//!
//! ## Coordinate system
//!
//! `PaintScene` uses a **top-left origin** with Y increasing downward
//! (same as SVG, HTML Canvas, and CSS).
//!
//! Metal's normalised device coordinates (NDC) use a **centre origin**
//! with Y increasing upward, ranging from −1 to +1:
//!
//! ```text
//!  Scene coordinates:       Metal NDC:
//!  (0,0)──────(w,0)        (-1,+1)────(+1,+1)
//!    │              │           │              │
//!    │              │           │    (0,0)     │
//!    │              │           │              │
//!  (0,h)──────(w,h)        (-1,-1)────(+1,-1)
//! ```
//!
//! The vertex shader handles the conversion:
//! ```text
//! ndc.x = (pixel_x / width) * 2.0 - 1.0
//! ndc.y = 1.0 - (pixel_y / height) * 2.0
//! ```

// This crate's real implementation requires arm64 Apple Silicon — the
// objc_msgSend ABI for struct arguments differs between arm64 and
// x86_64, and the Metal / CoreGraphics frameworks are Apple-only.
//
// On non-Apple targets we expose a `render` stub that panics at
// runtime. This lets downstream workspace members (notably
// markdown-reader) continue to link against paint-metal on Linux CI
// without pulling in the Apple-only FFI surface. At runtime on
// non-Apple the panic makes the unsupported path loud.

pub const VERSION: &str = "0.2.0";

pub use paint_instructions::PixelContainer;

#[cfg(not(target_vendor = "apple"))]
pub fn render(_scene: &paint_instructions::PaintScene) -> PixelContainer {
    panic!(
        "paint-metal::render is only implemented on target_vendor = \"apple\"; \
         use a different paint backend on this platform."
    );
}

#[cfg(all(target_vendor = "apple", not(target_arch = "aarch64")))]
compile_error!("paint-metal requires arm64 Apple Silicon. Intel macOS is not supported.");

#[cfg(target_vendor = "apple")]
use objc_bridge::*;
use paint_instructions::{
    PaintEllipse, PaintInstruction, PaintLine, PaintPath, PaintRect, PaintScene, PathCommand,
};
#[allow(unused_imports)]
use std::ffi::{c_int, c_ulong};
#[allow(unused_imports)]
use std::ptr;

// ---------------------------------------------------------------------------
// Metal Shading Language (MSL) source code
// ---------------------------------------------------------------------------
//
// These shaders run on the GPU.  They are compiled at runtime by Metal from
// source strings via `newLibraryWithSource:options:error:`.
//
// MSL is a C++-like GPU language.  Each shader program has a vertex function
// (processes one vertex at a time) and a fragment function (computes one
// pixel at a time after the rasteriser interpolates between vertices).

/// MSL shader source for rendering solid-colour triangles (rects, ellipses, paths).
///
/// ## Data flow
///
/// ```text
/// CPU → vertex buffer:       [position(float2), color(float4)] per vertex
/// GPU vertex shader:         pixel_coords → NDC, pass color through
/// GPU rasteriser:            interpolates (position, color) across triangle
/// GPU fragment shader:       emits interpolated color as the pixel output
/// ```
const RECT_SHADER_SOURCE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct RectVertexOut {
    float4 position [[position]];
    float4 color;
};

vertex RectVertexOut rect_vertex(
    uint vid [[vertex_id]],
    const device float2* positions [[buffer(0)]],
    const device float4* colors    [[buffer(1)]],
    constant float2& viewport      [[buffer(2)]]
) {
    RectVertexOut out;
    float2 px = positions[vid];
    out.position = float4(
        (px.x / viewport.x) * 2.0 - 1.0,
        1.0 - (px.y / viewport.y) * 2.0,
        0.0,
        1.0
    );
    out.color = colors[vid];
    return out;
}

fragment float4 rect_fragment(RectVertexOut in [[stage_in]]) {
    return in.color;
}
"#;

// ---------------------------------------------------------------------------
// Color parsing
// ---------------------------------------------------------------------------

/// Parse a hex colour string to RGBA floats in the range 0.0–1.0.
///
/// Supported formats:
/// - `"#rrggbb"`   → (r, g, b, 1.0)
/// - `"#rrggbbaa"` → (r, g, b, a)
/// - `"#rgb"`      → expanded to `#rrggbb`
/// - `"transparent"` / anything else → (0.0, 0.0, 0.0, 0.0)
///
/// Returns `(0.0, 0.0, 0.0, 1.0)` for unrecognised non-transparent input.
fn parse_hex_color(s: &str) -> (f64, f64, f64, f64) {
    let s = s.trim();
    if s == "transparent" {
        return (0.0, 0.0, 0.0, 0.0);
    }
    // CSS rgb()/rgba() support — layout-to-paint emits these.
    if let Some(inner) = s.strip_prefix("rgba(").and_then(|t| t.strip_suffix(')')) {
        return parse_rgb_components(inner, true);
    }
    if let Some(inner) = s.strip_prefix("rgb(").and_then(|t| t.strip_suffix(')')) {
        return parse_rgb_components(inner, false);
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

/// Parse the comma-separated r,g,b(,a) components from inside an
/// `rgb(...)` / `rgba(...)` CSS string. r/g/b are 0..=255 decimal, a
/// is 0..=1 decimal. Missing / malformed components clamp gracefully
/// toward opaque black.
fn parse_rgb_components(inner: &str, has_alpha: bool) -> (f64, f64, f64, f64) {
    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
    if parts.len() < 3 {
        return (0.0, 0.0, 0.0, 1.0);
    }
    let r = parts[0].parse::<f64>().unwrap_or(0.0) / 255.0;
    let g = parts[1].parse::<f64>().unwrap_or(0.0) / 255.0;
    let b = parts[2].parse::<f64>().unwrap_or(0.0) / 255.0;
    let a = if has_alpha && parts.len() >= 4 {
        parts[3].parse::<f64>().unwrap_or(1.0)
    } else {
        1.0
    };
    (
        r.clamp(0.0, 1.0),
        g.clamp(0.0, 1.0),
        b.clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    )
}

// ---------------------------------------------------------------------------
// Vertex generation — PaintInstruction → triangle vertices
// ---------------------------------------------------------------------------
//
// Each visible instruction becomes some number of triangles.  We collect
// all positions and colours into flat arrays, then upload them to GPU buffers
// in one batch.  This is more efficient than one draw call per instruction.
//
// The GPU only needs the triangle vertex stream — it has no concept of
// "rectangles", "ellipses", or "paths".  Everything is triangles.
//
/// Collect triangle vertices from a [`PaintInstruction`] tree.
///
/// - Rects, lines, ellipses, and paths → triangle vertices in `positions`/`colors`.
/// - Group and Clip nodes are recursed into (up to `MAX_GROUP_DEPTH` levels).
/// - GlyphRun is rendered by the CoreText overlay (glyph_run_overlay module).
/// - Text (PaintText) is Canvas/SVG/DOM-only — not rendered by Metal.
/// - Layer, Gradient, Image are deferred to P2D08.
///
/// `depth` must be 0 on the initial call; it is incremented for each recursive Group/Clip.
fn collect_geometry(
    instructions: &[PaintInstruction],
    positions: &mut Vec<f32>,
    colors: &mut Vec<f32>,
    depth: usize,
) {
    // Guard against stack overflow from pathologically deep instruction trees.
    const MAX_GROUP_DEPTH: usize = 128;
    if depth > MAX_GROUP_DEPTH {
        return;
    }

    for instr in instructions {
        match instr {
            PaintInstruction::Rect(rect) => {
                add_rect_vertices(rect, positions, colors);
            }
            PaintInstruction::Line(line) => {
                add_line_vertices(line, positions, colors);
            }
            PaintInstruction::Ellipse(ellipse) => {
                add_ellipse_vertices(ellipse, positions, colors);
            }
            PaintInstruction::Path(path) => {
                add_path_vertices(path, positions, colors);
            }
            PaintInstruction::Group(group) => {
                collect_geometry(&group.children, positions, colors, depth + 1);
            }
            PaintInstruction::Clip(clip) => {
                // Render clip children without a stencil clip for now.
                collect_geometry(&clip.children, positions, colors, depth + 1);
            }
            // Rendered via CoreText glyph_run_overlay:
            PaintInstruction::GlyphRun(_) => {}
            // PaintText is Canvas/DOM-only — not handled by Metal.
            PaintInstruction::Text(_) => {}
            // Deferred to P2D08:
            PaintInstruction::Layer(_)
            | PaintInstruction::Gradient(_)
            | PaintInstruction::Image(_) => {}
        }
    }
}

/// Add 6 triangle vertices for a `PaintRect` fill.
///
/// A rectangle is two right triangles sharing the diagonal:
///
/// ```text
/// (x,   y) ─────── (x+w, y)
///   │ ╲                  │
///   │   ╲                │
///   │     ╲              │
///   │       ╲            │
/// (x, y+h) ──── (x+w, y+h)
///
/// Triangle 1: top-left, top-right, bottom-left
/// Triangle 2: top-right, bottom-right, bottom-left
/// ```
fn add_rect_vertices(rect: &PaintRect, positions: &mut Vec<f32>, colors: &mut Vec<f32>) {
    let fill = rect.fill.as_deref().unwrap_or("transparent");
    let (r, g, b, a) = parse_hex_color(fill);
    if a > 0.0 {
        let (r, g, b, a) = (r as f32, g as f32, b as f32, a as f32);
        let x = rect.x as f32;
        let y = rect.y as f32;
        let w = rect.width as f32;
        let h = rect.height as f32;
        // Triangle 1: top-left → top-right → bottom-left
        positions.extend_from_slice(&[x, y, x + w, y, x, y + h]);
        colors.extend_from_slice(&[r, g, b, a, r, g, b, a, r, g, b, a]);
        // Triangle 2: top-right → bottom-right → bottom-left
        positions.extend_from_slice(&[x + w, y, x + w, y + h, x, y + h]);
        colors.extend_from_slice(&[r, g, b, a, r, g, b, a, r, g, b, a]);
    }

    // Stroke: 4 thin edge rects (top, right, bottom, left)
    if let Some(stroke_str) = rect.stroke.as_deref() {
        let (sr, sg, sb, sa) = parse_hex_color(stroke_str);
        if sa > 0.0 {
            let sw = rect.stroke_width.unwrap_or(1.0) as f32;
            let (sr, sg, sb, sa) = (sr as f32, sg as f32, sb as f32, sa as f32);
            let x = rect.x as f32;
            let y = rect.y as f32;
            let w = rect.width as f32;
            let h = rect.height as f32;
            // top edge
            emit_filled_rect(x, y, w, sw, sr, sg, sb, sa, positions, colors);
            // bottom edge
            emit_filled_rect(x, y + h - sw, w, sw, sr, sg, sb, sa, positions, colors);
            // left edge
            emit_filled_rect(x, y, sw, h, sr, sg, sb, sa, positions, colors);
            // right edge
            emit_filled_rect(x + w - sw, y, sw, h, sr, sg, sb, sa, positions, colors);
        }
    }
}

/// Emit a filled axis-aligned rectangle as two triangles (helper).
fn emit_filled_rect(
    x: f32, y: f32, w: f32, h: f32,
    r: f32, g: f32, b: f32, a: f32,
    positions: &mut Vec<f32>,
    colors: &mut Vec<f32>,
) {
    positions.extend_from_slice(&[x, y, x + w, y, x, y + h]);
    colors.extend_from_slice(&[r, g, b, a, r, g, b, a, r, g, b, a]);
    positions.extend_from_slice(&[x + w, y, x + w, y + h, x, y + h]);
    colors.extend_from_slice(&[r, g, b, a, r, g, b, a, r, g, b, a]);
}

/// Render a `PaintLine` as a thin rectangle perpendicular to the line direction.
///
/// A line from `(x1, y1)` to `(x2, y2)` with `stroke_width` becomes a thin
/// rectangle centred on the line:
///
/// ```text
/// p0 ─────────────── p2     ← offset by nx,ny from the line
///  │   actual line   │
/// p1 ─────────────── p3     ← offset by -nx,-ny from the line
/// ```
fn add_line_vertices(line: &PaintLine, positions: &mut Vec<f32>, colors: &mut Vec<f32>) {
    let (r, g, b, a) = parse_hex_color(&line.stroke);
    if a == 0.0 {
        return;
    }
    let (r, g, b, a) = (r as f32, g as f32, b as f32, a as f32);

    let x1 = line.x1 as f32;
    let y1 = line.y1 as f32;
    let x2 = line.x2 as f32;
    let y2 = line.y2 as f32;
    let half_w = (line.stroke_width.unwrap_or(1.0) as f32) / 2.0;

    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 {
        return; // degenerate zero-length line
    }
    // Perpendicular unit normal scaled by half_w
    let nx = -dy / len * half_w;
    let ny = dx / len * half_w;

    let p0x = x1 + nx; let p0y = y1 + ny;
    let p1x = x1 - nx; let p1y = y1 - ny;
    let p2x = x2 + nx; let p2y = y2 + ny;
    let p3x = x2 - nx; let p3y = y2 - ny;

    positions.extend_from_slice(&[p0x, p0y, p2x, p2y, p1x, p1y]);
    colors.extend_from_slice(&[r, g, b, a, r, g, b, a, r, g, b, a]);
    positions.extend_from_slice(&[p2x, p2y, p3x, p3y, p1x, p1y]);
    colors.extend_from_slice(&[r, g, b, a, r, g, b, a, r, g, b, a]);
}

/// Tessellate a `PaintEllipse` into GPU triangles.
///
/// ## Fill — fan tessellation
///
/// The filled ellipse is approximated by N=64 triangles radiating from the
/// centre, each covering one arc slice:
///
/// ```text
///          p[0]
///         ╱    ╲
///        ╱  T0   ╲
///  center ── T1 ── p[1]
///        ╲  T2   ╱
///         ╲    ╱
///          p[2]
/// ```
///
/// Each triangle: `(center, p[i], p[(i+1) % N])`.
///
/// ## Stroke — ring of N thin quads
///
/// A ring quad is the trapezoid between outer point `p_out[i]`
/// and inner point `p_in[i]` at `(rx - sw)`, `(ry - sw)`.
const ELLIPSE_SEGMENTS: usize = 64;

fn add_ellipse_vertices(ellipse: &PaintEllipse, positions: &mut Vec<f32>, colors: &mut Vec<f32>) {
    use std::f64::consts::TAU;
    let cx = ellipse.cx as f32;
    let cy = ellipse.cy as f32;
    let rx = ellipse.rx as f32;
    let ry = ellipse.ry as f32;

    // Pre-compute perimeter points
    let mut pts: Vec<(f32, f32)> = Vec::with_capacity(ELLIPSE_SEGMENTS);
    for i in 0..ELLIPSE_SEGMENTS {
        let angle = (i as f64 / ELLIPSE_SEGMENTS as f64) * TAU;
        pts.push((
            cx + rx * angle.cos() as f32,
            cy + ry * angle.sin() as f32,
        ));
    }

    // Fill: fan from centre
    if let Some(fill_str) = ellipse.fill.as_deref() {
        let (r, g, b, a) = parse_hex_color(fill_str);
        if a > 0.0 {
            let (r, g, b, a) = (r as f32, g as f32, b as f32, a as f32);
            for i in 0..ELLIPSE_SEGMENTS {
                let (ax, ay) = pts[i];
                let (bx, by) = pts[(i + 1) % ELLIPSE_SEGMENTS];
                positions.extend_from_slice(&[cx, cy, ax, ay, bx, by]);
                colors.extend_from_slice(&[r, g, b, a, r, g, b, a, r, g, b, a]);
            }
        }
    }

    // Stroke: ring of thin quads
    if let Some(stroke_str) = ellipse.stroke.as_deref() {
        let (sr, sg, sb, sa) = parse_hex_color(stroke_str);
        if sa > 0.0 {
            let (sr, sg, sb, sa) = (sr as f32, sg as f32, sb as f32, sa as f32);
            let sw = ellipse.stroke_width.unwrap_or(1.0) as f32;
            let inner_rx = (rx - sw).max(0.0);
            let inner_ry = (ry - sw).max(0.0);
            // Inner perimeter points
            let mut inner: Vec<(f32, f32)> = Vec::with_capacity(ELLIPSE_SEGMENTS);
            for i in 0..ELLIPSE_SEGMENTS {
                let angle = (i as f64 / ELLIPSE_SEGMENTS as f64) * TAU;
                inner.push((
                    cx + inner_rx * angle.cos() as f32,
                    cy + inner_ry * angle.sin() as f32,
                ));
            }
            // Each quad: outer[i], outer[i+1], inner[i], inner[i+1]
            for i in 0..ELLIPSE_SEGMENTS {
                let j = (i + 1) % ELLIPSE_SEGMENTS;
                let (ox0, oy0) = pts[i];
                let (ox1, oy1) = pts[j];
                let (ix0, iy0) = inner[i];
                let (ix1, iy1) = inner[j];
                // Two triangles per quad
                positions.extend_from_slice(&[ox0, oy0, ox1, oy1, ix0, iy0]);
                colors.extend_from_slice(&[sr, sg, sb, sa, sr, sg, sb, sa, sr, sg, sb, sa]);
                positions.extend_from_slice(&[ox1, oy1, ix1, iy1, ix0, iy0]);
                colors.extend_from_slice(&[sr, sg, sb, sa, sr, sg, sb, sa, sr, sg, sb, sa]);
            }
        }
    }
}

/// Tessellate a `PaintPath` into GPU triangles.
///
/// ## Fill — fan tessellation from first point
///
/// Correct for convex polygons, which covers all shapes that
/// `diagram-to-paint` emits (rects, diamonds, arrowheads).  Non-convex
/// polygons may have artefacts, but diagrams never produce them.
///
/// The fan pivots at `pts[0]` and covers every subsequent consecutive pair:
/// `(pts[0], pts[i], pts[i+1])` for `i in 1..n-1`.
///
/// ## Stroke — segment-to-rectangle
///
/// Each `LineTo` (and Bézier approximation) segment becomes a thin
/// rectangle perpendicular to the segment direction, width = `stroke_width`.
///
/// ## Bézier curves
///
/// `QuadTo` is approximated with 8 linear segments via de Casteljau.
/// `CubicTo` is approximated with 8 linear segments via de Casteljau.
/// `ArcTo` is not yet tessellated — it is silently skipped.
fn add_path_vertices(path: &PaintPath, positions: &mut Vec<f32>, colors: &mut Vec<f32>) {
    // Guard: each CubicTo/QuadTo expands to 8 points; cap total to prevent OOM.
    const MAX_PATH_COMMANDS: usize = 10_000;
    if path.commands.len() > MAX_PATH_COMMANDS {
        return;
    }

    // Flatten all path commands into a sequence of (x, y) points.
    // Each subpath (starting at MoveTo) is collected, then we tessellate fill
    // and stroke across all points.
    let mut subpaths: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut current: Vec<(f32, f32)> = Vec::new();
    let mut cx = 0.0f32;
    let mut cy = 0.0f32;
    let mut first_x = 0.0f32;
    let mut first_y = 0.0f32;

    for cmd in &path.commands {
        match cmd {
            PathCommand::MoveTo { x, y } => {
                if !current.is_empty() {
                    subpaths.push(current.clone());
                    current.clear();
                }
                cx = *x as f32;
                cy = *y as f32;
                first_x = cx;
                first_y = cy;
                current.push((cx, cy));
            }
            PathCommand::LineTo { x, y } => {
                cx = *x as f32;
                cy = *y as f32;
                current.push((cx, cy));
            }
            PathCommand::QuadTo { cx: qcx, cy: qcy, x, y } => {
                // De Casteljau — 8 linear segments
                let p0x = cx; let p0y = cy;
                let p1x = *qcx as f32; let p1y = *qcy as f32;
                let p2x = *x as f32;   let p2y = *y as f32;
                for k in 1..=8u32 {
                    let t = k as f32 / 8.0;
                    let u = 1.0 - t;
                    let qx = u * u * p0x + 2.0 * u * t * p1x + t * t * p2x;
                    let qy = u * u * p0y + 2.0 * u * t * p1y + t * t * p2y;
                    current.push((qx, qy));
                }
                cx = p2x; cy = p2y;
            }
            PathCommand::CubicTo { cx1, cy1, cx2, cy2, x, y } => {
                // De Casteljau — 8 linear segments
                let p0x = cx; let p0y = cy;
                let p1x = *cx1 as f32; let p1y = *cy1 as f32;
                let p2x = *cx2 as f32; let p2y = *cy2 as f32;
                let p3x = *x as f32;   let p3y = *y as f32;
                for k in 1..=8u32 {
                    let t = k as f32 / 8.0;
                    let u = 1.0 - t;
                    let qx = u*u*u*p0x + 3.0*u*u*t*p1x + 3.0*u*t*t*p2x + t*t*t*p3x;
                    let qy = u*u*u*p0y + 3.0*u*u*t*p1y + 3.0*u*t*t*p2y + t*t*t*p3y;
                    current.push((qx, qy));
                }
                cx = p3x; cy = p3y;
            }
            PathCommand::ArcTo { .. } => {
                // ArcTo: not tessellated yet — skip. Diagrams don't use arcs.
            }
            PathCommand::Close => {
                current.push((first_x, first_y));
                subpaths.push(current.clone());
                current.clear();
            }
        }
    }
    if !current.is_empty() {
        subpaths.push(current);
    }

    // Fill: fan tessellation per subpath
    if let Some(fill_str) = path.fill.as_deref().filter(|s| *s != "none") {
        let (r, g, b, a) = parse_hex_color(fill_str);
        if a > 0.0 {
            let (r, g, b, a) = (r as f32, g as f32, b as f32, a as f32);
            for pts in &subpaths {
                if pts.len() < 3 {
                    continue;
                }
                let (fx, fy) = pts[0];
                for i in 1..pts.len() - 1 {
                    let (ax, ay) = pts[i];
                    let (bx, by) = pts[i + 1];
                    positions.extend_from_slice(&[fx, fy, ax, ay, bx, by]);
                    colors.extend_from_slice(&[r, g, b, a, r, g, b, a, r, g, b, a]);
                }
            }
        }
    }

    // Stroke: segment rectangles per subpath
    if let Some(stroke_str) = path.stroke.as_deref().filter(|s| *s != "none") {
        let (sr, sg, sb, sa) = parse_hex_color(stroke_str);
        if sa > 0.0 {
            let (sr, sg, sb, sa) = (sr as f32, sg as f32, sb as f32, sa as f32);
            let half_sw = (path.stroke_width.unwrap_or(1.0) as f32) / 2.0;
            for pts in &subpaths {
                for i in 0..pts.len().saturating_sub(1) {
                    let (x1, y1) = pts[i];
                    let (x2, y2) = pts[i + 1];
                    let dx = x2 - x1;
                    let dy = y2 - y1;
                    let len = (dx * dx + dy * dy).sqrt();
                    if len < 0.001 {
                        continue;
                    }
                    let nx = -dy / len * half_sw;
                    let ny = dx / len * half_sw;
                    // Quad corners
                    let (ax, ay) = (x1 + nx, y1 + ny);
                    let (bx, by) = (x1 - nx, y1 - ny);
                    let (cx2, cy2) = (x2 + nx, y2 + ny);
                    let (dx2, dy2) = (x2 - nx, y2 - ny);
                    positions.extend_from_slice(&[ax, ay, cx2, cy2, bx, by]);
                    colors.extend_from_slice(&[sr, sg, sb, sa, sr, sg, sb, sa, sr, sg, sb, sa]);
                    positions.extend_from_slice(&[cx2, cy2, dx2, dy2, bx, by]);
                    colors.extend_from_slice(&[sr, sg, sb, sa, sr, sg, sb, sa, sr, sg, sb, sa]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Render a [`PaintScene`] to a [`PixelContainer`] using the Metal GPU.
///
/// ## Pipeline
///
/// 1. Create a Metal device and command queue
/// 2. Allocate an offscreen RGBA8 texture at `scene.width × scene.height`
/// 3. Compile the rect shader from MSL source
/// 4. Convert `PaintInstruction` tree to triangle vertex buffers (Rect, Line, Ellipse, Path)
/// 5. Encode a render pass that clears to `scene.background` then draws all triangles
/// 6. Commit the command buffer and wait for GPU completion
/// 7. Read back the RGBA8 pixels with `getBytes()`
/// 8. Apply CoreText overlay for PaintGlyphRun
/// 9. Return the pixels as a `PixelContainer`
#[cfg(target_vendor = "apple")]
pub fn render(scene: &PaintScene) -> PixelContainer {
    let mut pixels = unsafe { render_unsafe(scene) };

    // Render PaintGlyphRun instructions via CoreText glyph drawing.
    unsafe {
        glyph_run_overlay::overlay_coretext_glyph_runs(scene, &mut pixels);
    }
    pixels
}

#[cfg(target_vendor = "apple")]
unsafe fn render_unsafe(scene: &PaintScene) -> PixelContainer {
    // Guard against NaN/Inf before casting — `f64::INFINITY as u32` saturates to
    // u32::MAX on Rust (4 294 967 295), which would bypass the zero-size check and
    // trigger the dimension assert with a confusing message.
    const MAX_DIMENSION_F: f64 = 16384.0;
    if !scene.width.is_finite()
        || !scene.height.is_finite()
        || scene.width > MAX_DIMENSION_F
        || scene.height > MAX_DIMENSION_F
    {
        panic!(
            "Scene dimensions {}×{} are non-finite or exceed maximum {}×{}",
            scene.width, scene.height, MAX_DIMENSION_F, MAX_DIMENSION_F
        );
    }

    let width = scene.width as u32;
    let height = scene.height as u32;

    if width == 0 || height == 0 {
        return PixelContainer::new(width, height);
    }

    // ── Step 1: Metal device + command queue ─────────────────────────────────
    let device = MTLCreateSystemDefaultDevice();
    assert!(!device.is_null(), "No Metal-capable GPU found");

    let command_queue = msg_send_id(device, "newCommandQueue");
    assert!(!command_queue.is_null(), "Failed to create command queue");

    // ── Step 2: Offscreen RGBA8 texture ──────────────────────────────────────
    let texture = create_offscreen_texture(device, width, height);

    // ── Step 3 & 4: Compile shader + build pipeline state ────────────────────
    let rect_pipeline = create_rect_pipeline(device);

    // ── Step 5: Generate triangle vertices from PaintInstructions ────────────
    let mut positions: Vec<f32> = Vec::new();
    let mut colors: Vec<f32> = Vec::new();
    collect_geometry(&scene.instructions, &mut positions, &mut colors, 0);

    // ── Step 6: Render pass ───────────────────────────────────────────────────
    let pass_desc_class = class("MTLRenderPassDescriptor");
    let pass_desc: Id = msg_send_class(pass_desc_class, "renderPassDescriptor");

    let color_attachments = msg_send_id(pass_desc, "colorAttachments");
    let attachment0: Id = msg!(color_attachments, "objectAtIndexedSubscript:", 0usize);

    let (cr, cg, cb, ca) = parse_hex_color(&scene.background);

    msg!(attachment0, "setTexture:", texture);
    msg!(attachment0, "setLoadAction:", MTL_LOAD_ACTION_CLEAR as usize);
    msg!(attachment0, "setStoreAction:", MTL_STORE_ACTION_STORE as usize);

    // MTLClearColor is 4 doubles — passed as HFA in d0-d3 on arm64
    let clear_color = MTLClearColor { red: cr, green: cg, blue: cb, alpha: ca };
    let set_clear_color: unsafe extern "C" fn(Id, Sel, MTLClearColor) =
        std::mem::transmute(objc_msgSend as *const ());
    set_clear_color(attachment0, sel("setClearColor:"), clear_color);

    let command_buffer = msg_send_id(command_queue, "commandBuffer");
    let encoder: Id = msg!(command_buffer, "renderCommandEncoderWithDescriptor:", pass_desc);

    let viewport_size: [f32; 2] = [width as f32, height as f32];

    // Draw all rectangles, lines, ellipses, paths (collected as triangles)
    if !positions.is_empty() {
        let vertex_count = positions.len() / 2;

        let pos_buffer = create_buffer(device, &positions);
        let color_buffer = create_buffer(device, &colors);

        msg!(encoder, "setRenderPipelineState:", rect_pipeline);
        msg!(encoder, "setVertexBuffer:offset:atIndex:", pos_buffer, 0usize, 0usize);
        msg!(encoder, "setVertexBuffer:offset:atIndex:", color_buffer, 0usize, 1usize);

        let vp_ptr = viewport_size.as_ptr() as *const std::ffi::c_void as Id;
        msg!(encoder, "setVertexBytes:length:atIndex:", vp_ptr, 8usize, 2usize);

        msg!(encoder, "drawPrimitives:vertexStart:vertexCount:",
            MTL_PRIMITIVE_TYPE_TRIANGLE as usize, 0usize, vertex_count);

        release(pos_buffer);
        release(color_buffer);
    }

    msg!(encoder, "endEncoding");
    msg!(command_buffer, "commit");
    msg!(command_buffer, "waitUntilCompleted");

    // ── Step 7 & 8: Read back pixels ─────────────────────────────────────────
    let pixel_container = read_back_pixels(texture, width, height);

    // Clean up Metal objects we own
    release(texture);
    release(rect_pipeline);
    release(command_queue);
    release(device);

    pixel_container
}

// ---------------------------------------------------------------------------
// Metal helper functions
// ---------------------------------------------------------------------------

#[cfg(target_vendor = "apple")]
unsafe fn create_offscreen_texture(device: Id, width: u32, height: u32) -> Id {
    let desc = alloc_init("MTLTextureDescriptor");

    // MTLPixelFormatRGBA8Unorm = 70
    msg!(desc, "setPixelFormat:", MTL_PIXEL_FORMAT_RGBA8_UNORM as usize);
    msg!(desc, "setWidth:", width as usize);
    msg!(desc, "setHeight:", height as usize);
    // MTLTextureType2D = 2
    msg!(desc, "setTextureType:", MTL_TEXTURE_TYPE_2D as usize);

    let usage = MTL_TEXTURE_USAGE_RENDER_TARGET | MTL_TEXTURE_USAGE_SHADER_READ;
    msg!(desc, "setUsage:", usage as usize);

    let texture: Id = msg!(device, "newTextureWithDescriptor:", desc);
    release(desc);
    assert!(!texture.is_null(), "Failed to create offscreen texture");
    texture
}

#[cfg(target_vendor = "apple")]
unsafe fn compile_shader_library(device: Id, source: &str) -> Id {
    let source_ns = nsstring(source);
    let options: Id = ptr::null_mut();
    let mut error: Id = ptr::null_mut();
    let library: Id = msg!(
        device, "newLibraryWithSource:options:error:",
        source_ns, options, &mut error as *mut Id
    );
    CFRelease(source_ns);

    if library.is_null() {
        panic!("Metal shader compilation failed — check MSL source");
    }
    library
}

#[cfg(target_vendor = "apple")]
unsafe fn create_rect_pipeline(device: Id) -> Id {
    let library = compile_shader_library(device, RECT_SHADER_SOURCE);

    let vname = nsstring("rect_vertex");
    let fname = nsstring("rect_fragment");
    let vertex_fn: Id = msg!(library, "newFunctionWithName:", vname);
    let fragment_fn: Id = msg!(library, "newFunctionWithName:", fname);
    CFRelease(vname);
    CFRelease(fname);

    assert!(!vertex_fn.is_null(), "rect_vertex shader not found");
    assert!(!fragment_fn.is_null(), "rect_fragment shader not found");

    let desc = alloc_init("MTLRenderPipelineDescriptor");
    msg!(desc, "setVertexFunction:", vertex_fn);
    msg!(desc, "setFragmentFunction:", fragment_fn);

    setup_pipeline_color_attachment(desc);

    let mut error: Id = ptr::null_mut();
    let pipeline: Id = msg!(
        device, "newRenderPipelineStateWithDescriptor:error:",
        desc, &mut error as *mut Id
    );

    release(vertex_fn);
    release(fragment_fn);
    release(library);
    release(desc);

    assert!(!pipeline.is_null(), "Failed to create rect render pipeline state");
    pipeline
}

#[cfg(target_vendor = "apple")]
unsafe fn setup_pipeline_color_attachment(desc: Id) {
    let attachments = msg_send_id(desc, "colorAttachments");
    let att0: Id = msg!(attachments, "objectAtIndexedSubscript:", 0usize);
    msg!(att0, "setPixelFormat:", MTL_PIXEL_FORMAT_RGBA8_UNORM as usize);

    // Enable standard src-over alpha blending so transparent pixels composite correctly.
    // The formula is:  dst = src.rgb * src.a + dst.rgb * (1 - src.a)
    msg!(att0, "setBlendingEnabled:", 1usize);
    msg!(att0, "setSourceRGBBlendFactor:", 4usize);        // sourceAlpha
    msg!(att0, "setDestinationRGBBlendFactor:", 5usize);   // oneMinusSourceAlpha
    msg!(att0, "setSourceAlphaBlendFactor:", 1usize);       // one
    msg!(att0, "setDestinationAlphaBlendFactor:", 5usize); // oneMinusSourceAlpha
}

#[cfg(target_vendor = "apple")]
unsafe fn create_buffer(device: Id, data: &[f32]) -> Id {
    let byte_len = data.len() * std::mem::size_of::<f32>();
    // MTLResourceStorageModeShared = 0
    let buffer: Id = msg!(
        device, "newBufferWithBytes:length:options:",
        data.as_ptr() as Id, byte_len as usize, 0usize
    );
    assert!(!buffer.is_null(), "Failed to create Metal buffer");
    buffer
}

#[cfg(target_vendor = "apple")]
unsafe fn read_back_pixels(texture: Id, width: u32, height: u32) -> PixelContainer {
    let bytes_per_row = (width as usize) * 4;
    let total_bytes = bytes_per_row * (height as usize);
    let mut data = vec![0u8; total_bytes];

    let region = MTLRegion {
        origin: MTLOrigin { x: 0, y: 0, z: 0 },
        size: MTLSize {
            width: width as c_ulong,
            height: height as c_ulong,
            depth: 1,
        },
    };

    // On arm64, composite types > 16 bytes are passed indirectly (by pointer).
    // MTLRegion is 48 bytes, so we use a typed function pointer that lets the
    // compiler generate the correct ABI (pass by value triggers indirect passing).
    let get_bytes: unsafe extern "C" fn(
        Id, Sel,
        *mut u8,   // bytes pointer
        usize,     // bytesPerRow
        MTLRegion, // region (compiler passes indirectly on arm64)
        usize,     // mipmapLevel
    ) = std::mem::transmute(objc_msgSend as *const ());
    get_bytes(
        texture, sel("getBytes:bytesPerRow:fromRegion:mipmapLevel:"),
        data.as_mut_ptr(), bytes_per_row,
        region, 0,
    );

    PixelContainer::from_data(width, height, data)
}


// ---------------------------------------------------------------------------
// CoreText glyph-run overlay (Apple only)
// ---------------------------------------------------------------------------
//
// The Metal render pass above rasterizes rects / lines / groups / clips
// into RGBA bytes. To render text, we resolve `PaintGlyphRun`
// instructions with `font_ref` starting `"coretext:"` by wrapping the
// RGBA pixel buffer in a `CGBitmapContext` and calling `CTFontDrawGlyphs`.
//
// The font_ref string carries everything needed to recreate the
// CTFontRef: `"coretext:<PostScript-name>@<size>"`. We parse it and
// call `CTFontCreateWithName` per run. Creating a CTFontRef is cheap —
// CoreText caches internally — so this is acceptable for v1 without
// a separate font registry.

#[cfg(target_vendor = "apple")]
mod glyph_run_overlay {
    use objc_bridge::{
        cfstring_checked, CFRelease, CGAffineTransform, CGBitmapContextCreate,
        CGColorSpaceCreateDeviceRGB, CGColorSpaceRelease, CGContextRelease,
        CGContextRestoreGState, CGContextSaveGState, CGContextSetRGBFillColor,
        CGContextSetShouldAntialias, CGContextSetShouldSmoothFonts, CGContextSetTextMatrix,
        CGPoint, CTFontCreateWithName, CTFontDrawGlyphs, CGContextRef, Id,
        K_CG_IMAGE_ALPHA_PREMULTIPLIED_FIRST, K_CG_BITMAP_BYTE_ORDER_32_LITTLE, NIL,
    };
    use paint_instructions::{
        PaintGlyphRun, PaintInstruction, PaintScene, PixelContainer,
    };

    pub(super) unsafe fn overlay_coretext_glyph_runs(
        scene: &PaintScene,
        pixels: &mut PixelContainer,
    ) {
        let width = pixels.width as usize;
        let height = pixels.height as usize;
        if width == 0 || height == 0 {
            return;
        }

        let runs = collect_coretext_runs(&scene.instructions);
        if runs.is_empty() {
            return;
        }

        let color_space = CGColorSpaceCreateDeviceRGB();
        if color_space.is_null() {
            return;
        }

        // SAFETY: see text_overlay::overlay_paint_text for the aliasing argument.
        // `as_mut_ptr()` is required here because CGBitmapContextCreate writes
        // glyph pixels through this pointer. The context is fully released before return.
        let data_ptr = pixels.data.as_mut_ptr() as *mut std::ffi::c_void;

        let ctx: CGContextRef = CGBitmapContextCreate(
            data_ptr,
            width,
            height,
            8,
            width * 4,
            color_space,
            K_CG_IMAGE_ALPHA_PREMULTIPLIED_FIRST | K_CG_BITMAP_BYTE_ORDER_32_LITTLE,
        );
        CGColorSpaceRelease(color_space);
        if ctx.is_null() {
            return;
        }

        CGContextSaveGState(ctx);
        CGContextSetShouldAntialias(ctx, true);
        CGContextSetShouldSmoothFonts(ctx, true);
        CGContextSetTextMatrix(ctx, CGAffineTransform::IDENTITY);

        for gr in runs {
            draw_one_glyph_run(ctx, gr, height as f64);
        }

        CGContextRestoreGState(ctx);
        CGContextRelease(ctx);
    }

    fn collect_coretext_runs(
        instructions: &[PaintInstruction],
    ) -> Vec<&PaintGlyphRun> {
        let mut out: Vec<&PaintGlyphRun> = Vec::new();
        fn walk<'a>(
            ins: &'a [PaintInstruction],
            out: &mut Vec<&'a PaintGlyphRun>,
        ) {
            for i in ins {
                match i {
                    PaintInstruction::GlyphRun(g) => {
                        if g.font_ref.starts_with("coretext:") {
                            out.push(g);
                        }
                    }
                    PaintInstruction::Group(grp) => walk(&grp.children, out),
                    PaintInstruction::Clip(c) => walk(&c.children, out),
                    PaintInstruction::Layer(l) => walk(&l.children, out),
                    _ => {}
                }
            }
        }
        walk(instructions, &mut out);
        out
    }

    unsafe fn draw_one_glyph_run(ctx: CGContextRef, run: &PaintGlyphRun, image_height: f64) {
        let (ps_name, size_from_ref) = parse_coretext_font_ref(&run.font_ref);
        let size = size_from_ref.unwrap_or(run.font_size);

        let cf_name = match cfstring_checked(&ps_name) {
            Some(s) => s,
            None => return,
        };
        let font: Id = CTFontCreateWithName(cf_name, size, std::ptr::null());
        CFRelease(cf_name);
        if font == NIL {
            return;
        }

        let (r, g, b, a) = parse_css_color(run.fill.as_deref().unwrap_or("rgb(0, 0, 0)"));
        CGContextSetRGBFillColor(ctx, r, g, b, a);

        let glyph_ids: Vec<u16> = run.glyphs.iter().map(|g| g.glyph_id as u16).collect();
        let positions: Vec<CGPoint> = run
            .glyphs
            .iter()
            .map(|g| CGPoint {
                x: g.x,
                y: image_height - g.y,
            })
            .collect();

        if !glyph_ids.is_empty() {
            CTFontDrawGlyphs(
                font,
                glyph_ids.as_ptr(),
                positions.as_ptr(),
                glyph_ids.len(),
                ctx,
            );
        }
        CFRelease(font);
    }

    /// Parse `"coretext:PSName@Size"` into `(PSName, Some(size))`.
    fn parse_coretext_font_ref(s: &str) -> (String, Option<f64>) {
        let rest = s.strip_prefix("coretext:").unwrap_or(s);
        if let Some(at_idx) = rest.rfind('@') {
            let name = &rest[..at_idx];
            let size_str = &rest[at_idx + 1..];
            let size = size_str.parse::<f64>().ok();
            return (name.to_string(), size);
        }
        (rest.to_string(), None)
    }

    /// Parse a subset of CSS colours into (r, g, b, a) in 0..=1.
    fn parse_css_color(s: &str) -> (f64, f64, f64, f64) {
        let s = s.trim();
        let (inner, has_alpha) = if let Some(i) = s.strip_prefix("rgba(").and_then(|x| x.strip_suffix(")")) {
            (i, true)
        } else if let Some(i) = s.strip_prefix("rgb(").and_then(|x| x.strip_suffix(")")) {
            (i, false)
        } else {
            return (0.0, 0.0, 0.0, 1.0);
        };
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() < 3 {
            return (0.0, 0.0, 0.0, 1.0);
        }
        let r = parts[0].parse::<f64>().unwrap_or(0.0) / 255.0;
        let g = parts[1].parse::<f64>().unwrap_or(0.0) / 255.0;
        let b = parts[2].parse::<f64>().unwrap_or(0.0) / 255.0;
        let a = if has_alpha && parts.len() >= 4 {
            parts[3].parse::<f64>().unwrap_or(1.0)
        } else {
            1.0
        };
        (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), a.clamp(0.0, 1.0))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parse_coretext_font_ref_full() {
            let (name, size) = parse_coretext_font_ref("coretext:Helvetica-Bold@16.0");
            assert_eq!(name, "Helvetica-Bold");
            assert_eq!(size, Some(16.0));
        }

        #[test]
        fn parse_coretext_font_ref_malformed_no_at() {
            let (name, size) = parse_coretext_font_ref("coretext:Helvetica-Bold");
            assert_eq!(name, "Helvetica-Bold");
            assert_eq!(size, None);
        }

        #[test]
        fn parse_coretext_font_ref_non_numeric_size() {
            let (name, size) = parse_coretext_font_ref("coretext:Helvetica@abc");
            assert_eq!(name, "Helvetica");
            assert_eq!(size, None);
        }

        #[test]
        fn parse_css_color_rgb() {
            let (r, g, b, a) = parse_css_color("rgb(255, 128, 0)");
            assert!((r - 1.0).abs() < 1e-6);
            assert!((g - 128.0 / 255.0).abs() < 1e-6);
            assert!((b - 0.0).abs() < 1e-6);
            assert_eq!(a, 1.0);
        }

        #[test]
        fn parse_css_color_rgba() {
            let (_r, _g, _b, a) = parse_css_color("rgba(0, 0, 0, 0.5)");
            assert!((a - 0.5).abs() < 1e-6);
        }

        #[test]
        fn parse_css_color_malformed_returns_black() {
            let (r, g, b, a) = parse_css_color("not-a-color");
            assert_eq!((r, g, b, a), (0.0, 0.0, 0.0, 1.0));
        }
    }
}

// ---------------------------------------------------------------------------
// Live-drawable present (Apple only)
// ---------------------------------------------------------------------------

#[cfg(target_vendor = "apple")]
pub fn render_to_metal_layer(
    scene: &PaintScene,
    metal_layer: objc_bridge::Id,
) -> Result<(), PaintMetalError> {
    let pixels = render(scene);
    unsafe { live_present::present_pixels_to_layer(metal_layer, &pixels) }
}

/// Errors from the live-drawable render path.
#[cfg(target_vendor = "apple")]
#[derive(Debug, Clone)]
pub enum PaintMetalError {
    NoDrawableAvailable,
    LayerMissingDevice,
    CommandBufferCreationFailed,
}

#[cfg(target_vendor = "apple")]
impl std::fmt::Display for PaintMetalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDrawableAvailable => write!(f, "CAMetalLayer had no current drawable"),
            Self::LayerMissingDevice => write!(f, "CAMetalLayer had no MTLDevice"),
            Self::CommandBufferCreationFailed => {
                write!(f, "MTLCommandQueue.commandBuffer returned nil")
            }
        }
    }
}

#[cfg(target_vendor = "apple")]
impl std::error::Error for PaintMetalError {}

#[cfg(target_vendor = "apple")]
mod live_present {
    use objc_bridge::{
        msg, MTLOrigin, MTLRegion, MTLSize,
        Id, NIL,
    };
    use paint_instructions::PixelContainer;

    use super::PaintMetalError;

    pub(super) unsafe fn present_pixels_to_layer(
        layer: Id,
        pixels: &PixelContainer,
    ) -> Result<(), PaintMetalError> {
        if layer == NIL {
            return Err(PaintMetalError::LayerMissingDevice);
        }
        let drawable: Id = msg!(layer, "nextDrawable");
        if drawable == NIL {
            return Err(PaintMetalError::NoDrawableAvailable);
        }

        let texture: Id = msg!(drawable, "texture");
        if texture == NIL {
            return Err(PaintMetalError::NoDrawableAvailable);
        }

        let w = pixels.width as usize;
        let h = pixels.height as usize;
        if w == 0 || h == 0 {
            return Ok(());
        }

        let mut bgra = pixels.data.clone();
        let stride = w * 4;
        for row in 0..h {
            for col in 0..w {
                let base = row * stride + col * 4;
                bgra.swap(base, base + 2); // swap R and B
            }
        }

        let region = MTLRegion {
            origin: MTLOrigin { x: 0, y: 0, z: 0 },
            size: MTLSize {
                width: w as u64,
                height: h as u64,
                depth: 1,
            },
        };

        use objc_bridge::objc_msgSend;
        let replace_fn: unsafe extern "C" fn(
            Id,
            objc_bridge::Sel,
            MTLRegion,
            usize,
            *const std::ffi::c_void,
            usize,
        ) = std::mem::transmute(objc_msgSend as *const ());
        replace_fn(
            texture,
            objc_bridge::sel("replaceRegion:mipmapLevel:withBytes:bytesPerRow:"),
            region,
            0,
            bgra.as_ptr() as *const _,
            stride,
        );

        let device: Id = msg!(layer, "device");
        if device == NIL {
            return Err(PaintMetalError::LayerMissingDevice);
        }
        let queue: Id = msg!(device, "newCommandQueue");
        if queue == NIL {
            return Err(PaintMetalError::CommandBufferCreationFailed);
        }
        let cmd_buffer: Id = msg!(queue, "commandBuffer");
        if cmd_buffer == NIL {
            objc_bridge::release(queue);
            return Err(PaintMetalError::CommandBufferCreationFailed);
        }
        let _: Id = msg!(cmd_buffer, "presentDrawable:", drawable);
        let _: Id = msg!(cmd_buffer, "commit");
        objc_bridge::release(queue);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, target_vendor = "apple"))]
mod tests {
    use super::*;
    use paint_instructions::{
        PaintBase, PaintEllipse, PaintInstruction, PaintPath, PaintRect, PaintScene,
        PathCommand,
    };

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.2.0");
    }

    // ─── Color parser tests ──────────────────────────────────────────────────

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
        // 0x80 = 128 → 128/255 ≈ 0.502
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

    // ─── Vertex generation tests ─────────────────────────────────────────────

    #[test]
    fn rect_generates_6_vertices() {
        let rect = PaintInstruction::Rect(PaintRect::filled(10.0, 20.0, 30.0, 40.0, "#ff0000"));
        let mut positions = Vec::new();
        let mut colors = Vec::new();
        collect_geometry(&[rect], &mut positions, &mut colors, 0);
        // 6 vertices × 2 floats (x, y) each = 12 position floats
        assert_eq!(positions.len(), 12);
        // 6 vertices × 4 floats (r, g, b, a) each = 24 color floats
        assert_eq!(colors.len(), 24);
    }

    #[test]
    fn transparent_rect_generates_no_vertices() {
        let rect = PaintInstruction::Rect(PaintRect::filled(0.0, 0.0, 50.0, 50.0, "transparent"));
        let mut positions = Vec::new();
        let mut colors = Vec::new();
        collect_geometry(&[rect], &mut positions, &mut colors, 0);
        assert!(positions.is_empty(), "transparent rect should produce no vertices");
    }

    #[test]
    fn group_recurses_into_children() {
        use paint_instructions::PaintGroup;
        let group = PaintInstruction::Group(PaintGroup {
            base: PaintBase::default(),
            children: vec![
                PaintInstruction::Rect(PaintRect::filled(0.0, 0.0, 10.0, 10.0, "#ff0000")),
                PaintInstruction::Rect(PaintRect::filled(10.0, 0.0, 10.0, 10.0, "#00ff00")),
            ],
            transform: None,
            opacity: None,
        });
        let mut positions = Vec::new();
        let mut colors = Vec::new();
        collect_geometry(&[group], &mut positions, &mut colors, 0);
        // 2 rects × 6 vertices × 2 floats = 24 positions
        assert_eq!(positions.len(), 24);
    }

    #[test]
    fn ellipse_fill_generates_correct_vertex_count() {
        // A filled ellipse with no stroke: ELLIPSE_SEGMENTS triangles × 3 vertices × 2 floats
        let ellipse = PaintInstruction::Ellipse(PaintEllipse {
            base: PaintBase::default(),
            cx: 50.0,
            cy: 50.0,
            rx: 30.0,
            ry: 20.0,
            fill: Some("#0000ff".to_string()),
            stroke: None,
            stroke_width: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        });
        let mut positions = Vec::new();
        let mut colors = Vec::new();
        collect_geometry(&[ellipse], &mut positions, &mut colors, 0);
        let expected = ELLIPSE_SEGMENTS * 3 * 2; // 64 triangles × 3 verts × 2 floats
        assert_eq!(positions.len(), expected, "ellipse fill vertex count");
    }

    #[test]
    fn ellipse_stroke_generates_ring_vertices() {
        // Fill + stroke: fill (64 tris) + stroke ring (64 quads = 128 tris)
        let ellipse = PaintInstruction::Ellipse(PaintEllipse {
            base: PaintBase::default(),
            cx: 50.0,
            cy: 50.0,
            rx: 30.0,
            ry: 20.0,
            fill: Some("#0000ff".to_string()),
            stroke: Some("#ff0000".to_string()),
            stroke_width: Some(2.0),
            stroke_dash: None,
            stroke_dash_offset: None,
        });
        let mut positions = Vec::new();
        let mut colors = Vec::new();
        collect_geometry(&[ellipse], &mut positions, &mut colors, 0);
        // fill: 64 * 3 verts, stroke ring: 64 quads * 2 tris * 3 verts = 384
        let expected = (ELLIPSE_SEGMENTS * 3 + ELLIPSE_SEGMENTS * 2 * 3) * 2; // × 2 for x,y
        assert_eq!(positions.len(), expected, "ellipse fill+stroke vertex count");
    }

    #[test]
    fn diamond_path_fill_generates_vertices() {
        // A diamond is 4-point closed polygon (5 points including close)
        let diamond = PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 50.0, y: 10.0 },  // top
                PathCommand::LineTo { x: 90.0, y: 50.0 },  // right
                PathCommand::LineTo { x: 50.0, y: 90.0 },  // bottom
                PathCommand::LineTo { x: 10.0, y: 50.0 },  // left
                PathCommand::Close,
            ],
            fill: Some("#ffff00".to_string()),
            fill_rule: None,
            stroke: None,
            stroke_width: None,
            stroke_cap: None,
            stroke_join: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        });
        let mut positions = Vec::new();
        let mut colors = Vec::new();
        collect_geometry(&[diamond], &mut positions, &mut colors, 0);
        // Subpath has 5 points (top, right, bottom, left, top-again from close).
        // Fan: pivot=pts[0], triangles for i in 1..4 → 3 triangles
        // Each triangle: 3 vertices × 2 floats = 6 floats → 18 total
        assert!(positions.len() >= 18, "diamond fill should have at least 3 triangles");
    }

    #[test]
    fn paint_text_silently_ignored() {
        // PaintText is Canvas/SVG/DOM-only — Metal ignores it entirely.
        let text_instr = PaintInstruction::Text(paint_instructions::PaintText {
            base: PaintBase::default(),
            x: 50.0,
            y: 50.0,
            text: "Hello".to_string(),
            font_ref: None,
            font_size: 14.0,
            fill: Some("#000000".to_string()),
            text_align: None,
        });
        let mut positions = Vec::new();
        let mut colors = Vec::new();
        collect_geometry(&[text_instr], &mut positions, &mut colors, 0);
        assert!(positions.is_empty(), "PaintText should not generate triangle vertices");
        assert!(colors.is_empty(), "PaintText should not generate color vertices");
    }

    #[test]
    fn empty_scene_returns_empty_pixel_container() {
        let scene = PaintScene::new(0.0, 0.0);
        let pixels = render(&scene);
        assert_eq!(pixels.width, 0);
        assert_eq!(pixels.height, 0);
        assert!(pixels.data.is_empty());
    }

    /// Render a scene with a red rectangle on a white background.
    #[test]
    fn render_red_rect_on_white() {
        let mut scene = PaintScene::new(100.0, 100.0);
        scene.instructions.push(PaintInstruction::Rect(
            PaintRect::filled(10.0, 10.0, 80.0, 80.0, "#ff0000"),
        ));

        let pixels = render(&scene);
        assert_eq!(pixels.width, 100);
        assert_eq!(pixels.height, 100);

        // Centre of the red rectangle should be red
        let (r, g, b, a) = pixels.pixel_at(50, 50);
        assert_eq!(r, 255, "red channel at centre");
        assert_eq!(g, 0,   "green channel at centre");
        assert_eq!(b, 0,   "blue channel at centre");
        assert_eq!(a, 255, "alpha at centre");

        // Top-left corner is outside the rect → white background
        let (r, g, b, a) = pixels.pixel_at(2, 2);
        assert_eq!(r, 255, "red channel at corner (background)");
        assert_eq!(g, 255, "green channel at corner (background)");
        assert_eq!(b, 255, "blue channel at corner (background)");
        assert_eq!(a, 255, "alpha at corner (background)");
    }

    /// Render a blue filled ellipse and verify the centre pixel is blue.
    #[test]
    fn render_blue_ellipse() {
        let mut scene = PaintScene::new(100.0, 100.0);
        scene.instructions.push(PaintInstruction::Ellipse(PaintEllipse {
            base: PaintBase::default(),
            cx: 50.0,
            cy: 50.0,
            rx: 30.0,
            ry: 30.0,
            fill: Some("#0000ff".to_string()),
            stroke: None,
            stroke_width: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let pixels = render(&scene);

        // Centre of the ellipse should be blue
        let (r, g, b, _a) = pixels.pixel_at(50, 50);
        assert_eq!(r, 0,   "red channel at ellipse centre should be 0");
        assert_eq!(g, 0,   "green channel at ellipse centre should be 0");
        assert_eq!(b, 255, "blue channel at ellipse centre should be 255");

        // Pixel well outside the ellipse should be white background
        let (r, g, b, _a) = pixels.pixel_at(2, 2);
        assert_eq!(r, 255, "corner should be background white");
        assert_eq!(g, 255, "corner should be background white");
        assert_eq!(b, 255, "corner should be background white");
    }

    /// Render a yellow diamond (PaintPath) and verify the centre is yellow.
    #[test]
    fn render_yellow_diamond() {
        let mut scene = PaintScene::new(100.0, 100.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 50.0, y: 10.0 },  // top
                PathCommand::LineTo { x: 90.0, y: 50.0 },  // right
                PathCommand::LineTo { x: 50.0, y: 90.0 },  // bottom
                PathCommand::LineTo { x: 10.0, y: 50.0 },  // left
                PathCommand::Close,
            ],
            fill: Some("#ffff00".to_string()),
            fill_rule: None,
            stroke: None,
            stroke_width: None,
            stroke_cap: None,
            stroke_join: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let pixels = render(&scene);

        // Centre (50, 50) should be inside the diamond → yellow
        let (r, g, b, _a) = pixels.pixel_at(50, 50);
        assert_eq!(r, 255, "yellow: r=255 at diamond centre");
        assert_eq!(g, 255, "yellow: g=255 at diamond centre");
        assert_eq!(b, 0,   "yellow: b=0 at diamond centre");

        // Corner (2, 2) is well outside the diamond → white
        let (r, g, b, _a) = pixels.pixel_at(2, 2);
        assert_eq!(r, 255, "background white at corner");
        assert_eq!(g, 255, "background white at corner");
        assert_eq!(b, 255, "background white at corner");
    }

    /// Render a dark module grid pattern (like a QR code quiet zone).
    #[test]
    fn render_black_modules_on_white() {
        let module_size = 4.0_f64;
        let mut scene = PaintScene::new(40.0, 40.0);

        for row in 0..4u32 {
            for col in 0..4u32 {
                if (row + col) % 2 == 0 {
                    scene.instructions.push(PaintInstruction::Rect(PaintRect::filled(
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

        // Top-left module (0,0) is black → pixel (2, 2) should be black
        let (r, g, b, _a) = pixels.pixel_at(2, 2);
        assert_eq!(r, 0, "black module should have r=0");
        assert_eq!(g, 0, "black module should have g=0");
        assert_eq!(b, 0, "black module should have b=0");
    }
}
