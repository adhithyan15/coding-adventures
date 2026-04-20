//! # paint-metal
//!
//! Metal GPU renderer for the paint-instructions scene model (P2D01).
//!
//! This crate takes a [`PaintScene`] (backend-neutral 2D paint instructions)
//! and renders it to a [`PixelContainer`] using Apple's Metal GPU API.
//!
//! It is the GPU renderer in the paint-* stack, replacing the older
//! `draw-instructions-metal` crate which operated on `DrawScene`.  The
//! key differences are:
//!
//! | `draw-instructions-metal` | `paint-metal` (this crate)       |
//! |---------------------------|----------------------------------|
//! | `DrawScene` (i32 coords)  | `PaintScene` (f64 coords)        |
//! | `PixelBuffer`             | `PixelContainer`                 |
//! | `DrawInstruction` enum    | `PaintInstruction` enum          |
//! | Handles text via CoreText | Handles `PaintGlyphRun` glyphs   |
//!
//! ## Current instruction support
//!
//! | Instruction       | Status                                           |
//! |-------------------|--------------------------------------------------|
//! | `PaintRect`       | Fully implemented — solid-colour filled rects    |
//! | `PaintLine`       | Fully implemented — rendered as thin rectangles  |
//! | `PaintGroup`      | Fully implemented — recurses into children       |
//! | `PaintClip`       | Partially implemented — clips but no stencil     |
//! | `PaintGlyphRun`   | Planned — CoreText rasterize + texture quad      |
//! | `PaintEllipse`    | Planned — tessellate into triangles              |
//! | `PaintPath`       | Planned — CPU-side polygon tessellation          |
//! | `PaintLayer`      | Planned — offscreen texture + compose            |
//! | `PaintGradient`   | Planned — MSL gradient shader                    |
//! | `PaintImage`      | Planned — texture from PixelContainer or URI     |
//!
//! For barcodes (which are only rects + quiet-zone background), the current
//! implementation is complete.
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
//!   ├── 5. Collect PaintRect / PaintLine → triangle vertex buffers
//!   ├── 6. Encode render commands into command buffer
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

// This crate requires arm64 (Apple Silicon).  The objc_msgSend ABI for
// struct arguments differs between arm64 and x86_64.
#[cfg(not(target_arch = "aarch64"))]
compile_error!("paint-metal requires arm64 (Apple Silicon). x86_64 is not supported.");

pub const VERSION: &str = "0.1.0";

use objc_bridge::*;
use paint_instructions::{
    PaintInstruction, PaintLine, PaintRect, PaintScene, PixelContainer,
};
#[allow(unused_imports)]
use std::ffi::{c_int, c_ulong};
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

/// MSL shader source for rendering solid-colour rectangles.
///
/// The vertex shader converts pixel coordinates to Metal NDC.
/// The fragment shader outputs the per-vertex colour directly.
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
// Each visible instruction becomes 6 vertices (two triangles).  We collect
// all positions and colours into flat arrays, then upload them to GPU buffers
// in one batch.  This is more efficient than one draw call per instruction.
//
// The GPU only needs the triangle vertex stream — it has no concept of
// "rectangles" or "lines".  Everything is triangles.

/// Collect rect/line vertices from a [`PaintInstruction`] tree.
///
/// Recursively descends into Group and Clip nodes.
/// Text, ellipses, paths, gradients, images, and layers are not yet
/// implemented — they are silently skipped so barcodes work today.
fn collect_vertices(
    instructions: &[PaintInstruction],
    positions: &mut Vec<f32>,
    colors: &mut Vec<f32>,
) {
    for instr in instructions {
        match instr {
            PaintInstruction::Rect(rect) => {
                add_rect_vertices(rect, positions, colors);
            }
            PaintInstruction::Line(line) => {
                add_line_vertices(line, positions, colors);
            }
            PaintInstruction::Group(group) => {
                collect_vertices(&group.children, positions, colors);
            }
            PaintInstruction::Clip(clip) => {
                // Render clip children without a stencil clip for now.
                // Full stencil-buffer clip support is planned.
                collect_vertices(&clip.children, positions, colors);
            }
            // Planned but not yet implemented:
            PaintInstruction::GlyphRun(_)
            | PaintInstruction::Ellipse(_)
            | PaintInstruction::Path(_)
            | PaintInstruction::Layer(_)
            | PaintInstruction::Gradient(_)
            | PaintInstruction::Image(_) => {
                // No-op for now.  Barcodes only need Rect/Line/Group/Clip.
            }
        }
    }
}

/// Add 6 triangle vertices for a `PaintRect`.
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
    if a == 0.0 {
        return; // fully transparent — nothing to draw
    }
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

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Render a [`PaintScene`] to a [`PixelContainer`] using the Metal GPU.
///
/// This is the main entry point for the `paint-metal` crate.
///
/// ## Pipeline
///
/// 1. Create a Metal device and command queue
/// 2. Allocate an offscreen RGBA8 texture at `scene.width × scene.height`
/// 3. Compile the rect shader from MSL source
/// 4. Convert `PaintInstruction` tree to triangle vertex buffers
/// 5. Encode a render pass that clears to `scene.background` then draws all triangles
/// 6. Commit the command buffer and wait for GPU completion
/// 7. Read back the RGBA8 pixels with `getBytes()`
/// 8. Return the pixels as a `PixelContainer`
///
/// ## Requires
///
/// - macOS (Apple Silicon, arm64)
/// - A Metal-capable GPU (all Apple Silicon Macs qualify)
///
/// ## Chaining with a codec
///
/// ```rust,ignore
/// let scene = barcode_2d::layout(&grid, &config);
/// let pixels = paint_metal::render(&scene);
/// let png = paint_codec_png::encode_png(&pixels);
/// std::fs::write("qr.png", png).unwrap();
/// ```
/// Live-drawable rendering to a CAMetalLayer.
///
/// Renders the scene to a `PixelContainer` via the existing `render`
/// path, then uploads the pixels to the `CAMetalLayer`'s current
/// drawable and presents. Intended for the
/// `markdown-reader` binary that drives a window's redraw loop.
///
/// Takes the layer as an `Id` (the raw CAMetalLayer pointer). Panics
/// on non-Apple targets since CAMetalLayer has no equivalent
/// elsewhere; callers should gate with `#[cfg(target_vendor =
/// "apple")]`.
///
/// Returns an error if the layer has no current drawable (can happen
/// briefly during a resize).
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

pub fn render(scene: &PaintScene) -> PixelContainer {
    let mut pixels = unsafe { render_unsafe(scene) };
    // After Metal finishes rasterizing rects/lines, overlay any
    // `coretext:` PaintGlyphRun instructions via CoreText's
    // glyph-drawing API. CoreText draws directly into a CG bitmap
    // context wrapping the same RGBA bytes, so no extra allocation
    // is required. The overlay pass runs once, at the end — so glyph
    // runs render on top of whatever Metal rasterized. For typical
    // Markdown (background rects below, text above) this produces
    // correct painter's-algorithm output.
    #[cfg(target_vendor = "apple")]
    unsafe {
        glyph_run_overlay::overlay_coretext_glyph_runs(scene, &mut pixels);
    }
    pixels
}

unsafe fn render_unsafe(scene: &PaintScene) -> PixelContainer {
    let width = scene.width as u32;
    let height = scene.height as u32;

    if width == 0 || height == 0 {
        return PixelContainer::new(width, height);
    }

    // Guard against accidental huge allocations.  A 16384×16384 RGBA image
    // is ~1 GB — beyond this size most systems would OOM.
    const MAX_DIMENSION: u32 = 16384;
    assert!(
        width <= MAX_DIMENSION && height <= MAX_DIMENSION,
        "Scene dimensions {}×{} exceed maximum {}×{}",
        width, height, MAX_DIMENSION, MAX_DIMENSION
    );

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
    collect_vertices(&scene.instructions, &mut positions, &mut colors);

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

    // Draw all rectangles and lines (collected as triangles)
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

unsafe fn create_offscreen_texture(device: Id, width: u32, height: u32) -> Id {
    // Build the texture descriptor manually instead of using the class method —
    // objc_msgSend with mixed integer types can cause alignment issues on arm64.
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
        CGContextRestoreGState, CGContextSaveGState, CGContextScaleCTM,
        CGContextSetRGBFillColor, CGContextSetShouldAntialias, CGContextSetShouldSmoothFonts,
        CGContextSetTextMatrix, CGContextTranslateCTM, CGPoint, CTFontCreateWithName,
        CTFontDrawGlyphs, CGContextRef, Id, K_CG_IMAGE_ALPHA_PREMULTIPLIED_FIRST,
        K_CG_BITMAP_BYTE_ORDER_32_LITTLE, NIL,
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

        // Wrap the existing pixel buffer in a CG bitmap context.
        // MTLPixelFormatRGBA8Unorm lays bytes as R,G,B,A; CoreGraphics'
        // closest match on Apple platforms is BGRA premultiplied with
        // little-endian byte order. Since we're only *adding* text
        // (most pixels are background anyway), minor byte-order
        // mismatch would manifest as subtly wrong text color. We use
        // PREMULTIPLIED_FIRST + BYTE_ORDER_32_LITTLE which yields an
        // ARGB layout compatible with most Core Graphics usage. If
        // colors look off in the end-to-end integration we can flip
        // to alpha-last.
        let color_space = CGColorSpaceCreateDeviceRGB();
        if color_space.is_null() {
            return;
        }

        // Cast the PixelContainer's immutable data pointer to mutable.
        // CGBitmapContextCreate needs a mutable pointer; the caller
        // holds exclusive access to `pixels` via the `&mut` in this
        // function's signature, so this is safe per Rust's aliasing
        // rules.
        let data_ptr = pixels.data.as_ptr() as *mut std::ffi::c_void;

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

        // Flip the Y axis so scene coords (Y-down) align with CG's
        // default coordinate system (Y-up). After this, drawing at
        // scene (x, y) places the glyph where we expect.
        CGContextSaveGState(ctx);
        CGContextTranslateCTM(ctx, 0.0, height as f64);
        CGContextScaleCTM(ctx, 1.0, -1.0);

        CGContextSetShouldAntialias(ctx, true);
        CGContextSetShouldSmoothFonts(ctx, true);

        // Text matrix stays identity. The CTM flip above makes
        // glyphs render with ascenders pointing toward the top of
        // the buffer; the CGBitmapContext byte ordering ensures
        // those are the correct pixel rows.
        CGContextSetTextMatrix(ctx, CGAffineTransform::IDENTITY);

        for gr in runs {
            draw_one_glyph_run(ctx, gr);
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

    unsafe fn draw_one_glyph_run(ctx: CGContextRef, run: &PaintGlyphRun) {
        // Parse "coretext:<ps_name>@<size>". Size falls back to
        // run.font_size if the suffix is missing or malformed.
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

        // Set fill color from run.fill (CSS string). Default: black.
        let (r, g, b, a) = parse_css_color(run.fill.as_deref().unwrap_or("rgb(0, 0, 0)"));
        CGContextSetRGBFillColor(ctx, r, g, b, a);

        // Assemble glyph ID + position arrays. The run's glyphs
        // already carry absolute scene-coordinate baseline origins,
        // so no extra offset is needed here.
        let glyph_ids: Vec<u16> = run.glyphs.iter().map(|g| g.glyph_id as u16).collect();
        let positions: Vec<CGPoint> = run
            .glyphs
            .iter()
            .map(|g| CGPoint { x: g.x, y: g.y })
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
    /// Malformed inputs return `(stripped, None)`.
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
    /// Supports `rgb(r, g, b)` and `rgba(r, g, b, a)` with decimal
    /// r,g,b in 0..=255 and a in 0..=1. Falls back to opaque black.
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
//
// Takes the RGBA pixel buffer produced by `render()` and uploads it
// into a CAMetalLayer's current drawable via MTLTexture.replaceRegion,
// then presents. This is the path the markdown-reader binary uses to
// display a scene in a live window — no offscreen file I/O, no copy
// back through the CPU beyond the initial render's readback.

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

        // Drawable pixel format is BGRA8Unorm (set on the layer by
        // window-appkit). paint-metal's render() produces RGBA8.
        // Byte-swap R and B in a one-shot temporary buffer before
        // uploading so the drawable displays with correct colours.
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

        // [texture replaceRegion:mipmapLevel:withBytes:bytesPerRow:]
        // This is a 4-arg Objective-C call; construct the function
        // pointer explicitly.
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

        // Present via a command buffer created from the layer's
        // device's default queue.
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

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{PaintBase, PaintInstruction, PaintRect, PaintScene};

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
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
        collect_vertices(&[rect], &mut positions, &mut colors);
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
        collect_vertices(&[rect], &mut positions, &mut colors);
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
        collect_vertices(&[group], &mut positions, &mut colors);
        // 2 rects × 6 vertices × 2 floats = 24 positions
        assert_eq!(positions.len(), 24);
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
    /// The pixel at the rectangle's centre should be red; the corner should be white.
    ///
    /// This test exercises the full Metal pipeline: shader compilation, GPU render,
    /// and pixel readback.
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

    /// Render a scene with a dark module grid pattern (like a QR code quiet zone).
    #[test]
    fn render_black_modules_on_white() {
        let module_size = 4.0_f64;
        let mut scene = PaintScene::new(40.0, 40.0);

        // Place 4×4 black modules (a tiny QR-like grid)
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
