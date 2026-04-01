//! # draw-instructions-metal
//!
//! Metal GPU renderer for the draw-instructions scene model.
//!
//! This crate takes a `DrawScene` (backend-neutral 2D instructions) and
//! renders it to a `PixelBuffer` using Apple's Metal GPU API.  It is the
//! GPU equivalent of `draw-instructions-svg` — same input, different output.
//!
//! ## How GPU rendering works
//!
//! GPU rendering is fundamentally different from SVG serialization:
//!
//! | SVG renderer                  | Metal renderer                      |
//! |-------------------------------|-------------------------------------|
//! | Produces text (XML markup)    | Produces pixels (RGBA byte buffer)  |
//! | Runs on CPU                   | Runs on GPU                         |
//! | Output is resolution-free     | Output is at a fixed resolution     |
//! | Zero dependencies             | Requires Metal-capable GPU          |
//!
//! ## The Metal pipeline
//!
//! ```text
//! DrawScene
//!   │
//!   ├── 1. Create Metal device (MTLCreateSystemDefaultDevice)
//!   ├── 2. Create offscreen texture (RGBA8Unorm)
//!   ├── 3. Compile MSL shaders (vertex + fragment)
//!   ├── 4. Build render pipeline state
//!   ├── 5. Convert DrawInstructions → vertex buffers
//!   ├── 6. Encode draw commands into command buffer
//!   ├── 7. Commit and wait for GPU completion
//!   └── 8. Read back pixels → PixelBuffer
//! ```
//!
//! ## Coordinate system
//!
//! The draw-instructions IR uses a **top-left origin** with Y increasing
//! downward (same as SVG, HTML Canvas, and most 2D graphics).
//!
//! Metal's normalized device coordinates (NDC) use a **center origin**
//! with Y increasing upward, ranging from -1 to +1:
//!
//! ```text
//!  IR coordinates:          Metal NDC:
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

pub const VERSION: &str = "0.1.0";

use draw_instructions::{
    DrawInstruction, DrawLineInstruction, DrawRectInstruction, DrawScene, DrawTextInstruction,
    Renderer,
};
use draw_instructions_pixels::PixelBuffer;
use objc_bridge::*;
#[allow(unused_imports)]
use std::ffi::{c_int, c_ulong};
use std::ptr;

// ---------------------------------------------------------------------------
// Metal Shading Language (MSL) source code
// ---------------------------------------------------------------------------
//
// These shaders run on the GPU.  They are compiled at runtime from source
// strings using Metal's newLibraryWithSource:options:error: method.
//
// MSL is a C++-like language designed for GPU computation.  Each shader
// program has a vertex function (processes vertices) and a fragment
// function (computes pixel colors).

/// MSL shader source for rendering solid-color rectangles.
///
/// The vertex shader transforms pixel coordinates to Metal NDC.
/// The fragment shader outputs the per-vertex color directly.
///
/// ## How vertex data flows through the GPU
///
/// ```text
/// CPU (vertex buffer)     GPU vertex shader      GPU rasterizer      GPU fragment shader
/// ┌──────────────────┐   ┌─────────────────┐   ┌───────────────┐   ┌──────────────────┐
/// │ position (float2)│──▶│ pixel → NDC     │──▶│ interpolate   │──▶│ output color     │
/// │ color (float4)   │   │ pass color      │   │ between verts │   │ from interpolated│
/// └──────────────────┘   └─────────────────┘   └───────────────┘   └──────────────────┘
/// ```
const RECT_SHADER_SOURCE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct RectVertexIn {
    float2 position [[attribute(0)]];
    float4 color    [[attribute(1)]];
};

struct RectVertexOut {
    float4 position [[position]];
    float4 color;
};

vertex RectVertexOut rect_vertex(
    uint vid [[vertex_id]],
    const device float2* positions [[buffer(0)]],
    const device float4* colors    [[buffer(1)]],
    constant float2& viewport     [[buffer(2)]]
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

/// MSL shader source for rendering textured quads (used for text glyphs).
///
/// The glyph bitmap is uploaded to a texture.  The fragment shader samples
/// the texture's red channel as an alpha mask and combines it with the
/// text color.
const TEXT_SHADER_SOURCE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct TextVertexOut {
    float4 position [[position]];
    float2 texcoord;
};

vertex TextVertexOut text_vertex(
    uint vid [[vertex_id]],
    const device float2* positions [[buffer(0)]],
    const device float2* texcoords [[buffer(1)]],
    constant float2& viewport     [[buffer(2)]]
) {
    TextVertexOut out;
    float2 px = positions[vid];
    out.position = float4(
        (px.x / viewport.x) * 2.0 - 1.0,
        1.0 - (px.y / viewport.y) * 2.0,
        0.0,
        1.0
    );
    out.texcoord = texcoords[vid];
    return out;
}

fragment float4 text_fragment(
    TextVertexOut in [[stage_in]],
    texture2d<float> glyph_tex [[texture(0)]],
    constant float4& text_color [[buffer(0)]]
) {
    constexpr sampler s(mag_filter::linear, min_filter::linear);
    float alpha = glyph_tex.sample(s, in.texcoord).r;
    return float4(text_color.rgb, text_color.a * alpha);
}
"#;

// ---------------------------------------------------------------------------
// Color parsing
// ---------------------------------------------------------------------------

/// Parse a hex color string to RGBA floats (0.0–1.0).
///
/// Supports:
/// - `"#rrggbb"` → (r, g, b, 1.0)
/// - `"#rrggbbaa"` → (r, g, b, a)
/// - `"#rgb"` → expanded to `#rrggbb`
///
/// Returns (0.0, 0.0, 0.0, 1.0) for invalid input.
fn parse_hex_color(hex: &str) -> (f64, f64, f64, f64) {
    let hex = hex.trim_start_matches('#');
    let hex = if hex.len() == 3 {
        // Expand #rgb → #rrggbb
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

// ---------------------------------------------------------------------------
// Vertex generation
// ---------------------------------------------------------------------------
//
// Each draw instruction is converted to vertices.  A rectangle becomes
// two triangles (6 vertices).  We collect all positions and colors into
// flat arrays, then upload them to GPU buffers.

/// Collect rectangle vertices from a draw instruction tree.
///
/// Each rectangle becomes 6 vertices (2 triangles):
///
/// ```text
/// (x, y) ─────── (x+w, y)
///   │  ╲              │
///   │    ╲            │
///   │      ╲          │
///   │        ╲        │
///   │          ╲      │
///   │            ╲    │
///   │              ╲  │
/// (x, y+h) ──── (x+w, y+h)
///
/// Triangle 1: top-left, top-right, bottom-left
/// Triangle 2: top-right, bottom-right, bottom-left
/// ```
fn collect_rect_vertices(
    instructions: &[DrawInstruction],
    positions: &mut Vec<f32>,
    colors: &mut Vec<f32>,
) {
    for instruction in instructions {
        match instruction {
            DrawInstruction::Rect(rect) => {
                add_rect_vertices(rect, positions, colors);
            }
            DrawInstruction::Group(group) => {
                collect_rect_vertices(&group.children, positions, colors);
            }
            DrawInstruction::Clip(clip) => {
                // For now, render clip children without clipping.
                // Full clip support requires stencil buffer work.
                collect_rect_vertices(&clip.children, positions, colors);
            }
            DrawInstruction::Line(line) => {
                // Render lines as thin rectangles.
                add_line_vertices(line, positions, colors);
            }
            DrawInstruction::Text(_) => {
                // Text is handled separately via CoreText rasterization.
            }
        }
    }
}

fn add_rect_vertices(rect: &DrawRectInstruction, positions: &mut Vec<f32>, colors: &mut Vec<f32>) {
    let x = rect.x as f32;
    let y = rect.y as f32;
    let w = rect.width as f32;
    let h = rect.height as f32;
    let (r, g, b, a) = parse_hex_color(&rect.fill);
    let (r, g, b, a) = (r as f32, g as f32, b as f32, a as f32);

    // Triangle 1: top-left, top-right, bottom-left
    positions.extend_from_slice(&[x, y, x + w, y, x, y + h]);
    colors.extend_from_slice(&[r, g, b, a, r, g, b, a, r, g, b, a]);

    // Triangle 2: top-right, bottom-right, bottom-left
    positions.extend_from_slice(&[x + w, y, x + w, y + h, x, y + h]);
    colors.extend_from_slice(&[r, g, b, a, r, g, b, a, r, g, b, a]);
}

/// Render a line as a thin rectangle perpendicular to the line direction.
fn add_line_vertices(line: &DrawLineInstruction, positions: &mut Vec<f32>, colors: &mut Vec<f32>) {
    let x1 = line.x1 as f32;
    let y1 = line.y1 as f32;
    let x2 = line.x2 as f32;
    let y2 = line.y2 as f32;
    let half_w = (line.stroke_width as f32) / 2.0;
    let (r, g, b, a) = parse_hex_color(&line.stroke);
    let (r, g, b, a) = (r as f32, g as f32, b as f32, a as f32);

    // Compute perpendicular offset
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 {
        return; // degenerate line
    }
    let nx = -dy / len * half_w;
    let ny = dx / len * half_w;

    // Four corners of the line rectangle
    let p0x = x1 + nx;
    let p0y = y1 + ny;
    let p1x = x1 - nx;
    let p1y = y1 - ny;
    let p2x = x2 + nx;
    let p2y = y2 + ny;
    let p3x = x2 - nx;
    let p3y = y2 - ny;

    // Two triangles
    positions.extend_from_slice(&[p0x, p0y, p2x, p2y, p1x, p1y]);
    colors.extend_from_slice(&[r, g, b, a, r, g, b, a, r, g, b, a]);
    positions.extend_from_slice(&[p2x, p2y, p3x, p3y, p1x, p1y]);
    colors.extend_from_slice(&[r, g, b, a, r, g, b, a, r, g, b, a]);
}

/// Collect text instructions from the instruction tree.
fn collect_text_instructions(instructions: &[DrawInstruction], texts: &mut Vec<DrawTextInstruction>) {
    for instruction in instructions {
        match instruction {
            DrawInstruction::Text(text) => {
                texts.push(text.clone());
            }
            DrawInstruction::Group(group) => {
                collect_text_instructions(&group.children, texts);
            }
            DrawInstruction::Clip(clip) => {
                collect_text_instructions(&clip.children, texts);
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// CoreText text rasterization
// ---------------------------------------------------------------------------

/// Rasterize a text instruction to a bitmap using CoreText.
///
/// Returns the bitmap data (grayscale, 1 byte per pixel), width, and height.
/// The bitmap uses CoreGraphics' coordinate system (bottom-left origin),
/// but we account for this when computing the texture quad's Y coordinates.
fn rasterize_text(text: &DrawTextInstruction) -> Option<(Vec<u8>, u32, u32)> {
    unsafe {
        // Create the font
        let font_name = nsstring(&text.font_family);
        let font = CTFontCreateWithName(font_name, text.font_size as f64, ptr::null());
        CFRelease(font_name);
        if font.is_null() {
            return None;
        }

        // Create attributed string with the font
        let text_str = cfstring(&text.value);

        let keys: [*const std::ffi::c_void; 1] = [kCTFontAttributeName as *const _];
        let values: [*const std::ffi::c_void; 1] = [font as *const _];
        let attributes = CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &kCFTypeDictionaryKeyCallBacks as *const _ as *const _,
            &kCFTypeDictionaryValueCallBacks as *const _ as *const _,
        );

        let attr_string = CFAttributedStringCreate(ptr::null(), text_str, attributes);
        CFRelease(text_str);
        CFRelease(attributes);

        // Create a CTLine for measurement and drawing
        let line = CTLineCreateWithAttributedString(attr_string);
        CFRelease(attr_string);

        // Get typographic bounds
        let mut ascent: f64 = 0.0;
        let mut descent: f64 = 0.0;
        let mut leading: f64 = 0.0;
        let width = CTLineGetTypographicBounds(line, &mut ascent, &mut descent, &mut leading);

        let bmp_width = (width.ceil() as u32).max(1);
        let bmp_height = ((ascent + descent + leading).ceil() as u32).max(1);
        let bytes_per_row = bmp_width as usize;

        // Create a grayscale bitmap context
        // We use a single-channel (gray) context — each byte is an alpha value
        let mut bitmap_data = vec![0u8; bytes_per_row * bmp_height as usize];
        let color_space = CGColorSpaceCreateDeviceRGB();

        // Use an RGBA context for compatibility, then extract just the alpha
        let rgba_bytes_per_row = bmp_width as usize * 4;
        let mut rgba_data = vec![0u8; rgba_bytes_per_row * bmp_height as usize];

        let context = CGBitmapContextCreate(
            rgba_data.as_mut_ptr() as *mut _,
            bmp_width as usize,
            bmp_height as usize,
            8,
            rgba_bytes_per_row,
            color_space,
            K_CG_IMAGE_ALPHA_PREMULTIPLIED_LAST | K_CG_BITMAP_BYTE_ORDER_32_BIG,
        );
        CGColorSpaceRelease(color_space);

        if context.is_null() {
            CFRelease(line);
            CFRelease(font);
            return None;
        }

        // Set text color to white (we'll use the alpha channel as a mask)
        CGContextSetRGBFillColor(context, 1.0, 1.0, 1.0, 1.0);

        // Position text at baseline
        // CoreGraphics has Y-up, so baseline is at descent from bottom
        CGContextSetTextPosition(context, 0.0, descent);

        // Draw the text
        CTLineDraw(line, context);

        // Extract alpha channel as grayscale mask
        for i in 0..(bmp_width * bmp_height) as usize {
            // Use the red channel (since text is white, R=G=B=A)
            bitmap_data[i] = rgba_data[i * 4];
        }

        CGContextRelease(context);
        CFRelease(line);
        CFRelease(font);

        Some((bitmap_data, bmp_width, bmp_height))
    }
}

// ---------------------------------------------------------------------------
// MetalRenderer
// ---------------------------------------------------------------------------

/// Metal GPU renderer for draw-instructions scenes.
///
/// Implements the `Renderer<PixelBuffer>` trait from `draw-instructions`.
/// Each call to `render` creates a Metal device, compiles shaders, renders
/// the scene to an offscreen texture, and reads back the pixels.
///
/// This is intentionally stateless — each render call is self-contained.
/// For production use with many renders, you'd want to cache the device
/// and pipeline state.  But for the educational/barcode use case, the
/// simplicity of stateless rendering is worth the setup cost.
pub struct MetalRenderer;

impl Renderer<PixelBuffer> for MetalRenderer {
    fn render(&self, scene: &DrawScene) -> PixelBuffer {
        render_metal(scene)
    }
}

/// Render a DrawScene to a PixelBuffer using Metal.
///
/// This is the main entry point.  It performs the complete Metal rendering
/// pipeline and returns the result as CPU-accessible pixels.
pub fn render_metal(scene: &DrawScene) -> PixelBuffer {
    unsafe { render_metal_unsafe(scene) }
}

unsafe fn render_metal_unsafe(scene: &DrawScene) -> PixelBuffer {
    let width = scene.width as u32;
    let height = scene.height as u32;

    if width == 0 || height == 0 {
        return PixelBuffer::new(width, height);
    }

    // Step 1: Create Metal device
    let device = MTLCreateSystemDefaultDevice();
    assert!(!device.is_null(), "No Metal-capable GPU found");

    // Step 2: Create command queue
    let command_queue = msg_send_id(device, "newCommandQueue");
    assert!(!command_queue.is_null(), "Failed to create command queue");

    // Step 3: Create offscreen texture
    let texture = create_offscreen_texture(device, width, height);

    // Step 4: Compile shaders and create pipeline states
    let rect_pipeline = create_rect_pipeline(device);
    let text_pipeline = create_text_pipeline(device);

    // Step 5: Generate vertices from draw instructions
    let mut positions: Vec<f32> = Vec::new();
    let mut colors: Vec<f32> = Vec::new();
    collect_rect_vertices(&scene.instructions, &mut positions, &mut colors);

    let mut texts: Vec<DrawTextInstruction> = Vec::new();
    collect_text_instructions(&scene.instructions, &mut texts);

    // Step 6: Create render pass descriptor
    // Must use the class factory method, not alloc/init
    let pass_desc_class = class("MTLRenderPassDescriptor");
    let pass_desc: Id = msg_send_class(pass_desc_class, "renderPassDescriptor");

    // Get colorAttachments[0]
    let color_attachments = msg_send_id(pass_desc, "colorAttachments");
    let attachment0: Id = msg!(color_attachments, "objectAtIndexedSubscript:", 0usize);

    // Set texture, load action (clear), store action (store), clear color
    let (cr, cg, cb, ca) = parse_hex_color(&scene.background);
    // clear_color components are used directly in set_clear_color call below

    msg!(attachment0, "setTexture:", texture);
    msg!(attachment0, "setLoadAction:", MTL_LOAD_ACTION_CLEAR as usize);
    msg!(attachment0, "setStoreAction:", MTL_STORE_ACTION_STORE as usize);
    // MTLClearColor is a struct of 4 doubles.  On arm64, a Homogeneous
    // Floating-point Aggregate (HFA) of 4 doubles is passed in d0-d3.
    let clear_color = MTLClearColor { red: cr, green: cg, blue: cb, alpha: ca };
    let set_clear_color: unsafe extern "C" fn(Id, Sel, MTLClearColor) =
        std::mem::transmute(objc_msgSend as *const ());
    set_clear_color(attachment0, sel("setClearColor:"), clear_color);

    // Step 7: Create command buffer and render encoder
    let command_buffer = msg_send_id(command_queue, "commandBuffer");
    let encoder: Id = msg!(command_buffer, "renderCommandEncoderWithDescriptor:", pass_desc);

    let viewport_size: [f32; 2] = [width as f32, height as f32];

    // Draw rectangles (and lines converted to rects)
    if !positions.is_empty() {
        let vertex_count = positions.len() / 2;

        // Create vertex buffers
        let pos_buffer = create_buffer(device, &positions);
        let color_buffer = create_buffer(device, &colors);

        msg!(encoder, "setRenderPipelineState:", rect_pipeline);

        // setVertexBuffer:offset:atIndex:
        msg!(encoder, "setVertexBuffer:offset:atIndex:", pos_buffer, 0usize, 0usize);
        msg!(encoder, "setVertexBuffer:offset:atIndex:", color_buffer, 0usize, 1usize);

        // setVertexBytes for viewport
        let vp_ptr = viewport_size.as_ptr() as *const std::ffi::c_void as Id;
        msg!(encoder, "setVertexBytes:length:atIndex:", vp_ptr, 8usize, 2usize);

        // Draw
        msg!(encoder, "drawPrimitives:vertexStart:vertexCount:", MTL_PRIMITIVE_TYPE_TRIANGLE as usize, 0usize, vertex_count);

        release(pos_buffer);
        release(color_buffer);
    }

    // Draw text
    for text_instr in &texts {
        render_text_instruction(
            device,
            encoder,
            text_pipeline,
            texture,
            text_instr,
            &viewport_size,
        );
    }

    // End encoding, commit, wait
    msg!(encoder, "endEncoding");
    msg!(command_buffer, "commit");
    msg!(command_buffer, "waitUntilCompleted");

    // Step 8: Read back pixels
    let pixel_buffer = read_back_pixels(texture, width, height);

    // Clean up
    // Only release objects we own (created with new*/alloc/copy).
    // pass_desc was returned by a factory method — it's autoreleased.
    // command_buffer was returned by commandBuffer — also autoreleased.
    release(texture);
    release(rect_pipeline);
    release(text_pipeline);
    release(command_queue);
    release(device);

    pixel_buffer
}

// ---------------------------------------------------------------------------
// Metal helper functions
// ---------------------------------------------------------------------------

unsafe fn create_offscreen_texture(device: Id, width: u32, height: u32) -> Id {
    // Create texture descriptor manually instead of using the class method,
    // because objc_msgSend with mixed integer types can cause alignment issues.
    let desc = alloc_init("MTLTextureDescriptor");

    // MTLPixelFormatRGBA8Unorm = 70
    msg!(desc, "setPixelFormat:", MTL_PIXEL_FORMAT_RGBA8_UNORM as usize);
    msg!(desc, "setWidth:", width as usize);
    msg!(desc, "setHeight:", height as usize);

    // MTLTextureType2D = 2
    msg!(desc, "setTextureType:", MTL_TEXTURE_TYPE_2D as usize);

    // Set usage flags: render target + shader read
    let usage = MTL_TEXTURE_USAGE_RENDER_TARGET | MTL_TEXTURE_USAGE_SHADER_READ;
    msg!(desc, "setUsage:", usage as usize);

    let texture: Id = msg!(device, "newTextureWithDescriptor:", desc);
    release(desc);
    assert!(!texture.is_null(), "Failed to create offscreen texture");
    texture
}

unsafe fn compile_shader_library(device: Id, source: &str) -> Id {
    let source_ns = nsstring(source);
    let options: Id = ptr::null_mut(); // nil options = defaults

    // newLibraryWithSource:options:error:
    let mut error: Id = ptr::null_mut();
    let error_ptr = &mut error as *mut Id;
    let library: Id = msg!(device, "newLibraryWithSource:options:error:", source_ns, options, error_ptr);
    CFRelease(source_ns);

    if library.is_null() {
        // Try to get error description for debugging
        if !error.is_null() {
            let desc = msg_send_id(error, "localizedDescription");
            if !desc.is_null() {
                // In a real app we'd extract the string, but for now just panic
                panic!("Metal shader compilation failed (use RUST_BACKTRACE=1 for details)");
            }
        }
        panic!("Metal shader compilation failed");
    }

    library
}

unsafe fn setup_pipeline_color_attachment(desc: Id) {
    let attachments = msg_send_id(desc, "colorAttachments");
    // Use objectAtIndexedSubscript: to get attachment 0
    let att0: Id = msg!(attachments, "objectAtIndexedSubscript:", 0usize);
    msg!(att0, "setPixelFormat:", MTL_PIXEL_FORMAT_RGBA8_UNORM as usize);

    // Enable alpha blending for text rendering over background
    msg!(att0, "setBlendingEnabled:", 1usize);
    // source * sourceAlpha + dest * (1 - sourceAlpha)
    msg!(att0, "setSourceRGBBlendFactor:", 4usize); // sourceAlpha
    msg!(att0, "setDestinationRGBBlendFactor:", 5usize); // oneMinusSourceAlpha
    msg!(att0, "setSourceAlphaBlendFactor:", 1usize); // one
    msg!(att0, "setDestinationAlphaBlendFactor:", 5usize); // oneMinusSourceAlpha
}

unsafe fn create_rect_pipeline(device: Id) -> Id {
    let library = compile_shader_library(device, RECT_SHADER_SOURCE);

    let vertex_name = nsstring("rect_vertex");
    let fragment_name = nsstring("rect_fragment");

    let vertex_fn: Id = msg!(library, "newFunctionWithName:", vertex_name);
    let fragment_fn: Id = msg!(library, "newFunctionWithName:", fragment_name);
    CFRelease(vertex_name);
    CFRelease(fragment_name);

    assert!(!vertex_fn.is_null(), "rect_vertex function not found in shader library");
    assert!(!fragment_fn.is_null(), "rect_fragment function not found in shader library");

    let desc = alloc_init("MTLRenderPipelineDescriptor");
    msg!(desc, "setVertexFunction:", vertex_fn);
    msg!(desc, "setFragmentFunction:", fragment_fn);

    setup_pipeline_color_attachment(desc);

    let mut error: Id = ptr::null_mut();
    let error_ptr = &mut error as *mut Id;
    let pipeline: Id = msg!(device, "newRenderPipelineStateWithDescriptor:error:", desc, error_ptr);

    release(vertex_fn);
    release(fragment_fn);
    release(library);
    release(desc);

    assert!(!pipeline.is_null(), "Failed to create rect render pipeline state");
    pipeline
}

unsafe fn create_text_pipeline(device: Id) -> Id {
    let library = compile_shader_library(device, TEXT_SHADER_SOURCE);

    let vertex_name = nsstring("text_vertex");
    let fragment_name = nsstring("text_fragment");

    let vertex_fn: Id = msg!(library, "newFunctionWithName:", vertex_name);
    let fragment_fn: Id = msg!(library, "newFunctionWithName:", fragment_name);
    CFRelease(vertex_name);
    CFRelease(fragment_name);

    assert!(!vertex_fn.is_null(), "text_vertex function not found");
    assert!(!fragment_fn.is_null(), "text_fragment function not found");

    let desc = alloc_init("MTLRenderPipelineDescriptor");
    msg!(desc, "setVertexFunction:", vertex_fn);
    msg!(desc, "setFragmentFunction:", fragment_fn);

    setup_pipeline_color_attachment(desc);

    let mut error: Id = ptr::null_mut();
    let error_ptr = &mut error as *mut Id;
    let pipeline: Id = msg!(device, "newRenderPipelineStateWithDescriptor:error:", desc, error_ptr);

    release(vertex_fn);
    release(fragment_fn);
    release(library);
    release(desc);

    assert!(!pipeline.is_null(), "Failed to create text render pipeline state");
    pipeline
}

unsafe fn create_buffer(device: Id, data: &[f32]) -> Id {
    let byte_len = data.len() * std::mem::size_of::<f32>();
    // MTLResourceStorageModeShared = 0
    let buffer: Id = msg!(device, "newBufferWithBytes:length:options:", data.as_ptr() as Id, byte_len as usize, 0usize);
    assert!(!buffer.is_null(), "Failed to create Metal buffer");
    buffer
}

unsafe fn create_texture_from_grayscale(device: Id, data: &[u8], width: u32, height: u32) -> Id {
    // Create a single-channel (r8Unorm) texture for glyph data
    let desc = alloc_init("MTLTextureDescriptor");

    // MTLPixelFormatR8Unorm = 10
    msg!(desc, "setPixelFormat:", 10 as usize);
    msg!(desc, "setWidth:", width as usize);
    msg!(desc, "setHeight:", height as usize);
    msg!(desc, "setTextureType:", MTL_TEXTURE_TYPE_2D as usize);
    msg!(desc, "setUsage:", MTL_TEXTURE_USAGE_SHADER_READ as usize);

    let texture: Id = msg!(device, "newTextureWithDescriptor:", desc);
    release(desc);

    if !texture.is_null() {
        // Upload data
        let region = MTLRegion {
            origin: MTLOrigin { x: 0, y: 0, z: 0 },
            size: MTLSize {
                width: width as c_ulong,
                height: height as c_ulong,
                depth: 1,
            },
        };

        // MTLRegion is 48 bytes — passed indirectly on arm64
        let replace_fn: unsafe extern "C" fn(
            Id, Sel,
            MTLRegion,     // region
            usize,         // mipmapLevel
            *const u8,     // bytes
            usize,         // bytesPerRow
        ) = std::mem::transmute(objc_msgSend as *const ());
        replace_fn(
            texture, sel("replaceRegion:mipmapLevel:withBytes:bytesPerRow:"),
            region, 0, data.as_ptr(), width as usize,
        );
    }

    texture
}

unsafe fn render_text_instruction(
    device: Id,
    encoder: Id,
    text_pipeline: Id,
    _target_texture: Id,
    text: &DrawTextInstruction,
    viewport_size: &[f32; 2],
) {
    // Rasterize text to bitmap via CoreText
    let (bitmap_data, bmp_w, bmp_h) = match rasterize_text(text) {
        Some(result) => result,
        None => return,
    };

    // Upload bitmap to Metal texture
    let glyph_texture = create_texture_from_grayscale(device, &bitmap_data, bmp_w, bmp_h);
    if glyph_texture.is_null() {
        return;
    }

    // Compute quad position based on alignment
    let text_x = text.x as f32;
    let text_y = text.y as f32;
    let w = bmp_w as f32;
    let h = bmp_h as f32;

    let x0 = match text.align.as_str() {
        "start" => text_x,
        "middle" => text_x - w / 2.0,
        "end" => text_x - w,
        _ => text_x - w / 2.0,
    };
    let y0 = text_y - h; // text position is at baseline, draw above it
    let x1 = x0 + w;
    let y1 = y0 + h;

    // Quad vertices (2 triangles)
    let positions: [f32; 12] = [
        x0, y0, x1, y0, x0, y1,
        x1, y0, x1, y1, x0, y1,
    ];
    let texcoords: [f32; 12] = [
        0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
        1.0, 0.0, 1.0, 1.0, 0.0, 1.0,
    ];

    let (r, g, b, a) = parse_hex_color(&text.fill);
    let text_color: [f32; 4] = [r as f32, g as f32, b as f32, a as f32];

    let pos_buffer = create_buffer(device, &positions);
    let tex_buffer = create_buffer(device, &texcoords);

    msg!(encoder, "setRenderPipelineState:", text_pipeline);

    msg!(encoder, "setVertexBuffer:offset:atIndex:", pos_buffer, 0 as usize, 0 as usize);
    msg!(encoder, "setVertexBuffer:offset:atIndex:", tex_buffer, 0 as usize, 1 as usize);
    msg!(encoder, "setVertexBytes:length:atIndex:", viewport_size.as_ptr() as Id, 8 as usize, 2 as usize);

    // Set fragment texture and color
    msg!(encoder, "setFragmentTexture:atIndex:", glyph_texture, 0 as usize);
    msg!(encoder, "setFragmentBytes:length:atIndex:", text_color.as_ptr() as Id, 16 as usize, 0 as usize);

    msg!(encoder, "drawPrimitives:vertexStart:vertexCount:", MTL_PRIMITIVE_TYPE_TRIANGLE as usize, 0 as usize, 6 as usize);

    release(pos_buffer);
    release(tex_buffer);
    release(glyph_texture);
}

unsafe fn read_back_pixels(texture: Id, width: u32, height: u32) -> PixelBuffer {
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
    // MTLRegion is 48 bytes (6 × 8), so it's passed by pointer on arm64.
    // We pass it as the struct directly and let the compiler handle the ABI.
    let get_bytes: unsafe extern "C" fn(
        Id, Sel,
        *mut u8,      // bytes pointer
        usize,        // bytesPerRow
        MTLRegion,    // region (passed indirectly by compiler on arm64)
        usize,        // mipmapLevel
    ) = std::mem::transmute(objc_msgSend as *const ());
    get_bytes(
        texture, sel("getBytes:bytesPerRow:fromRegion:mipmapLevel:"),
        data.as_mut_ptr(), bytes_per_row,
        region,
        0,
    );

    PixelBuffer::from_data(width, height, data)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use draw_instructions::{create_scene, draw_rect, draw_text, Metadata};

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn parse_hex_color_6_digit() {
        let (r, g, b, a) = parse_hex_color("#ff0000");
        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 0.0).abs() < 0.01);
        assert!((b - 0.0).abs() < 0.01);
        assert!((a - 1.0).abs() < 0.01);
    }

    #[test]
    fn parse_hex_color_8_digit() {
        let (r, g, b, a) = parse_hex_color("#00ff0080");
        assert!((r - 0.0).abs() < 0.01);
        assert!((g - 1.0).abs() < 0.01);
        assert!((b - 0.0).abs() < 0.01);
        assert!((a - 0.502).abs() < 0.01);
    }

    #[test]
    fn parse_hex_color_3_digit() {
        let (r, g, b, _a) = parse_hex_color("#f00");
        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 0.0).abs() < 0.01);
        assert!((b - 0.0).abs() < 0.01);
    }

    #[test]
    fn vertex_generation_rect() {
        let rect = draw_rect(10, 20, 30, 40, "#ff0000", Metadata::new());
        let mut positions = Vec::new();
        let mut colors = Vec::new();
        collect_rect_vertices(&[rect], &mut positions, &mut colors);

        // 6 vertices × 2 coords = 12 floats
        assert_eq!(positions.len(), 12);
        // 6 vertices × 4 color components = 24 floats
        assert_eq!(colors.len(), 24);
    }

    #[test]
    fn empty_scene_returns_empty_buffer() {
        let scene = create_scene(0, 0, vec![], "", Metadata::new());
        let buf = render_metal(&scene);
        assert_eq!(buf.width, 0);
        assert_eq!(buf.height, 0);
        assert!(buf.data.is_empty());
    }

    /// Render a simple scene with a red rectangle on a white background.
    /// Verify that the pixel at the rectangle's center is red and the
    /// pixel outside is white.
    #[test]
    fn render_red_rect_on_white() {
        let scene = create_scene(
            100,
            100,
            vec![draw_rect(10, 10, 80, 80, "#ff0000", Metadata::new())],
            "#ffffff",
            Metadata::new(),
        );

        let buf = render_metal(&scene);
        assert_eq!(buf.width, 100);
        assert_eq!(buf.height, 100);

        // Center of the red rectangle (50, 50) should be red
        let (r, g, b, a) = buf.pixel_at(50, 50);
        assert_eq!(r, 255, "red channel at center should be 255");
        assert_eq!(g, 0, "green channel at center should be 0");
        assert_eq!(b, 0, "blue channel at center should be 0");
        assert_eq!(a, 255, "alpha at center should be 255");

        // Corner (0, 0) should be white (background)
        let (r, g, b, a) = buf.pixel_at(0, 0);
        assert_eq!(r, 255, "red channel at corner should be 255");
        assert_eq!(g, 255, "green channel at corner should be 255");
        assert_eq!(b, 255, "blue channel at corner should be 255");
        assert_eq!(a, 255, "alpha at corner should be 255");
    }

    /// Render a scene with text and verify it doesn't crash.
    /// We can't easily verify exact pixel values for text (font rendering
    /// varies by system), but we can verify the pipeline doesn't panic.
    #[test]
    fn render_text_does_not_crash() {
        let scene = create_scene(
            200,
            50,
            vec![draw_text(100, 30, "HELLO", Metadata::new())],
            "#ffffff",
            Metadata::new(),
        );

        let buf = render_metal(&scene);
        assert_eq!(buf.width, 200);
        assert_eq!(buf.height, 50);
    }
}
