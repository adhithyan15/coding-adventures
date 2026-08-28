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
//! | `PaintLayer`      | Flattened normal layers; filtered/blended isolation planned |
//! | `PaintGradient`   | Shared GPU-plan texture ramps                               |
//! | `PaintImage`      | Ordered GPU textures with transforms, clips, and opacity    |
//!
//! ## Metal pipeline
//!
//! ```text
//! PaintScene
//!   │
//!   ├── 1. Create Metal device (MTLCreateSystemDefaultDevice)
//!   ├── 2. Create offscreen RGBA8 texture (width × height)
//!   ├── 3. Lower through paint-vm-gpu-core's ordered render plan
//!   ├── 4. Upload mesh and image/gradient textures
//!   ├── 5. Rasterize CoreText glyph runs into transient ordered textures
//!   ├── 6. Encode ordered mesh, image, glyph, and scissor commands
//!   ├── 7. Commit and wait for GPU completion
//!   └── 8. Read back RGBA8 pixels → PixelContainer
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

// Platform-conditional: code for the non-native platform is intentionally inactive; allow the resulting dead_code/unused lints only where it does not compile in.
#![cfg_attr(not(target_vendor = "apple"), allow(dead_code, unused_imports))]

pub const VERSION: &str = "0.7.0";

pub use paint_instructions::PixelContainer;
pub use paint_vm_gpu_core::GpuImageResolver;

/// Describe the shared GPU-plan features executed by this backend.
pub const fn profile() -> paint_vm_gpu_core::GpuBackendProfile {
    paint_vm_gpu_core::GpuBackendProfile::tier1_textured(
        "paint-metal",
        paint_vm_gpu_core::GpuApiFamily::Metal,
        paint_vm_gpu_core::GpuRenderPath::GraphicsPipeline,
        "MSL",
        paint_vm_gpu_core::GpuReadbackStrategy::TextureCopyToBuffer,
    )
    .with_isolated_layers()
}

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
use paint_instructions::PaintScene;
#[cfg(test)]
use paint_instructions::{
    PaintEllipse, PaintInstruction, PaintLine, PaintPath, PaintRect, PathCommand,
};
#[cfg(target_vendor = "apple")]
use paint_vm_gpu_core::{
    plan_scene, plan_scene_with_image_resolver, GpuBlendMode, GpuColor, GpuCommand, GpuFilter,
    GpuImageUpload, GpuLayer, GpuMesh, GpuPaintPlan, GpuPoint, GpuRect, GpuTextureFilter,
    GpuVertex,
};
#[allow(unused_imports)]
use std::ffi::{c_int, c_ulong, CStr};
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

/// MSL shader source for ordered solid and textured triangles.
///
/// ## Data flow
///
/// ```text
/// CPU → vertex buffer:       [position(float2), uv(float2), color(float4)]
/// GPU vertex shader:         pixel_coords → NDC, pass UV/color through
/// GPU rasteriser:            interpolates (position, color) across triangle
/// GPU fragment shader:       samples image/gradient texture × vertex color
/// ```
const TEXTURED_SHADER_SOURCE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct PaintVertexOut {
    float4 position [[position]];
    float2 uv;
    float4 color;
};

vertex PaintVertexOut paint_vertex(
    uint vid [[vertex_id]],
    const device float2* positions [[buffer(0)]],
    const device float2* uvs       [[buffer(1)]],
    const device float4* colors    [[buffer(2)]],
    constant float2& viewport      [[buffer(3)]]
) {
    PaintVertexOut out;
    float2 px = positions[vid];
    out.position = float4(
        (px.x / viewport.x) * 2.0 - 1.0,
        1.0 - (px.y / viewport.y) * 2.0,
        0.0,
        1.0
    );
    out.uv = uvs[vid];
    out.color = colors[vid];
    return out;
}

fragment float4 paint_fragment(
    PaintVertexOut in [[stage_in]],
    texture2d<float> image [[texture(0)]],
    sampler image_sampler [[sampler(0)]]
) {
    return image.sample(image_sampler, in.uv) * in.color;
}
"#;

/// Compute kernels for ordered layer filters and destination-aware blending.
///
/// Render targets store premultiplied RGBA. Color filters temporarily
/// unpremultiply, while blur and compositing stay in premultiplied space.
#[cfg(target_vendor = "apple")]
const LAYER_SHADER_SOURCE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct FilterParams {
    uint kind;
    uint padding0;
    uint padding1;
    uint padding2;
    float4 params;
    float4 color;
    float4 matrix0;
    float4 matrix1;
    float4 matrix2;
    float4 matrix3;
    float4 bias;
};

struct CompositeParams {
    uint blend_mode;
    uint padding0;
    uint padding1;
    uint padding2;
    float opacity;
    float clip_x;
    float clip_y;
    float clip_width;
    float clip_height;
};

float4 straight_color(float4 value) {
    return value.a > 0.000001 ? float4(value.rgb / value.a, value.a) : float4(0.0);
}

float4 premultiplied_color(float4 value) {
    return float4(value.rgb * value.a, value.a);
}

float3 hue_rotated(float3 color, float degrees) {
    float angle = degrees * (3.14159265358979323846f / 180.0f);
    float c = cos(angle);
    float s = sin(angle);
    return float3(
        dot(color, float3(0.213 + c * 0.787 - s * 0.213,
                          0.715 - c * 0.715 - s * 0.715,
                          0.072 - c * 0.072 + s * 0.928)),
        dot(color, float3(0.213 - c * 0.213 + s * 0.143,
                          0.715 + c * 0.285 + s * 0.140,
                          0.072 - c * 0.072 - s * 0.283)),
        dot(color, float3(0.213 - c * 0.213 - s * 0.787,
                          0.715 - c * 0.715 + s * 0.715,
                          0.072 + c * 0.928 + s * 0.072))
    );
}

kernel void paint_filter(
    texture2d<float, access::read> source [[texture(0)]],
    texture2d<float, access::write> destination [[texture(1)]],
    constant FilterParams& filter [[buffer(0)]],
    uint2 gid [[thread_position_in_grid]]
) {
    uint width = source.get_width();
    uint height = source.get_height();
    if (gid.x >= width || gid.y >= height) return;

    if (filter.kind == 0) {
        int radius = min(int(round(filter.params.x)), 32);
        float4 total = float4(0.0);
        uint count = 0;
        for (int y = -radius; y <= radius; ++y) {
            for (int x = -radius; x <= radius; ++x) {
                int2 sample_position = int2(gid) + int2(x, y);
                if (sample_position.x >= 0 && sample_position.y >= 0 &&
                    sample_position.x < int(width) && sample_position.y < int(height)) {
                    total += source.read(uint2(sample_position));
                }
                count += 1;
            }
        }
        destination.write(total / max(float(count), 1.0), gid);
        return;
    }

    float4 input = source.read(gid);
    if (filter.kind == 1) {
        int radius = min(int(round(filter.params.w)), 32);
        int2 center = int2(gid) - int2(round(filter.params.xy));
        float alpha = 0.0;
        uint count = 0;
        for (int y = -radius; y <= radius; ++y) {
            for (int x = -radius; x <= radius; ++x) {
                int2 sample_position = center + int2(x, y);
                if (sample_position.x >= 0 && sample_position.y >= 0 &&
                    sample_position.x < int(width) && sample_position.y < int(height)) {
                    alpha += source.read(uint2(sample_position)).a;
                }
                count += 1;
            }
        }
        alpha = alpha / max(float(count), 1.0) * filter.color.a;
        float4 shadow = float4(filter.color.rgb * alpha, alpha);
        destination.write(input + shadow * (1.0 - input.a), gid);
        return;
    }

    float4 straight = straight_color(input);
    if (filter.kind == 2) {
        float4 channels = straight;
        straight = float4(
            dot(filter.matrix0, channels) + filter.bias.x,
            dot(filter.matrix1, channels) + filter.bias.y,
            dot(filter.matrix2, channels) + filter.bias.z,
            dot(filter.matrix3, channels) + filter.bias.w
        );
    } else if (filter.kind == 3) {
        straight.rgb *= filter.params.x;
    } else if (filter.kind == 4) {
        straight.rgb = (straight.rgb - 0.5) * filter.params.x + 0.5;
    } else if (filter.kind == 5) {
        float luminance = dot(straight.rgb, float3(0.2126, 0.7152, 0.0722));
        straight.rgb = mix(float3(luminance), straight.rgb, filter.params.x);
    } else if (filter.kind == 6) {
        straight.rgb = hue_rotated(straight.rgb, filter.params.x);
    } else if (filter.kind == 7) {
        straight.rgb = mix(straight.rgb, 1.0 - straight.rgb, filter.params.x);
    } else if (filter.kind == 8) {
        straight.a *= filter.params.x;
    }
    destination.write(premultiplied_color(clamp(straight, 0.0, 1.0)), gid);
}

float luminance(float3 color) {
    return dot(color, float3(0.3, 0.59, 0.11));
}

float saturation(float3 color) {
    return max(color.r, max(color.g, color.b)) - min(color.r, min(color.g, color.b));
}

float3 clip_color(float3 color) {
    float l = luminance(color);
    float minimum = min(color.r, min(color.g, color.b));
    float maximum = max(color.r, max(color.g, color.b));
    if (minimum < 0.0) color = l + ((color - l) * l) / max(l - minimum, 0.000001);
    if (maximum > 1.0) color = l + ((color - l) * (1.0 - l)) / max(maximum - l, 0.000001);
    return color;
}

float3 set_luminance(float3 color, float value) {
    return clip_color(color + (value - luminance(color)));
}

float3 set_saturation(float3 color, float value) {
    float minimum = min(color.r, min(color.g, color.b));
    float maximum = max(color.r, max(color.g, color.b));
    if (maximum <= minimum) return float3(0.0);
    return (color - minimum) * (value / (maximum - minimum));
}

float soft_light(float backdrop, float source) {
    if (source <= 0.5) return backdrop - (1.0 - 2.0 * source) * backdrop * (1.0 - backdrop);
    float d = backdrop <= 0.25
        ? ((16.0 * backdrop - 12.0) * backdrop + 4.0) * backdrop
        : sqrt(backdrop);
    return backdrop + (2.0 * source - 1.0) * (d - backdrop);
}

float3 blend_rgb(uint mode, float3 backdrop, float3 source) {
    if (mode == 1) return backdrop * source;
    if (mode == 2) return backdrop + source - backdrop * source;
    if (mode == 3) return select(2.0 * backdrop * source,
                                 1.0 - 2.0 * (1.0 - backdrop) * (1.0 - source),
                                 backdrop >= 0.5);
    if (mode == 4) return min(backdrop, source);
    if (mode == 5) return max(backdrop, source);
    if (mode == 6) return select(backdrop / max(1.0 - source, 0.000001),
                                 float3(1.0), source >= 1.0);
    if (mode == 7) return select(1.0 - (1.0 - backdrop) / max(source, 0.000001),
                                 float3(0.0), source <= 0.0);
    if (mode == 8) return select(2.0 * backdrop * source,
                                 1.0 - 2.0 * (1.0 - backdrop) * (1.0 - source),
                                 source >= 0.5);
    if (mode == 9) return float3(soft_light(backdrop.r, source.r),
                                 soft_light(backdrop.g, source.g),
                                 soft_light(backdrop.b, source.b));
    if (mode == 10) return abs(backdrop - source);
    if (mode == 11) return backdrop + source - 2.0 * backdrop * source;
    if (mode == 12) return set_luminance(set_saturation(source, saturation(backdrop)), luminance(backdrop));
    if (mode == 13) return set_luminance(set_saturation(backdrop, saturation(source)), luminance(backdrop));
    if (mode == 14) return set_luminance(source, luminance(backdrop));
    if (mode == 15) return set_luminance(backdrop, luminance(source));
    return source;
}

kernel void paint_composite(
    texture2d<float, access::read> source [[texture(0)]],
    texture2d<float, access::read> backdrop [[texture(1)]],
    texture2d<float, access::write> destination [[texture(2)]],
    constant CompositeParams& params [[buffer(0)]],
    uint2 gid [[thread_position_in_grid]]
) {
    if (gid.x >= source.get_width() || gid.y >= source.get_height()) return;
    float4 source_pixel = source.read(gid);
    float4 destination_pixel = backdrop.read(gid);
    if (float(gid.x) < params.clip_x || float(gid.y) < params.clip_y ||
        float(gid.x) >= params.clip_x + params.clip_width ||
        float(gid.y) >= params.clip_y + params.clip_height) {
        destination.write(destination_pixel, gid);
        return;
    }
    float source_alpha = clamp(source_pixel.a * params.opacity, 0.0, 1.0);
    float destination_alpha = destination_pixel.a;
    float3 source_color = source_pixel.a > 0.000001 ? source_pixel.rgb / source_pixel.a : float3(0.0);
    float3 destination_color = destination_alpha > 0.000001
        ? destination_pixel.rgb / destination_alpha
        : float3(0.0);
    float3 blended = clamp(blend_rgb(params.blend_mode, destination_color, source_color), 0.0, 1.0);
    float3 result = (1.0 - source_alpha) * destination_pixel.rgb
        + (1.0 - destination_alpha) * source_color * source_alpha
        + source_alpha * destination_alpha * blended;
    float alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    destination.write(float4(clamp(result, 0.0, 1.0), alpha), gid);
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
#[cfg(test)]
fn parse_hex_color(s: &str) -> (f64, f64, f64, f64) {
    let s = s.trim();
    if s == "transparent" {
        return (0.0, 0.0, 0.0, 0.0);
    }
    if let Some(hex) = css_named_color(s) {
        return parse_hex_color(hex);
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

#[cfg(test)]
fn css_named_color(name: &str) -> Option<&'static str> {
    Some(match name.to_ascii_lowercase().as_str() {
        "aqua" => "#00ffff",
        "black" => "#000000",
        "blue" => "#0000ff",
        "fuchsia" => "#ff00ff",
        "gray" | "grey" => "#808080",
        "green" => "#008000",
        "lime" => "#00ff00",
        "maroon" => "#800000",
        "navy" => "#000080",
        "olive" => "#808000",
        "orange" => "#ffa500",
        "purple" => "#800080",
        "red" => "#ff0000",
        "silver" => "#c0c0c0",
        "teal" => "#008080",
        "white" => "#ffffff",
        "yellow" => "#ffff00",
        _ => return None,
    })
}

/// Parse the comma-separated r,g,b(,a) components from inside an
/// `rgb(...)` / `rgba(...)` CSS string. r/g/b are 0..=255 decimal, a
/// is 0..=1 decimal. Missing / malformed components clamp gracefully
/// toward opaque black.
#[cfg(test)]
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
/// - Layer and Gradient are deferred to P2D08; Image is composited after readback.
///
/// `depth` must be 0 on the initial call; it is incremented for each recursive Group/Clip.
#[cfg(test)]
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
#[cfg(test)]
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
#[allow(clippy::too_many_arguments)] // geometry + color + buffers; signature kept as-is
#[cfg(test)]
fn emit_filled_rect(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
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
#[cfg(test)]
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

    let p0x = x1 + nx;
    let p0y = y1 + ny;
    let p1x = x1 - nx;
    let p1y = y1 - ny;
    let p2x = x2 + nx;
    let p2y = y2 + ny;
    let p3x = x2 - nx;
    let p3y = y2 - ny;

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
#[cfg(test)]
const ELLIPSE_SEGMENTS: usize = 64;

#[cfg(test)]
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
        pts.push((cx + rx * angle.cos() as f32, cy + ry * angle.sin() as f32));
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

#[cfg(test)]
fn add_path_stroke_quad(
    start: (f32, f32),
    end: (f32, f32),
    half_width: f32,
    color: (f32, f32, f32, f32),
    positions: &mut Vec<f32>,
    colors: &mut Vec<f32>,
) {
    let (x1, y1) = start;
    let (x2, y2) = end;
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 {
        return;
    }
    let nx = -dy / len * half_width;
    let ny = dx / len * half_width;
    let (ax, ay) = (x1 + nx, y1 + ny);
    let (bx, by) = (x1 - nx, y1 - ny);
    let (cx, cy) = (x2 + nx, y2 + ny);
    let (dx, dy) = (x2 - nx, y2 - ny);
    let (r, g, b, a) = color;
    positions.extend_from_slice(&[ax, ay, cx, cy, bx, by, cx, cy, dx, dy, bx, by]);
    for _ in 0..6 {
        colors.extend_from_slice(&[r, g, b, a]);
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
#[cfg(test)]
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
            PathCommand::QuadTo {
                cx: qcx,
                cy: qcy,
                x,
                y,
            } => {
                // De Casteljau — 8 linear segments
                let p0x = cx;
                let p0y = cy;
                let p1x = *qcx as f32;
                let p1y = *qcy as f32;
                let p2x = *x as f32;
                let p2y = *y as f32;
                for k in 1..=8u32 {
                    let t = k as f32 / 8.0;
                    let u = 1.0 - t;
                    let qx = u * u * p0x + 2.0 * u * t * p1x + t * t * p2x;
                    let qy = u * u * p0y + 2.0 * u * t * p1y + t * t * p2y;
                    current.push((qx, qy));
                }
                cx = p2x;
                cy = p2y;
            }
            PathCommand::CubicTo {
                cx1,
                cy1,
                cx2,
                cy2,
                x,
                y,
            } => {
                // De Casteljau — 8 linear segments
                let p0x = cx;
                let p0y = cy;
                let p1x = *cx1 as f32;
                let p1y = *cy1 as f32;
                let p2x = *cx2 as f32;
                let p2y = *cy2 as f32;
                let p3x = *x as f32;
                let p3y = *y as f32;
                for k in 1..=8u32 {
                    let t = k as f32 / 8.0;
                    let u = 1.0 - t;
                    let qx = u * u * u * p0x
                        + 3.0 * u * u * t * p1x
                        + 3.0 * u * t * t * p2x
                        + t * t * t * p3x;
                    let qy = u * u * u * p0y
                        + 3.0 * u * u * t * p1y
                        + 3.0 * u * t * t * p2y
                        + t * t * t * p3y;
                    current.push((qx, qy));
                }
                cx = p3x;
                cy = p3y;
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
            let mut dash_pattern: Vec<f32> = path
                .stroke_dash
                .as_deref()
                .unwrap_or_default()
                .iter()
                .copied()
                .filter(|length| *length > 0.0)
                .map(|length| length as f32)
                .collect();
            if dash_pattern.len() % 2 == 1 {
                dash_pattern.extend_from_within(..);
            }
            for pts in &subpaths {
                let mut dash_index = 0;
                let mut dash_remaining = dash_pattern.first().copied().unwrap_or(0.0);
                if !dash_pattern.is_empty() {
                    let cycle: f32 = dash_pattern.iter().sum();
                    let mut offset = path.stroke_dash_offset.unwrap_or(0.0) as f32 % cycle;
                    if offset < 0.0 {
                        offset += cycle;
                    }
                    while offset >= dash_remaining {
                        offset -= dash_remaining;
                        dash_index = (dash_index + 1) % dash_pattern.len();
                        dash_remaining = dash_pattern[dash_index];
                    }
                    dash_remaining -= offset;
                }
                for i in 0..pts.len().saturating_sub(1) {
                    let (x1, y1) = pts[i];
                    let (x2, y2) = pts[i + 1];
                    let dx = x2 - x1;
                    let dy = y2 - y1;
                    let len = (dx * dx + dy * dy).sqrt();
                    if len < 0.001 {
                        continue;
                    }
                    if dash_pattern.is_empty() {
                        add_path_stroke_quad(
                            (x1, y1),
                            (x2, y2),
                            half_sw,
                            (sr, sg, sb, sa),
                            positions,
                            colors,
                        );
                        continue;
                    }
                    let mut consumed = 0.0;
                    while consumed < len {
                        let step = dash_remaining.min(len - consumed);
                        if dash_index % 2 == 0 {
                            let start_t = consumed / len;
                            let end_t = (consumed + step) / len;
                            add_path_stroke_quad(
                                (x1 + dx * start_t, y1 + dy * start_t),
                                (x1 + dx * end_t, y1 + dy * end_t),
                                half_sw,
                                (sr, sg, sb, sa),
                                positions,
                                colors,
                            );
                        }
                        consumed += step;
                        dash_remaining -= step;
                        if dash_remaining <= f32::EPSILON {
                            dash_index = (dash_index + 1) % dash_pattern.len();
                            dash_remaining = dash_pattern[dash_index];
                        }
                    }
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
    validate_scene_dimensions(scene);
    unsafe { render_plan_unsafe(plan_scene(scene)) }
}

/// Render a scene while delegating URI-backed image loading to the host.
///
/// The resolver owns fetch, cache, security, and codec policy. Metal only
/// receives decoded RGBA pixels, matching the browser pipeline's resource
/// boundary and keeping network behavior out of the renderer.
#[cfg(target_vendor = "apple")]
pub fn render_with_image_resolver(
    scene: &PaintScene,
    resolver: &dyn GpuImageResolver,
) -> PixelContainer {
    validate_scene_dimensions(scene);
    unsafe { render_plan_unsafe(plan_scene_with_image_resolver(scene, resolver)) }
}

#[cfg(target_vendor = "apple")]
fn validate_scene_dimensions(scene: &PaintScene) {
    const MAX_DIMENSION: f64 = 16_384.0;
    if !scene.width.is_finite()
        || !scene.height.is_finite()
        || scene.width > MAX_DIMENSION
        || scene.height > MAX_DIMENSION
    {
        panic!(
            "Scene dimensions {}x{} are non-finite or exceed maximum {}x{}",
            scene.width, scene.height, MAX_DIMENSION, MAX_DIMENSION
        );
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod image_overlay {
    use paint_instructions::{ImageSrc, PaintImage, PaintInstruction, PaintScene, PixelContainer};

    const IDENTITY: [f64; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

    #[derive(Clone, Copy)]
    struct State {
        transform: [f64; 6],
        opacity: f64,
        clip: [f64; 4],
    }

    pub(super) fn overlay_decoded_images(scene: &PaintScene, output: &mut PixelContainer) {
        let state = State {
            transform: IDENTITY,
            opacity: 1.0,
            clip: [0.0, 0.0, scene.width, scene.height],
        };
        walk(&scene.instructions, state, output, 0);
    }

    fn walk(
        instructions: &[PaintInstruction],
        state: State,
        output: &mut PixelContainer,
        depth: usize,
    ) {
        if depth > 128 {
            return;
        }
        for instruction in instructions {
            match instruction {
                PaintInstruction::Image(image) => draw_image(image, state, output),
                PaintInstruction::Group(group) => walk(
                    &group.children,
                    State {
                        transform: compose(state.transform, group.transform.unwrap_or(IDENTITY)),
                        opacity: state.opacity * group.opacity.unwrap_or(1.0).clamp(0.0, 1.0),
                        ..state
                    },
                    output,
                    depth + 1,
                ),
                PaintInstruction::Layer(layer) => walk(
                    &layer.children,
                    State {
                        transform: compose(state.transform, layer.transform.unwrap_or(IDENTITY)),
                        opacity: state.opacity * layer.opacity.unwrap_or(1.0).clamp(0.0, 1.0),
                        ..state
                    },
                    output,
                    depth + 1,
                ),
                PaintInstruction::Clip(clip) => {
                    let corners = transformed_corners(
                        state.transform,
                        clip.x,
                        clip.y,
                        clip.width,
                        clip.height,
                    );
                    let bounds = intersect(state.clip, bounds(corners));
                    walk(
                        &clip.children,
                        State {
                            clip: bounds,
                            ..state
                        },
                        output,
                        depth + 1,
                    );
                }
                _ => {}
            }
        }
    }

    fn draw_image(image: &PaintImage, state: State, output: &mut PixelContainer) {
        let ImageSrc::Pixels(source) = &image.src else {
            return;
        };
        if source.width == 0 || source.height == 0 || image.width <= 0.0 || image.height <= 0.0 {
            return;
        }
        let Some(inverse) = inverse(state.transform) else {
            return;
        };
        let draw_bounds = intersect(
            state.clip,
            bounds(transformed_corners(
                state.transform,
                image.x,
                image.y,
                image.width,
                image.height,
            )),
        );
        let x0 = draw_bounds[0].floor().max(0.0) as u32;
        let y0 = draw_bounds[1].floor().max(0.0) as u32;
        let x1 = draw_bounds[2].ceil().min(output.width as f64) as u32;
        let y1 = draw_bounds[3].ceil().min(output.height as f64) as u32;
        let opacity = state.opacity * image.opacity.unwrap_or(1.0).clamp(0.0, 1.0);
        for y in y0..y1 {
            for x in x0..x1 {
                let [local_x, local_y] = point(inverse, x as f64 + 0.5, y as f64 + 0.5);
                if local_x < image.x
                    || local_y < image.y
                    || local_x >= image.x + image.width
                    || local_y >= image.y + image.height
                {
                    continue;
                }
                let u = (local_x - image.x) / image.width;
                let v = (local_y - image.y) / image.height;
                let sx = (u * source.width as f64)
                    .floor()
                    .min(source.width as f64 - 1.0) as u32;
                let sy = (v * source.height as f64)
                    .floor()
                    .min(source.height as f64 - 1.0) as u32;
                blend(output, x, y, source.pixel_at(sx, sy), opacity);
            }
        }
    }

    fn blend(output: &mut PixelContainer, x: u32, y: u32, source: (u8, u8, u8, u8), opacity: f64) {
        let offset = ((y * output.width + x) * 4) as usize;
        let source_alpha = source.3 as f64 / 255.0 * opacity;
        let destination_alpha = output.data[offset + 3] as f64 / 255.0;
        let out_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
        if out_alpha <= f64::EPSILON {
            output.data[offset..offset + 4].fill(0);
            return;
        }
        for (index, channel) in [source.0, source.1, source.2].into_iter().enumerate() {
            let source_value = channel as f64 / 255.0;
            let destination_value = output.data[offset + index] as f64 / 255.0;
            output.data[offset + index] = (((source_value * source_alpha
                + destination_value * destination_alpha * (1.0 - source_alpha))
                / out_alpha)
                * 255.0)
                .round() as u8;
        }
        output.data[offset + 3] = (out_alpha * 255.0).round() as u8;
    }

    fn compose(parent: [f64; 6], local: [f64; 6]) -> [f64; 6] {
        [
            parent[0] * local[0] + parent[2] * local[1],
            parent[1] * local[0] + parent[3] * local[1],
            parent[0] * local[2] + parent[2] * local[3],
            parent[1] * local[2] + parent[3] * local[3],
            parent[0] * local[4] + parent[2] * local[5] + parent[4],
            parent[1] * local[4] + parent[3] * local[5] + parent[5],
        ]
    }

    fn inverse(transform: [f64; 6]) -> Option<[f64; 6]> {
        let determinant = transform[0] * transform[3] - transform[1] * transform[2];
        if determinant.abs() <= f64::EPSILON {
            return None;
        }
        Some([
            transform[3] / determinant,
            -transform[1] / determinant,
            -transform[2] / determinant,
            transform[0] / determinant,
            (transform[2] * transform[5] - transform[3] * transform[4]) / determinant,
            (transform[1] * transform[4] - transform[0] * transform[5]) / determinant,
        ])
    }

    fn point(transform: [f64; 6], x: f64, y: f64) -> [f64; 2] {
        [
            transform[0] * x + transform[2] * y + transform[4],
            transform[1] * x + transform[3] * y + transform[5],
        ]
    }

    fn transformed_corners(
        transform: [f64; 6],
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> [[f64; 2]; 4] {
        [
            point(transform, x, y),
            point(transform, x + width, y),
            point(transform, x, y + height),
            point(transform, x + width, y + height),
        ]
    }

    fn bounds(points: [[f64; 2]; 4]) -> [f64; 4] {
        points.into_iter().fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |mut bounds, point| {
                bounds[0] = bounds[0].min(point[0]);
                bounds[1] = bounds[1].min(point[1]);
                bounds[2] = bounds[2].max(point[0]);
                bounds[3] = bounds[3].max(point[1]);
                bounds
            },
        )
    }

    fn intersect(left: [f64; 4], right: [f64; 4]) -> [f64; 4] {
        [
            left[0].max(right[0]),
            left[1].max(right[1]),
            left[2].min(right[2]),
            left[3].min(right[3]),
        ]
    }
}

#[cfg(target_vendor = "apple")]
#[derive(Clone, Copy)]
struct PreparedTexture {
    texture: Id,
    sampler: Id,
}

#[cfg(target_vendor = "apple")]
#[repr(C)]
struct LayerFilterParams {
    kind: u32,
    padding: [u32; 3],
    params: [f32; 4],
    color: [f32; 4],
    matrix: [[f32; 4]; 4],
    bias: [f32; 4],
}

#[cfg(target_vendor = "apple")]
#[repr(C)]
struct LayerCompositeParams {
    blend_mode: u32,
    padding: [u32; 3],
    opacity: f32,
    clip: [f32; 4],
}

#[cfg(target_vendor = "apple")]
struct MetalLayerFrame {
    parent_target: Id,
    descriptor: GpuLayer,
    composite_clip: GpuRect,
}

#[cfg(target_vendor = "apple")]
struct MetalLayerCompute {
    command_buffer: Id,
    device: Id,
    filter_pipeline: Id,
    composite_pipeline: Id,
    width: u32,
    height: u32,
    owned_textures: Vec<Id>,
}

#[cfg(target_vendor = "apple")]
#[repr(C)]
#[derive(Clone, Copy)]
struct MetalScissorRect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

#[cfg(target_vendor = "apple")]
unsafe fn render_plan_unsafe(plan: GpuPaintPlan) -> PixelContainer {
    const MAX_DIMENSION: u32 = 16_384;
    if plan.width > MAX_DIMENSION || plan.height > MAX_DIMENSION {
        panic!(
            "Scene dimensions {}x{} exceed maximum {}x{}",
            plan.width, plan.height, MAX_DIMENSION, MAX_DIMENSION
        );
    }
    if plan.width == 0 || plan.height == 0 {
        return PixelContainer::new(plan.width, plan.height);
    }

    let device = MTLCreateSystemDefaultDevice();
    assert!(!device.is_null(), "No Metal-capable GPU found");
    let command_queue = msg_send_id(device, "newCommandQueue");
    assert!(!command_queue.is_null(), "Failed to create command queue");
    let target = create_offscreen_texture(device, plan.width, plan.height);
    let pipeline = create_rect_pipeline(device);
    let layer_library = compile_shader_library(device, LAYER_SHADER_SOURCE);
    let filter_pipeline = create_compute_pipeline(device, layer_library, "paint_filter");
    let composite_pipeline = create_compute_pipeline(device, layer_library, "paint_composite");

    let white_upload = GpuImageUpload {
        width: 1,
        height: 1,
        data: vec![255, 255, 255, 255],
        filter: GpuTextureFilter::Nearest,
        kind: paint_vm_gpu_core::GpuTextureKind::Image,
    };
    let white = prepare_texture(device, &white_upload);
    let mut textures: Vec<PreparedTexture> = plan
        .images
        .iter()
        .map(|upload| prepare_texture(device, upload))
        .collect();

    let clear_color = MTLClearColor {
        red: plan.background.r as f64,
        green: plan.background.g as f64,
        blue: plan.background.b as f64,
        alpha: plan.background.a as f64,
    };
    let command_buffer = msg_send_id(command_queue, "commandBuffer");
    let mut current_target = target;
    let mut encoder = begin_render_encoder(command_buffer, target, true, clear_color);
    msg!(encoder, "setRenderPipelineState:", pipeline);

    let viewport = [plan.width as f32, plan.height as f32];
    let full_clip = GpuRect {
        x: 0.0,
        y: 0.0,
        width: plan.width as f32,
        height: plan.height as f32,
    };
    let mut clip_stack = vec![full_clip];
    let mut layer_stack: Vec<MetalLayerFrame> = Vec::new();
    let mut layer_compute = MetalLayerCompute {
        command_buffer,
        device,
        filter_pipeline,
        composite_pipeline,
        width: plan.width,
        height: plan.height,
        owned_textures: Vec::new(),
    };
    set_scissor(encoder, full_clip, plan.width, plan.height);

    for command in &plan.commands {
        match command {
            GpuCommand::DrawMesh { mesh_id } => {
                if clip_stack
                    .last()
                    .is_some_and(|clip| clip.width <= 0.0 || clip.height <= 0.0)
                {
                    continue;
                }
                if let Some(mesh) = plan.meshes.get(*mesh_id) {
                    let prepared = mesh
                        .texture_id
                        .and_then(|texture_id| textures.get(texture_id))
                        .copied()
                        .unwrap_or(white);
                    encode_mesh(encoder, device, mesh, prepared, viewport);
                }
            }
            GpuCommand::DrawGlyphRun(run) => {
                if clip_stack
                    .last()
                    .is_some_and(|clip| clip.width <= 0.0 || clip.height <= 0.0)
                {
                    continue;
                }
                if let Some(rasterized) =
                    glyph_run_overlay::rasterize_gpu_glyph_run(run, plan.width, plan.height)
                {
                    let upload = GpuImageUpload {
                        width: rasterized.pixels.width,
                        height: rasterized.pixels.height,
                        data: rasterized.pixels.data,
                        filter: GpuTextureFilter::Nearest,
                        kind: paint_vm_gpu_core::GpuTextureKind::Image,
                    };
                    let prepared = prepare_texture(device, &upload);
                    textures.push(prepared);
                    let mesh =
                        textured_rect_mesh(rasterized.x, rasterized.y, upload.width, upload.height);
                    encode_mesh(encoder, device, &mesh, prepared, viewport);
                }
            }
            GpuCommand::PushClip { rect } => {
                let clipped = intersect_gpu_rect(*clip_stack.last().unwrap(), *rect);
                clip_stack.push(clipped);
                set_scissor(encoder, clipped, plan.width, plan.height);
            }
            GpuCommand::PopClip => {
                if clip_stack.len() > 1 {
                    clip_stack.pop();
                }
                set_scissor(
                    encoder,
                    *clip_stack.last().unwrap(),
                    plan.width,
                    plan.height,
                );
            }
            GpuCommand::BeginLayer(descriptor) => {
                msg!(encoder, "endEncoding");
                let layer_target = layer_compute.create_target();
                layer_stack.push(MetalLayerFrame {
                    parent_target: current_target,
                    descriptor: descriptor.clone(),
                    composite_clip: *clip_stack.last().unwrap(),
                });
                current_target = layer_target;
                encoder = begin_render_encoder(
                    command_buffer,
                    current_target,
                    true,
                    MTLClearColor {
                        red: 0.0,
                        green: 0.0,
                        blue: 0.0,
                        alpha: 0.0,
                    },
                );
                msg!(encoder, "setRenderPipelineState:", pipeline);
                set_scissor(
                    encoder,
                    *clip_stack.last().unwrap(),
                    plan.width,
                    plan.height,
                );
            }
            GpuCommand::EndLayer => {
                let Some(frame) = layer_stack.pop() else {
                    continue;
                };
                msg!(encoder, "endEncoding");
                let mut filtered = current_target;
                for filter in &frame.descriptor.filters {
                    filtered = layer_compute.apply_filter(filtered, filter);
                }
                current_target = layer_compute.composite(
                    filtered,
                    frame.parent_target,
                    &frame.descriptor,
                    frame.composite_clip,
                );
                encoder = begin_render_encoder(
                    command_buffer,
                    current_target,
                    false,
                    MTLClearColor {
                        red: 0.0,
                        green: 0.0,
                        blue: 0.0,
                        alpha: 0.0,
                    },
                );
                msg!(encoder, "setRenderPipelineState:", pipeline);
                set_scissor(
                    encoder,
                    *clip_stack.last().unwrap(),
                    plan.width,
                    plan.height,
                );
            }
            // PaintText requires shaping before it can become a GPU command.
            // Layout-backed browser scenes emit GlyphRun, which is ordered above.
            GpuCommand::DrawText(_) => {}
        }
    }

    msg!(encoder, "endEncoding");
    msg!(command_buffer, "commit");
    msg!(command_buffer, "waitUntilCompleted");
    assert_command_buffer_completed(command_buffer);
    let pixels = read_back_pixels(current_target, plan.width, plan.height);

    for prepared in textures {
        release(prepared.texture);
        release(prepared.sampler);
    }
    for texture in layer_compute.owned_textures {
        release(texture);
    }
    release(white.texture);
    release(white.sampler);
    release(target);
    release(composite_pipeline);
    release(filter_pipeline);
    release(layer_library);
    release(pipeline);
    release(command_queue);
    release(device);
    pixels
}

#[cfg(target_vendor = "apple")]
unsafe fn assert_command_buffer_completed(command_buffer: Id) {
    let send_status: unsafe extern "C" fn(Id, Sel) -> usize =
        std::mem::transmute(objc_msgSend as *const ());
    let status = send_status(command_buffer, sel("status"));
    if status == 5 {
        let error: Id = msg!(command_buffer, "error");
        let message = if error.is_null() {
            "Metal returned no command-buffer diagnostic".to_string()
        } else {
            let description: Id = msg!(error, "localizedDescription");
            let utf8 = msg!(description, "UTF8String") as *const std::ffi::c_char;
            if utf8.is_null() {
                "Metal returned an unreadable command-buffer diagnostic".to_string()
            } else {
                CStr::from_ptr(utf8).to_string_lossy().into_owned()
            }
        };
        panic!("Metal command buffer failed: {message}");
    }
    assert_eq!(status, 4, "Metal command buffer ended with status {status}");
}

#[cfg(target_vendor = "apple")]
unsafe fn begin_render_encoder(
    command_buffer: Id,
    target: Id,
    clear: bool,
    clear_color: MTLClearColor,
) -> Id {
    let pass_desc = msg_send_class(class("MTLRenderPassDescriptor"), "renderPassDescriptor");
    let attachments = msg_send_id(pass_desc, "colorAttachments");
    let attachment: Id = msg!(attachments, "objectAtIndexedSubscript:", 0usize);
    msg!(attachment, "setTexture:", target);
    msg!(
        attachment,
        "setLoadAction:",
        if clear {
            MTL_LOAD_ACTION_CLEAR as usize
        } else {
            MTL_LOAD_ACTION_LOAD as usize
        }
    );
    msg!(
        attachment,
        "setStoreAction:",
        MTL_STORE_ACTION_STORE as usize
    );
    if clear {
        let set_clear_color: unsafe extern "C" fn(Id, Sel, MTLClearColor) =
            std::mem::transmute(objc_msgSend as *const ());
        set_clear_color(attachment, sel("setClearColor:"), clear_color);
    }
    msg!(
        command_buffer,
        "renderCommandEncoderWithDescriptor:",
        pass_desc
    )
}

#[cfg(target_vendor = "apple")]
unsafe fn create_compute_pipeline(device: Id, library: Id, function_name: &str) -> Id {
    let name = nsstring(function_name);
    let function: Id = msg!(library, "newFunctionWithName:", name);
    CFRelease(name);
    assert!(!function.is_null(), "{function_name} shader not found");
    let mut error: Id = ptr::null_mut();
    let pipeline: Id = msg!(
        device,
        "newComputePipelineStateWithFunction:error:",
        function,
        &mut error as *mut Id
    );
    release(function);
    assert!(
        !pipeline.is_null(),
        "failed to create {function_name} compute pipeline"
    );
    pipeline
}

#[cfg(target_vendor = "apple")]
impl MetalLayerCompute {
    unsafe fn create_target(&mut self) -> Id {
        let texture = create_offscreen_texture(self.device, self.width, self.height);
        self.owned_textures.push(texture);
        texture
    }

    unsafe fn apply_filter(&mut self, source: Id, filter: &GpuFilter) -> Id {
        let destination = self.create_target();
        let params = metal_filter_params(filter);
        let encoder = msg_send_id(self.command_buffer, "computeCommandEncoder");
        msg!(encoder, "setComputePipelineState:", self.filter_pipeline);
        msg!(encoder, "setTexture:atIndex:", source, 0usize);
        msg!(encoder, "setTexture:atIndex:", destination, 1usize);
        msg!(
            encoder,
            "setBytes:length:atIndex:",
            &params as *const LayerFilterParams as Id,
            std::mem::size_of::<LayerFilterParams>(),
            0usize
        );
        self.dispatch(encoder);
        destination
    }

    unsafe fn composite(
        &mut self,
        source: Id,
        backdrop: Id,
        layer: &GpuLayer,
        clip: GpuRect,
    ) -> Id {
        let destination = self.create_target();
        let params = LayerCompositeParams {
            blend_mode: metal_blend_mode(layer.blend_mode),
            padding: [0; 3],
            opacity: layer.opacity,
            clip: [clip.x, clip.y, clip.width, clip.height],
        };
        let encoder = msg_send_id(self.command_buffer, "computeCommandEncoder");
        msg!(encoder, "setComputePipelineState:", self.composite_pipeline);
        msg!(encoder, "setTexture:atIndex:", source, 0usize);
        msg!(encoder, "setTexture:atIndex:", backdrop, 1usize);
        msg!(encoder, "setTexture:atIndex:", destination, 2usize);
        msg!(
            encoder,
            "setBytes:length:atIndex:",
            &params as *const LayerCompositeParams as Id,
            std::mem::size_of::<LayerCompositeParams>(),
            0usize
        );
        self.dispatch(encoder);
        destination
    }

    unsafe fn dispatch(&self, encoder: Id) {
        let grid = MTLSize {
            width: self.width as c_ulong,
            height: self.height as c_ulong,
            depth: 1,
        };
        let threads = MTLSize {
            width: 8,
            height: 8,
            depth: 1,
        };
        let dispatch: unsafe extern "C" fn(Id, Sel, MTLSize, MTLSize) =
            std::mem::transmute(objc_msgSend as *const ());
        dispatch(
            encoder,
            sel("dispatchThreads:threadsPerThreadgroup:"),
            grid,
            threads,
        );
        msg!(encoder, "endEncoding");
    }
}

#[cfg(target_vendor = "apple")]
fn metal_filter_params(filter: &GpuFilter) -> LayerFilterParams {
    let mut params = LayerFilterParams {
        kind: 0,
        padding: [0; 3],
        params: [0.0; 4],
        color: [0.0; 4],
        matrix: [[0.0; 4]; 4],
        bias: [0.0; 4],
    };
    match filter {
        GpuFilter::Blur { radius } => {
            params.kind = 0;
            params.params[0] = *radius;
        }
        GpuFilter::DropShadow {
            dx,
            dy,
            blur,
            color,
        } => {
            params.kind = 1;
            params.params = [*dx, *dy, 0.0, *blur];
            params.color = [color.r, color.g, color.b, color.a];
        }
        GpuFilter::ColorMatrix { matrix } => {
            params.kind = 2;
            for row in 0..4 {
                let offset = row * 5;
                params.matrix[row].copy_from_slice(&matrix[offset..offset + 4]);
                params.bias[row] = matrix[offset + 4];
            }
        }
        GpuFilter::Brightness { amount } => {
            params.kind = 3;
            params.params[0] = *amount;
        }
        GpuFilter::Contrast { amount } => {
            params.kind = 4;
            params.params[0] = *amount;
        }
        GpuFilter::Saturate { amount } => {
            params.kind = 5;
            params.params[0] = *amount;
        }
        GpuFilter::HueRotate { angle_degrees } => {
            params.kind = 6;
            params.params[0] = *angle_degrees;
        }
        GpuFilter::Invert { amount } => {
            params.kind = 7;
            params.params[0] = *amount;
        }
        GpuFilter::Opacity { amount } => {
            params.kind = 8;
            params.params[0] = *amount;
        }
    }
    params
}

#[cfg(target_vendor = "apple")]
fn metal_blend_mode(mode: GpuBlendMode) -> u32 {
    match mode {
        GpuBlendMode::Normal => 0,
        GpuBlendMode::Multiply => 1,
        GpuBlendMode::Screen => 2,
        GpuBlendMode::Overlay => 3,
        GpuBlendMode::Darken => 4,
        GpuBlendMode::Lighten => 5,
        GpuBlendMode::ColorDodge => 6,
        GpuBlendMode::ColorBurn => 7,
        GpuBlendMode::HardLight => 8,
        GpuBlendMode::SoftLight => 9,
        GpuBlendMode::Difference => 10,
        GpuBlendMode::Exclusion => 11,
        GpuBlendMode::Hue => 12,
        GpuBlendMode::Saturation => 13,
        GpuBlendMode::Color => 14,
        GpuBlendMode::Luminosity => 15,
    }
}

#[cfg(target_vendor = "apple")]
fn textured_rect_mesh(x: f32, y: f32, width: u32, height: u32) -> GpuMesh {
    let white = GpuColor {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    GpuMesh {
        vertices: vec![
            GpuVertex {
                position: GpuPoint { x, y },
                uv: [0.0, 0.0],
                color: white,
            },
            GpuVertex {
                position: GpuPoint {
                    x: x + width as f32,
                    y,
                },
                uv: [1.0, 0.0],
                color: white,
            },
            GpuVertex {
                position: GpuPoint {
                    x: x + width as f32,
                    y: y + height as f32,
                },
                uv: [1.0, 1.0],
                color: white,
            },
            GpuVertex {
                position: GpuPoint {
                    x,
                    y: y + height as f32,
                },
                uv: [0.0, 1.0],
                color: white,
            },
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        texture_id: None,
        label: "coretext-glyph-run",
    }
}

#[cfg(target_vendor = "apple")]
fn intersect_gpu_rect(left: GpuRect, right: GpuRect) -> GpuRect {
    let x = left.x.max(right.x);
    let y = left.y.max(right.y);
    let right_edge = (left.x + left.width).min(right.x + right.width);
    let bottom_edge = (left.y + left.height).min(right.y + right.height);
    GpuRect {
        x,
        y,
        width: (right_edge - x).max(0.0),
        height: (bottom_edge - y).max(0.0),
    }
}

#[cfg(target_vendor = "apple")]
unsafe fn set_scissor(encoder: Id, rect: GpuRect, width: u32, height: u32) {
    let x = rect.x.floor().clamp(0.0, width as f32) as usize;
    let y = rect.y.floor().clamp(0.0, height as f32) as usize;
    let right = (rect.x + rect.width).ceil().clamp(x as f32, width as f32) as usize;
    let bottom = (rect.y + rect.height).ceil().clamp(y as f32, height as f32) as usize;
    let scissor = MetalScissorRect {
        x,
        y,
        width: right - x,
        height: bottom - y,
    };
    let set_scissor: unsafe extern "C" fn(Id, Sel, MetalScissorRect) =
        std::mem::transmute(objc_msgSend as *const ());
    set_scissor(encoder, sel("setScissorRect:"), scissor);
}

#[cfg(target_vendor = "apple")]
unsafe fn encode_mesh(
    encoder: Id,
    device: Id,
    mesh: &GpuMesh,
    texture: PreparedTexture,
    viewport: [f32; 2],
) {
    let mut positions = Vec::with_capacity(mesh.indices.len() * 2);
    let mut uvs = Vec::with_capacity(mesh.indices.len() * 2);
    let mut colors = Vec::with_capacity(mesh.indices.len() * 4);
    for index in &mesh.indices {
        let Some(vertex) = mesh.vertices.get(*index as usize) else {
            continue;
        };
        positions.extend_from_slice(&[vertex.position.x, vertex.position.y]);
        uvs.extend_from_slice(&vertex.uv);
        colors.extend_from_slice(&[
            vertex.color.r,
            vertex.color.g,
            vertex.color.b,
            vertex.color.a,
        ]);
    }
    if positions.is_empty() {
        return;
    }

    let position_buffer = create_buffer(device, &positions);
    let uv_buffer = create_buffer(device, &uvs);
    let color_buffer = create_buffer(device, &colors);
    msg!(
        encoder,
        "setVertexBuffer:offset:atIndex:",
        position_buffer,
        0usize,
        0usize
    );
    msg!(
        encoder,
        "setVertexBuffer:offset:atIndex:",
        uv_buffer,
        0usize,
        1usize
    );
    msg!(
        encoder,
        "setVertexBuffer:offset:atIndex:",
        color_buffer,
        0usize,
        2usize
    );
    msg!(
        encoder,
        "setVertexBytes:length:atIndex:",
        viewport.as_ptr() as Id,
        std::mem::size_of_val(&viewport),
        3usize
    );
    msg!(
        encoder,
        "setFragmentTexture:atIndex:",
        texture.texture,
        0usize
    );
    msg!(
        encoder,
        "setFragmentSamplerState:atIndex:",
        texture.sampler,
        0usize
    );
    msg!(
        encoder,
        "drawPrimitives:vertexStart:vertexCount:",
        MTL_PRIMITIVE_TYPE_TRIANGLE as usize,
        0usize,
        positions.len() / 2
    );
    release(position_buffer);
    release(uv_buffer);
    release(color_buffer);
}

#[cfg(target_vendor = "apple")]
unsafe fn prepare_texture(device: Id, upload: &GpuImageUpload) -> PreparedTexture {
    assert_eq!(
        upload.data.len(),
        upload.width as usize * upload.height as usize * 4,
        "GPU texture upload must contain tightly packed RGBA8 pixels"
    );
    let desc = alloc_init("MTLTextureDescriptor");
    msg!(
        desc,
        "setPixelFormat:",
        MTL_PIXEL_FORMAT_RGBA8_UNORM as usize
    );
    msg!(desc, "setWidth:", upload.width as usize);
    msg!(desc, "setHeight:", upload.height as usize);
    msg!(desc, "setTextureType:", MTL_TEXTURE_TYPE_2D as usize);
    msg!(desc, "setUsage:", MTL_TEXTURE_USAGE_SHADER_READ as usize);
    let texture: Id = msg!(device, "newTextureWithDescriptor:", desc);
    release(desc);
    assert!(!texture.is_null(), "Failed to create image texture");

    let region = MTLRegion {
        origin: MTLOrigin { x: 0, y: 0, z: 0 },
        size: MTLSize {
            width: upload.width as c_ulong,
            height: upload.height as c_ulong,
            depth: 1,
        },
    };
    let replace: unsafe extern "C" fn(Id, Sel, MTLRegion, usize, *const std::ffi::c_void, usize) =
        std::mem::transmute(objc_msgSend as *const ());
    replace(
        texture,
        sel("replaceRegion:mipmapLevel:withBytes:bytesPerRow:"),
        region,
        0,
        upload.data.as_ptr() as *const std::ffi::c_void,
        upload.width as usize * 4,
    );

    let sampler_desc = alloc_init("MTLSamplerDescriptor");
    let filter = match upload.filter {
        GpuTextureFilter::Nearest => 0usize,
        GpuTextureFilter::Linear => 1usize,
    };
    msg!(sampler_desc, "setMinFilter:", filter);
    msg!(sampler_desc, "setMagFilter:", filter);
    let sampler: Id = msg!(device, "newSamplerStateWithDescriptor:", sampler_desc);
    release(sampler_desc);
    assert!(!sampler.is_null(), "Failed to create image sampler");
    PreparedTexture { texture, sampler }
}

#[cfg(all(test, target_vendor = "apple"))]
#[allow(dead_code)]
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
    msg!(
        attachment0,
        "setLoadAction:",
        MTL_LOAD_ACTION_CLEAR as usize
    );
    msg!(
        attachment0,
        "setStoreAction:",
        MTL_STORE_ACTION_STORE as usize
    );

    // MTLClearColor is 4 doubles — passed as HFA in d0-d3 on arm64
    let clear_color = MTLClearColor {
        red: cr,
        green: cg,
        blue: cb,
        alpha: ca,
    };
    let set_clear_color: unsafe extern "C" fn(Id, Sel, MTLClearColor) =
        std::mem::transmute(objc_msgSend as *const ());
    set_clear_color(attachment0, sel("setClearColor:"), clear_color);

    let command_buffer = msg_send_id(command_queue, "commandBuffer");
    let encoder: Id = msg!(
        command_buffer,
        "renderCommandEncoderWithDescriptor:",
        pass_desc
    );

    let viewport_size: [f32; 2] = [width as f32, height as f32];

    // Draw all rectangles, lines, ellipses, paths (collected as triangles)
    if !positions.is_empty() {
        let vertex_count = positions.len() / 2;

        let pos_buffer = create_buffer(device, &positions);
        let color_buffer = create_buffer(device, &colors);

        msg!(encoder, "setRenderPipelineState:", rect_pipeline);
        msg!(
            encoder,
            "setVertexBuffer:offset:atIndex:",
            pos_buffer,
            0usize,
            0usize
        );
        msg!(
            encoder,
            "setVertexBuffer:offset:atIndex:",
            color_buffer,
            0usize,
            1usize
        );

        let vp_ptr = viewport_size.as_ptr() as *const std::ffi::c_void as Id;
        msg!(
            encoder,
            "setVertexBytes:length:atIndex:",
            vp_ptr,
            8usize,
            2usize
        );

        msg!(
            encoder,
            "drawPrimitives:vertexStart:vertexCount:",
            MTL_PRIMITIVE_TYPE_TRIANGLE as usize,
            0usize,
            vertex_count
        );

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
    msg!(
        desc,
        "setPixelFormat:",
        MTL_PIXEL_FORMAT_RGBA8_UNORM as usize
    );
    msg!(desc, "setWidth:", width as usize);
    msg!(desc, "setHeight:", height as usize);
    // MTLTextureType2D = 2
    msg!(desc, "setTextureType:", MTL_TEXTURE_TYPE_2D as usize);

    let usage = MTL_TEXTURE_USAGE_RENDER_TARGET
        | MTL_TEXTURE_USAGE_SHADER_READ
        | MTL_TEXTURE_USAGE_SHADER_WRITE;
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
        device,
        "newLibraryWithSource:options:error:",
        source_ns,
        options,
        &mut error as *mut Id
    );
    CFRelease(source_ns);

    if library.is_null() {
        let message = if error.is_null() {
            "Metal returned no compiler diagnostic".to_string()
        } else {
            let description: Id = msg!(error, "localizedDescription");
            let utf8 = msg!(description, "UTF8String") as *const std::ffi::c_char;
            if utf8.is_null() {
                "Metal returned an unreadable compiler diagnostic".to_string()
            } else {
                CStr::from_ptr(utf8).to_string_lossy().into_owned()
            }
        };
        panic!("Metal shader compilation failed: {message}");
    }
    library
}

#[cfg(target_vendor = "apple")]
unsafe fn create_rect_pipeline(device: Id) -> Id {
    let library = compile_shader_library(device, TEXTURED_SHADER_SOURCE);

    let vname = nsstring("paint_vertex");
    let fname = nsstring("paint_fragment");
    let vertex_fn: Id = msg!(library, "newFunctionWithName:", vname);
    let fragment_fn: Id = msg!(library, "newFunctionWithName:", fname);
    CFRelease(vname);
    CFRelease(fname);

    assert!(!vertex_fn.is_null(), "paint_vertex shader not found");
    assert!(!fragment_fn.is_null(), "paint_fragment shader not found");

    let desc = alloc_init("MTLRenderPipelineDescriptor");
    msg!(desc, "setVertexFunction:", vertex_fn);
    msg!(desc, "setFragmentFunction:", fragment_fn);

    setup_pipeline_color_attachment(desc);

    let mut error: Id = ptr::null_mut();
    let pipeline: Id = msg!(
        device,
        "newRenderPipelineStateWithDescriptor:error:",
        desc,
        &mut error as *mut Id
    );

    release(vertex_fn);
    release(fragment_fn);
    release(library);
    release(desc);

    assert!(
        !pipeline.is_null(),
        "Failed to create rect render pipeline state"
    );
    pipeline
}

#[cfg(target_vendor = "apple")]
unsafe fn setup_pipeline_color_attachment(desc: Id) {
    let attachments = msg_send_id(desc, "colorAttachments");
    let att0: Id = msg!(attachments, "objectAtIndexedSubscript:", 0usize);
    msg!(
        att0,
        "setPixelFormat:",
        MTL_PIXEL_FORMAT_RGBA8_UNORM as usize
    );

    // Enable standard src-over alpha blending so transparent pixels composite correctly.
    // The formula is:  dst = src.rgb * src.a + dst.rgb * (1 - src.a)
    msg!(att0, "setBlendingEnabled:", 1usize);
    msg!(att0, "setSourceRGBBlendFactor:", 4usize); // sourceAlpha
    msg!(att0, "setDestinationRGBBlendFactor:", 5usize); // oneMinusSourceAlpha
    msg!(att0, "setSourceAlphaBlendFactor:", 1usize); // one
    msg!(att0, "setDestinationAlphaBlendFactor:", 5usize); // oneMinusSourceAlpha
}

#[cfg(target_vendor = "apple")]
unsafe fn create_buffer(device: Id, data: &[f32]) -> Id {
    let byte_len = std::mem::size_of_val(data);
    // MTLResourceStorageModeShared = 0
    let buffer: Id = msg!(
        device,
        "newBufferWithBytes:length:options:",
        data.as_ptr() as Id,
        byte_len,
        0usize
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
        Id,
        Sel,
        *mut u8,   // bytes pointer
        usize,     // bytesPerRow
        MTLRegion, // region (compiler passes indirectly on arm64)
        usize,     // mipmapLevel
    ) = std::mem::transmute(objc_msgSend as *const ());
    get_bytes(
        texture,
        sel("getBytes:bytesPerRow:fromRegion:mipmapLevel:"),
        data.as_mut_ptr(),
        bytes_per_row,
        region,
        0,
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
        CGColorSpaceCreateDeviceRGB, CGColorSpaceRelease, CGContextRef, CGContextRelease,
        CGContextRestoreGState, CGContextSaveGState, CGContextSetRGBFillColor,
        CGContextSetShouldAntialias, CGContextSetShouldSmoothFonts, CGContextSetTextMatrix,
        CGPoint, CTFontCreateWithName, CTFontDrawGlyphs, Id, K_CG_BITMAP_BYTE_ORDER_32_LITTLE,
        K_CG_IMAGE_ALPHA_PREMULTIPLIED_FIRST, NIL,
    };
    use paint_instructions::{
        GlyphPosition, PaintBase, PaintGlyphRun, PaintInstruction, PaintScene, PixelContainer,
    };
    use paint_vm_gpu_core::GpuGlyphRun;

    pub(super) struct RasterizedGlyphRun {
        pub pixels: PixelContainer,
        pub x: f32,
        pub y: f32,
    }

    pub(super) unsafe fn rasterize_gpu_glyph_run(
        run: &GpuGlyphRun,
        width: u32,
        height: u32,
    ) -> Option<RasterizedGlyphRun> {
        if width == 0
            || height == 0
            || run.glyphs.is_empty()
            || !run.font_ref.starts_with("coretext:")
        {
            return None;
        }
        let margin = run.font_size.max(1.0) * 2.0;
        let min_x = run
            .glyphs
            .iter()
            .map(|glyph| glyph.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = run
            .glyphs
            .iter()
            .map(|glyph| glyph.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = run
            .glyphs
            .iter()
            .map(|glyph| glyph.y)
            .fold(f32::INFINITY, f32::min);
        let max_y = run
            .glyphs
            .iter()
            .map(|glyph| glyph.y)
            .fold(f32::NEG_INFINITY, f32::max);
        let x = (min_x - margin).floor().clamp(0.0, width as f32);
        let y = (min_y - margin).floor().clamp(0.0, height as f32);
        let right = (max_x + margin).ceil().clamp(x, width as f32);
        let bottom = (max_y + margin).ceil().clamp(y, height as f32);
        let texture_width = (right - x) as u32;
        let texture_height = (bottom - y) as u32;
        if texture_width == 0 || texture_height == 0 {
            return None;
        }
        let mut pixels = PixelContainer::new(texture_width, texture_height);
        let glyph_run = PaintGlyphRun {
            base: PaintBase::default(),
            glyphs: run
                .glyphs
                .iter()
                .map(|glyph| GlyphPosition {
                    glyph_id: glyph.glyph_id,
                    x: (glyph.x - x) as f64,
                    y: (glyph.y - y) as f64,
                })
                .collect(),
            font_ref: run.font_ref.clone(),
            font_size: run.font_size as f64,
            fill: Some(format!(
                "rgba({}, {}, {}, {})",
                (run.color.r.clamp(0.0, 1.0) * 255.0).round() as u8,
                (run.color.g.clamp(0.0, 1.0) * 255.0).round() as u8,
                (run.color.b.clamp(0.0, 1.0) * 255.0).round() as u8,
                run.color.a.clamp(0.0, 1.0)
            )),
        };
        let scene = PaintScene {
            width: texture_width as f64,
            height: texture_height as f64,
            background: "transparent".to_string(),
            instructions: vec![PaintInstruction::GlyphRun(glyph_run)],
            id: None,
            metadata: None,
        };
        overlay_coretext_glyph_runs(&scene, &mut pixels);
        unpremultiply_rgba(&mut pixels.data);
        Some(RasterizedGlyphRun { pixels, x, y })
    }

    fn unpremultiply_rgba(data: &mut [u8]) {
        for pixel in data.as_chunks_mut::<4>().0 {
            let alpha = u32::from(pixel[3]);
            if alpha == 0 {
                pixel[..3].fill(0);
                continue;
            }
            for channel in &mut pixel[..3] {
                *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }

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

        for (gr, transform) in runs {
            draw_one_glyph_run(ctx, gr, transform, height as f64);
        }

        CGContextRestoreGState(ctx);
        CGContextRelease(ctx);
    }

    fn multiply_transform(parent: [f64; 6], local: [f64; 6]) -> [f64; 6] {
        [
            parent[0] * local[0] + parent[2] * local[1],
            parent[1] * local[0] + parent[3] * local[1],
            parent[0] * local[2] + parent[2] * local[3],
            parent[1] * local[2] + parent[3] * local[3],
            parent[0] * local[4] + parent[2] * local[5] + parent[4],
            parent[1] * local[4] + parent[3] * local[5] + parent[5],
        ]
    }

    fn collect_coretext_runs(instructions: &[PaintInstruction]) -> Vec<(&PaintGlyphRun, [f64; 6])> {
        let mut out = Vec::new();
        fn walk<'a>(
            ins: &'a [PaintInstruction],
            transform: [f64; 6],
            out: &mut Vec<(&'a PaintGlyphRun, [f64; 6])>,
        ) {
            for i in ins {
                match i {
                    PaintInstruction::GlyphRun(g) if g.font_ref.starts_with("coretext:") => {
                        out.push((g, transform));
                    }
                    PaintInstruction::Group(grp) => walk(
                        &grp.children,
                        grp.transform
                            .map(|local| multiply_transform(transform, local))
                            .unwrap_or(transform),
                        out,
                    ),
                    PaintInstruction::Clip(c) => walk(&c.children, transform, out),
                    PaintInstruction::Layer(l) => walk(
                        &l.children,
                        l.transform
                            .map(|local| multiply_transform(transform, local))
                            .unwrap_or(transform),
                        out,
                    ),
                    _ => {}
                }
            }
        }
        walk(instructions, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], &mut out);
        out
    }

    unsafe fn draw_one_glyph_run(
        ctx: CGContextRef,
        run: &PaintGlyphRun,
        transform: [f64; 6],
        image_height: f64,
    ) {
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

        let color = parse_css_color(run.fill.as_deref().unwrap_or("rgb(0, 0, 0)"));
        let (r, g, b, a) = coregraphics_fill_color(color);
        CGContextSetRGBFillColor(ctx, r, g, b, a);
        CGContextSetTextMatrix(
            ctx,
            CGAffineTransform {
                a: transform[0],
                b: -transform[1],
                c: -transform[2],
                d: transform[3],
                tx: transform[2] * image_height + transform[4],
                ty: image_height * (1.0 - transform[3]) - transform[5],
            },
        );

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
        let (inner, has_alpha) =
            if let Some(i) = s.strip_prefix("rgba(").and_then(|x| x.strip_suffix(")")) {
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
        (
            r.clamp(0.0, 1.0),
            g.clamp(0.0, 1.0),
            b.clamp(0.0, 1.0),
            a.clamp(0.0, 1.0),
        )
    }

    fn coregraphics_fill_color((r, g, b, a): (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
        // This supported bitmap format is BGRA in memory while PixelContainer is RGBA.
        // Swap red and blue at the CoreGraphics boundary so stored bytes remain RGBA.
        (b, g, r, a)
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
        fn coregraphics_fill_color_preserves_rgba_pixel_bytes() {
            assert_eq!(
                coregraphics_fill_color((1.0, 0.5, 0.25, 0.75)),
                (0.25, 0.5, 1.0, 0.75)
            );
        }

        #[test]
        fn ordered_glyph_texture_converts_premultiplied_bytes_to_straight_alpha() {
            let mut rgba = vec![64, 32, 16, 128, 9, 8, 7, 0];
            unpremultiply_rgba(&mut rgba);
            assert_eq!(rgba, vec![128, 64, 32, 128, 0, 0, 0, 0]);
        }

        #[test]
        fn parse_css_color_malformed_returns_black() {
            let (r, g, b, a) = parse_css_color("not-a-color");
            assert_eq!((r, g, b, a), (0.0, 0.0, 0.0, 1.0));
        }

        #[test]
        fn coretext_runs_inherit_group_transforms() {
            let transform = [0.0, -1.0, 1.0, 0.0, 25.0, 40.0];
            let instructions = vec![PaintInstruction::Group(paint_instructions::PaintGroup {
                base: paint_instructions::PaintBase::default(),
                children: vec![PaintInstruction::GlyphRun(PaintGlyphRun {
                    base: paint_instructions::PaintBase::default(),
                    glyphs: vec![],
                    font_ref: "coretext:Helvetica@14".into(),
                    font_size: 14.0,
                    fill: None,
                })],
                transform: Some(transform),
                opacity: None,
            })];

            let runs = collect_coretext_runs(&instructions);
            assert_eq!(runs.len(), 1);
            assert_eq!(runs[0].1, transform);
        }
    }
}

// ---------------------------------------------------------------------------
// Live-drawable present (Apple only)
// ---------------------------------------------------------------------------

// `metal_layer` is a raw Objective-C `id` handle that we hand to the Metal
// live-present path. This is an FFI boundary — the caller owns the CAMetalLayer
// and upholds the pointer contract — so the safe wrapper is intentional.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[cfg(target_vendor = "apple")]
pub fn render_to_metal_layer(
    scene: &PaintScene,
    metal_layer: objc_bridge::Id,
) -> Result<(), PaintMetalError> {
    let pixels = render(scene);
    unsafe { live_present::present_pixels_to_layer(metal_layer, &pixels) }
}

/// Present a scene whose URI-backed images are supplied by host resource policy.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[cfg(target_vendor = "apple")]
pub fn render_to_metal_layer_with_image_resolver(
    scene: &PaintScene,
    metal_layer: objc_bridge::Id,
    resolver: &dyn GpuImageResolver,
) -> Result<(), PaintMetalError> {
    let pixels = render_with_image_resolver(scene, resolver);
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
    use objc_bridge::{msg, Id, MTLOrigin, MTLRegion, MTLSize, NIL};
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
        BlendMode, FilterEffect, GradientKind, GradientStop, ImageSrc, PaintBase, PaintClip,
        PaintEllipse, PaintGradient, PaintImage, PaintInstruction, PaintLayer, PaintPath,
        PaintRect, PaintScene, PathCommand,
    };

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.7.0");
    }

    #[test]
    fn profile_declares_full_isolated_layer_execution() {
        let profile = profile();
        assert!(profile.supports_isolated_layers);
        assert!(profile.supports_layer_filters);
        assert!(profile.supports_layer_blend_modes);
    }

    #[test]
    fn shared_blend_modes_have_stable_shader_discriminants() {
        let modes = [
            GpuBlendMode::Normal,
            GpuBlendMode::Multiply,
            GpuBlendMode::Screen,
            GpuBlendMode::Overlay,
            GpuBlendMode::Darken,
            GpuBlendMode::Lighten,
            GpuBlendMode::ColorDodge,
            GpuBlendMode::ColorBurn,
            GpuBlendMode::HardLight,
            GpuBlendMode::SoftLight,
            GpuBlendMode::Difference,
            GpuBlendMode::Exclusion,
            GpuBlendMode::Hue,
            GpuBlendMode::Saturation,
            GpuBlendMode::Color,
            GpuBlendMode::Luminosity,
        ];
        for (expected, mode) in modes.into_iter().enumerate() {
            assert_eq!(metal_blend_mode(mode), expected as u32);
        }
    }

    #[test]
    fn shared_filters_keep_stable_shader_parameter_layouts() {
        let matrix = std::array::from_fn(|index| index as f32);
        let cases = [
            GpuFilter::Blur { radius: 2.0 },
            GpuFilter::DropShadow {
                dx: 1.0,
                dy: 2.0,
                blur: 3.0,
                color: GpuColor {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 0.4,
                },
            },
            GpuFilter::ColorMatrix { matrix },
            GpuFilter::Brightness { amount: 1.1 },
            GpuFilter::Contrast { amount: 1.2 },
            GpuFilter::Saturate { amount: 1.3 },
            GpuFilter::HueRotate {
                angle_degrees: 45.0,
            },
            GpuFilter::Invert { amount: 0.6 },
            GpuFilter::Opacity { amount: 0.7 },
        ];
        for (expected, filter) in cases.iter().enumerate() {
            assert_eq!(metal_filter_params(filter).kind, expected as u32);
        }
        let encoded = metal_filter_params(&GpuFilter::ColorMatrix { matrix });
        assert_eq!(encoded.matrix[0], [0.0, 1.0, 2.0, 3.0]);
        assert_eq!(encoded.bias, [4.0, 9.0, 14.0, 19.0]);
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

    #[test]
    fn parse_css_named_color() {
        let (r, g, b, a) = parse_hex_color("Aqua");
        assert_eq!((r, g, b, a), (0.0, 1.0, 1.0, 1.0));
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
        assert!(
            positions.is_empty(),
            "transparent rect should produce no vertices"
        );
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
        assert_eq!(
            positions.len(),
            expected,
            "ellipse fill+stroke vertex count"
        );
    }

    #[test]
    fn diamond_path_fill_generates_vertices() {
        // A diamond is 4-point closed polygon (5 points including close)
        let diamond = PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 50.0, y: 10.0 }, // top
                PathCommand::LineTo { x: 90.0, y: 50.0 }, // right
                PathCommand::LineTo { x: 50.0, y: 90.0 }, // bottom
                PathCommand::LineTo { x: 10.0, y: 50.0 }, // left
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
        assert!(
            positions.len() >= 18,
            "diamond fill should have at least 3 triangles"
        );
    }

    #[test]
    fn dashed_path_stroke_generates_separate_quads() {
        let dashed = PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 0.0, y: 10.0 },
                PathCommand::LineTo { x: 20.0, y: 10.0 },
            ],
            fill: None,
            fill_rule: None,
            stroke: Some("#ff0000".to_string()),
            stroke_width: Some(2.0),
            stroke_cap: None,
            stroke_join: None,
            stroke_dash: Some(vec![4.0, 4.0]),
            stroke_dash_offset: Some(2.0),
        });
        let mut positions = Vec::new();
        let mut colors = Vec::new();
        collect_geometry(&[dashed], &mut positions, &mut colors, 0);

        assert_eq!(positions.len(), 36, "three dash quads");
        assert_eq!(colors.len(), 72, "six colored vertices per dash");
        assert_eq!(positions[2], 2.0, "offset shortens the first dash");
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
        assert!(
            positions.is_empty(),
            "PaintText should not generate triangle vertices"
        );
        assert!(
            colors.is_empty(),
            "PaintText should not generate color vertices"
        );
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

    fn isolated_layer(
        children: Vec<PaintInstruction>,
        filters: Vec<FilterEffect>,
        blend_mode: BlendMode,
        opacity: f64,
    ) -> PaintInstruction {
        PaintInstruction::Layer(PaintLayer {
            base: PaintBase::default(),
            children,
            filters: Some(filters),
            blend_mode: Some(blend_mode),
            opacity: Some(opacity),
            transform: None,
        })
    }

    fn assert_channel_near(actual: u8, expected: u8, tolerance: u8, label: &str) {
        assert!(
            actual.abs_diff(expected) <= tolerance,
            "{label}: expected {expected} +/- {tolerance}, got {actual}"
        );
    }

    #[test]
    fn isolated_layer_opacity_is_applied_once_after_children_overlap() {
        let mut scene = PaintScene::new(12.0, 8.0);
        scene.instructions.push(isolated_layer(
            vec![
                PaintInstruction::Rect(PaintRect::filled(1.0, 1.0, 7.0, 6.0, "#ff0000")),
                PaintInstruction::Rect(PaintRect::filled(4.0, 1.0, 7.0, 6.0, "#ff0000")),
            ],
            vec![],
            BlendMode::Normal,
            0.5,
        ));

        let pixels = render(&scene);
        let single = pixels.pixel_at(2, 4);
        let overlap = pixels.pixel_at(6, 4);
        assert_eq!(
            single, overlap,
            "isolated overlap must not compound opacity"
        );
        assert_channel_near(single.0, 255, 1, "red");
        assert_channel_near(single.1, 128, 2, "green");
        assert_channel_near(single.2, 128, 2, "blue");
        assert_eq!(single.3, 255);
    }

    #[test]
    fn isolated_layer_multiply_blends_with_the_parent_surface() {
        let mut scene = PaintScene::new(8.0, 8.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 8.0, 8.0, "#808080",
            )));
        scene.instructions.push(isolated_layer(
            vec![PaintInstruction::Rect(PaintRect::filled(
                1.0, 1.0, 6.0, 6.0, "#ff0000",
            ))],
            vec![],
            BlendMode::Multiply,
            1.0,
        ));

        let pixel = render(&scene).pixel_at(4, 4);
        assert_channel_near(pixel.0, 128, 2, "red");
        assert_channel_near(pixel.1, 0, 1, "green");
        assert_channel_near(pixel.2, 0, 1, "blue");
        assert_eq!(pixel.3, 255);
    }

    #[test]
    fn isolated_layer_filters_execute_in_declared_order() {
        let make_scene = |filters| {
            let mut scene = PaintScene::new(6.0, 6.0);
            scene.instructions.push(isolated_layer(
                vec![PaintInstruction::Rect(PaintRect::filled(
                    1.0, 1.0, 4.0, 4.0, "#404040",
                ))],
                filters,
                BlendMode::Normal,
                1.0,
            ));
            scene
        };
        let brightness_then_invert = render(&make_scene(vec![
            FilterEffect::Brightness { amount: 2.0 },
            FilterEffect::Invert { amount: 1.0 },
        ]))
        .pixel_at(3, 3);
        let invert_then_brightness = render(&make_scene(vec![
            FilterEffect::Invert { amount: 1.0 },
            FilterEffect::Brightness { amount: 2.0 },
        ]))
        .pixel_at(3, 3);

        assert!(
            brightness_then_invert.0 < invert_then_brightness.0,
            "filter order must remain observable: {brightness_then_invert:?} vs {invert_then_brightness:?}"
        );
    }

    #[test]
    fn isolated_layer_blur_spreads_alpha_before_compositing() {
        let mut scene = PaintScene::new(9.0, 9.0);
        scene.instructions.push(isolated_layer(
            vec![PaintInstruction::Rect(PaintRect::filled(
                4.0, 4.0, 1.0, 1.0, "#000000",
            ))],
            vec![FilterEffect::Blur { radius: 1.0 }],
            BlendMode::Normal,
            1.0,
        ));

        let pixels = render(&scene);
        let neighbor = pixels.pixel_at(3, 4);
        assert!(neighbor.0 < 255, "blur should darken a neighboring pixel");
        assert!(neighbor.0 > 0, "blur should remain partially transparent");
        assert_eq!(neighbor.3, 255);
    }

    #[test]
    fn isolated_layer_filters_remain_inside_the_active_clip() {
        let mut scene = PaintScene::new(9.0, 9.0);
        scene.instructions.push(PaintInstruction::Clip(PaintClip {
            base: PaintBase::default(),
            x: 4.0,
            y: 3.0,
            width: 2.0,
            height: 3.0,
            children: vec![isolated_layer(
                vec![PaintInstruction::Rect(PaintRect::filled(
                    4.0, 4.0, 1.0, 1.0, "#000000",
                ))],
                vec![FilterEffect::Blur { radius: 1.0 }],
                BlendMode::Normal,
                1.0,
            )],
        }));

        let pixels = render(&scene);
        assert!(pixels.pixel_at(4, 4).0 < 255, "blur should render in clip");
        assert_eq!(
            pixels.pixel_at(3, 4),
            (255, 255, 255, 255),
            "filter must not escape its outer clip"
        );
    }

    #[test]
    fn nested_isolated_layers_composite_each_opacity_once() {
        let inner = isolated_layer(
            vec![PaintInstruction::Rect(PaintRect::filled(
                1.0, 1.0, 6.0, 6.0, "#ff0000",
            ))],
            vec![],
            BlendMode::Normal,
            0.5,
        );
        let mut scene = PaintScene::new(8.0, 8.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 8.0, 8.0, "#808080",
            )));
        scene
            .instructions
            .push(isolated_layer(vec![inner], vec![], BlendMode::Normal, 0.5));

        let pixel = render(&scene).pixel_at(4, 4);
        assert_channel_near(pixel.0, 160, 2, "red");
        assert_channel_near(pixel.1, 96, 2, "green");
        assert_channel_near(pixel.2, 96, 2, "blue");
        assert_eq!(pixel.3, 255);
    }

    #[test]
    fn render_decoded_image_pixels_with_scaling() {
        let source = PixelContainer::from_data(2, 1, vec![255, 0, 255, 255, 0, 255, 255, 255]);
        let mut scene = PaintScene::new(20.0, 10.0);
        scene.instructions.push(PaintInstruction::Image(PaintImage {
            base: PaintBase::default(),
            x: 2.0,
            y: 2.0,
            width: 16.0,
            height: 6.0,
            src: ImageSrc::Pixels(source),
            opacity: None,
        }));

        let pixels = render(&scene);
        assert_eq!(pixels.pixel_at(4, 4), (255, 0, 255, 255));
        assert_eq!(pixels.pixel_at(15, 4), (0, 255, 255, 255));
        assert_eq!(pixels.pixel_at(0, 0), (255, 255, 255, 255));
    }

    #[test]
    fn images_follow_mixed_painter_order_on_the_gpu() {
        let green = PixelContainer::from_data(1, 1, vec![0, 255, 0, 255]);
        let mut scene = PaintScene::new(12.0, 8.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 12.0, 8.0, "#ff0000",
            )));
        scene.instructions.push(PaintInstruction::Image(PaintImage {
            base: PaintBase::default(),
            x: 2.0,
            y: 1.0,
            width: 8.0,
            height: 6.0,
            src: ImageSrc::Pixels(green),
            opacity: None,
        }));
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                5.0, 0.0, 2.0, 8.0, "#0000ff",
            )));

        let pixels = render(&scene);

        assert_eq!(pixels.pixel_at(1, 4), (255, 0, 0, 255));
        assert_eq!(pixels.pixel_at(3, 4), (0, 255, 0, 255));
        assert_eq!(pixels.pixel_at(6, 4), (0, 0, 255, 255));
    }

    #[test]
    fn ordered_images_inherit_affine_clip_and_opacity() {
        use paint_instructions::{PaintClip, PaintGroup};

        let magenta = PixelContainer::from_data(1, 1, vec![255, 0, 255, 255]);
        let image = PaintInstruction::Image(PaintImage {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 6.0,
            src: ImageSrc::Pixels(magenta),
            opacity: None,
        });
        let mut scene = PaintScene::new(16.0, 10.0);
        scene.background = "#000000".to_string();
        scene.instructions.push(PaintInstruction::Group(PaintGroup {
            base: PaintBase::default(),
            children: vec![PaintInstruction::Clip(PaintClip {
                base: PaintBase::default(),
                x: 2.0,
                y: 1.0,
                width: 4.0,
                height: 4.0,
                children: vec![image],
            })],
            transform: Some([1.0, 0.0, 0.0, 1.0, 4.0, 2.0]),
            opacity: Some(0.5),
        }));

        let pixels = render(&scene);

        assert_eq!(pixels.pixel_at(5, 4), (0, 0, 0, 255));
        let blended = pixels.pixel_at(7, 4);
        assert!(blended.0 >= 127 && blended.0 <= 128);
        assert_eq!(blended.1, 0);
        assert!(blended.2 >= 127 && blended.2 <= 128);
        assert_eq!(pixels.pixel_at(11, 4), (0, 0, 0, 255));
    }

    #[test]
    fn host_resolver_supplies_uri_pixels_without_backend_fetching() {
        let mut scene = PaintScene::new(6.0, 4.0);
        scene.instructions.push(PaintInstruction::Image(PaintImage {
            base: PaintBase::default(),
            x: 1.0,
            y: 1.0,
            width: 4.0,
            height: 2.0,
            src: ImageSrc::Uri("fixture://cyan".to_string()),
            opacity: None,
        }));
        let resolver = |uri: &str| {
            assert_eq!(uri, "fixture://cyan");
            Ok(PixelContainer::from_data(1, 1, vec![0, 255, 255, 255]))
        };

        let pixels = render_with_image_resolver(&scene, &resolver);

        assert_eq!(pixels.pixel_at(3, 2), (0, 255, 255, 255));
        assert_eq!(pixels.pixel_at(0, 0), (255, 255, 255, 255));
    }

    #[test]
    fn shared_linear_gradient_plan_renders_as_a_metal_texture() {
        let mut scene = PaintScene::new(16.0, 4.0);
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase {
                    id: Some("spectrum".to_string()),
                    metadata: None,
                },
                kind: GradientKind::Linear {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 16.0,
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
            width: 16.0,
            height: 4.0,
            fill: Some("url(#spectrum)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let pixels = render(&scene);
        let left = pixels.pixel_at(1, 2);
        let right = pixels.pixel_at(14, 2);

        assert!(
            left.0 > left.2,
            "left gradient edge should be red: {left:?}"
        );
        assert!(
            right.2 > right.0,
            "right gradient edge should be blue: {right:?}"
        );
    }

    /// Render a blue filled ellipse and verify the centre pixel is blue.
    #[test]
    fn render_blue_ellipse() {
        let mut scene = PaintScene::new(100.0, 100.0);
        scene
            .instructions
            .push(PaintInstruction::Ellipse(PaintEllipse {
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
        assert_eq!(r, 0, "red channel at ellipse centre should be 0");
        assert_eq!(g, 0, "green channel at ellipse centre should be 0");
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
                PathCommand::MoveTo { x: 50.0, y: 10.0 }, // top
                PathCommand::LineTo { x: 90.0, y: 50.0 }, // right
                PathCommand::LineTo { x: 50.0, y: 90.0 }, // bottom
                PathCommand::LineTo { x: 10.0, y: 50.0 }, // left
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
        assert_eq!(b, 0, "yellow: b=0 at diamond centre");

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

        // Top-left module (0,0) is black → pixel (2, 2) should be black
        let (r, g, b, _a) = pixels.pixel_at(2, 2);
        assert_eq!(r, 0, "black module should have r=0");
        assert_eq!(g, 0, "black module should have g=0");
        assert_eq!(b, 0, "black module should have b=0");
    }
}
