# Draw Instructions Metal Backend + Pixel Pipeline

## Overview

This spec defines a Metal-based GPU renderer for the draw-instructions IR,
plus the composable pipeline of crates needed to go from a `DrawScene` to a
rendered image file or a native macOS window.

The draw-instructions spec (see `draw-instructions.md`) establishes a
backend-neutral IR of five primitives (Rect, Text, Group, Line, Clip) plus
a Scene container. The SVG backend (`draw-instructions-svg`) translates
this IR to SVG markup. This spec adds a second backend that translates the
same IR to Metal GPU commands, producing either:

- an RGBA pixel buffer (for image file encoding)
- a native macOS window (for interactive display)

## Goals

- render any `DrawScene` to pixels using Apple's Metal GPU API
- define a shared `PixelBuffer` type that any renderer can produce and any
  image encoder can consume
- provide a PNG encoder as the first image format backend
- provide a windowed display mode for interactive use
- keep each concern in its own crate so they compose freely

## Non-goals (for this spec)

- Vulkan, Direct2D, GDI, or OpenGL renderers (future specs)
- JPEG, WebP, or other image encoders (future crates using the same
  `PixelBuffer` interface)
- V2 draw instructions (Line, Clip) — will be added to the Metal renderer
  once the V2 IR lands
- Cross-platform support — Metal is macOS-only and that is fine

## Architecture — The Pixel Pipeline

GPU rendering APIs (Metal, Vulkan, Direct2D, OpenGL) all render into
GPU-side framebuffers. To get an image file out, you do one read-back from
GPU to CPU memory, producing an RGBA pixel buffer. Then a separate encoder
turns that buffer into PNG, JPEG, WebP, or whatever format you need.

The key insight is that **the pixel buffer is the universal interchange
format between renderers and encoders.** Any renderer can produce one. Any
encoder can consume one. They never need to know about each other.

```
                         ┌─────────────────────────┐
                         │      DrawScene (IR)      │
                         └────────┬────────────────┘
                                  │
              ┌───────────────────┼───────────────────┐
              ▼                   ▼                   ▼
   draw-instructions-metal  (future: vulkan)   (future: direct2d)
              │                   │                   │
              ▼                   ▼                   ▼
         PixelBuffer         PixelBuffer         PixelBuffer
     (RGBA byte buffer)  (RGBA byte buffer)  (RGBA byte buffer)
              │
    ┌─────────┼─────────┐
    ▼         ▼         ▼
   PNG      JPEG      WebP
 encoder   encoder   encoder

   draw-instructions-metal-window
       (presents directly to screen — no read-back)
```

For windowed display, the GPU never reads back pixels at all. The Metal
render pass draws directly to the window's drawable surface (the swap
chain), and the window manager composites it to the screen. This is a
fundamentally different output path, which is why it lives in its own crate.

## New Rust Crates

| Crate | Path | Depends on | Purpose |
|-------|------|------------|---------|
| `draw-instructions-pixels` | `rust/draw-instructions-pixels/` | (none) | Shared `PixelBuffer` type |
| `draw-instructions-metal` | `rust/draw-instructions-metal/` | `draw-instructions`, `draw-instructions-pixels` | Metal GPU renderer: `DrawScene` → `PixelBuffer` |
| `draw-instructions-metal-window` | `rust/draw-instructions-metal-window/` | `draw-instructions`, `draw-instructions-metal` | Native macOS window display |
| `draw-instructions-png` | `rust/draw-instructions-png/` | `draw-instructions-pixels` | PNG encoder: `PixelBuffer` → PNG bytes |

---

## 1. draw-instructions-pixels

A tiny data crate with zero dependencies. It defines the shared type that
sits between renderers and encoders.

### Pixel format

RGBA8 — four bytes per pixel (red, green, blue, alpha), each in the range
0–255. Pixels are stored in row-major order with a top-left origin. This
is the native output format of:

- Metal `getBytes()` (with `MTLPixelFormat.rgba8Unorm`)
- Vulkan `vkMapMemory` (with `VK_FORMAT_R8G8B8A8_UNORM`)
- OpenGL `glReadPixels` (with `GL_RGBA` / `GL_UNSIGNED_BYTE`)

It is also the native input format for PNG, JPEG, and WebP encoders.

### Why top-left origin?

The draw-instructions IR uses top-left origin (Y increases downward), same
as SVG, HTML Canvas, and most 2D graphics systems. Metal's NDC has a
bottom-left origin, but the vertex shader handles the flip. The pixel
buffer stores the final result in the same orientation as the IR, so
encoder crates don't need to flip anything.

### Public API

```rust
pub const VERSION: &str = "0.1.0";

/// An RGBA pixel buffer — the universal interchange format between
/// GPU renderers and image encoders.
///
/// The buffer stores pixels in row-major order with a top-left origin.
/// Each pixel is four bytes: red, green, blue, alpha (0–255 each).
///
/// ## Memory layout
///
/// For a 3×2 image, the bytes are arranged like this:
///
/// ```text
/// byte index:  0  1  2  3    4  5  6  7    8  9 10 11
///              ├──R──G──B──A──┼──R──G──B──A──┼──R──G──B──A──┤  ← row 0
/// byte index: 12 13 14 15   16 17 18 19   20 21 22 23
///              ├──R──G──B──A──┼──R──G──B──A──┼──R──G──B──A──┤  ← row 1
/// ```
///
/// The byte offset for pixel (x, y) is `(y * width + x) * 4`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PixelBuffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl PixelBuffer {
    /// Create a new pixel buffer filled with transparent black (all zeros).
    pub fn new(width: u32, height: u32) -> Self;

    /// Create a pixel buffer from existing RGBA data.
    /// Panics if `data.len() != width * height * 4`.
    pub fn from_data(width: u32, height: u32, data: Vec<u8>) -> Self;

    /// Read one pixel. Returns (r, g, b, a).
    /// Panics if x >= width or y >= height.
    pub fn pixel_at(&self, x: u32, y: u32) -> (u8, u8, u8, u8);

    /// Write one pixel.
    /// Panics if x >= width or y >= height.
    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8);

    /// Total number of pixels (width × height).
    pub fn pixel_count(&self) -> usize;

    /// Number of bytes in the data buffer (width × height × 4).
    pub fn byte_count(&self) -> usize;
}
```

### Encoder trait

The crate also defines a trait that encoder crates implement:

```rust
/// Trait for image format encoders.
///
/// Each encoder (PNG, JPEG, WebP, etc.) implements this trait.
/// The encoder takes a pixel buffer and returns the encoded bytes
/// in the target format.
pub trait PixelEncoder {
    /// Encode a pixel buffer to the target image format.
    fn encode(&self, buffer: &PixelBuffer) -> Vec<u8>;
}
```

This trait lives in `draw-instructions-pixels` so that encoder crates only
need one dependency and don't need to know about any renderer.

---

## 2. draw-instructions-metal

The core Metal renderer. It takes a `DrawScene` and produces a `PixelBuffer`
by rendering via the GPU.

### Dependencies

- `draw-instructions` — the IR types (`DrawScene`, `DrawInstruction`, etc.)
- `draw-instructions-pixels` — the `PixelBuffer` output type
- `objc2` — safe Objective-C runtime bindings for Rust
- `objc2-metal` — typed wrappers around Metal framework classes
- `objc2-foundation` — `NSString`, `NSError`, etc.
- `objc2-quartz-core` — CoreAnimation types (needed for `CAMetalDrawable`)
- `core-text` (or raw CoreText via objc2) — font rasterization for text

### Platform restriction

This crate only compiles on macOS. The `Cargo.toml` should specify:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.6"
objc2-metal = "0.3"
objc2-foundation = "0.3"
```

On non-macOS platforms, the crate should compile but expose no public API
(empty module behind a `#[cfg]` gate). This way downstream crates that
conditionally depend on it won't fail to compile on Linux CI.

### Renderer trait implementation

```rust
use draw_instructions::{DrawScene, Renderer};
use draw_instructions_pixels::PixelBuffer;

pub struct MetalRenderer;

impl Renderer<PixelBuffer> for MetalRenderer {
    fn render(&self, scene: &DrawScene) -> PixelBuffer;
}

/// Convenience function matching the pattern from draw-instructions-svg.
pub fn render_metal(scene: &DrawScene) -> PixelBuffer;
```

### Metal pipeline — step by step

The renderer follows these steps:

#### Step 1: Device and command queue

```
let device = MTLCreateSystemDefaultDevice()
let command_queue = device.newCommandQueue()
```

The device represents the GPU. The command queue is a serial pipeline of
work items submitted to the GPU.

#### Step 2: Offscreen texture

```
let descriptor = MTLTextureDescriptor::texture2DDescriptor(
    pixelFormat: .rgba8Unorm,
    width: scene.width,
    height: scene.height,
    mipmapped: false
)
descriptor.usage = [.renderTarget, .shaderRead]
let texture = device.newTexture(descriptor)
```

This creates a GPU-side image buffer at the scene's pixel dimensions.
`rgba8Unorm` matches our `PixelBuffer` format exactly — four bytes per
pixel, values normalized to 0.0–1.0 in the shader.

#### Step 3: Render pass descriptor

```
let pass = MTLRenderPassDescriptor()
pass.colorAttachments[0].texture = texture
pass.colorAttachments[0].loadAction = .clear
pass.colorAttachments[0].clearColor = parse_hex_color(scene.background)
pass.colorAttachments[0].storeAction = .store
```

The load action `.clear` fills the texture with the scene's background
color before any geometry is drawn. The store action `.store` ensures the
rendered pixels are written back to the texture (not discarded).

#### Step 4: Shaders

Two shader programs, written in MSL (Metal Shading Language):

**Rect shader** — solid color fill:

```metal
struct RectVertex {
    float2 position;    // pixel coordinates
    float4 color;       // RGBA fill color
};

vertex float4 rect_vertex(
    const device RectVertex* vertices [[buffer(0)]],
    constant float2& viewport_size [[buffer(1)]],
    uint vid [[vertex_id]]
) {
    float2 px = vertices[vid].position;
    float2 ndc;
    ndc.x = (px.x / viewport_size.x) * 2.0 - 1.0;
    ndc.y = 1.0 - (px.y / viewport_size.y) * 2.0;
    return float4(ndc, 0.0, 1.0);
}

fragment float4 rect_fragment(
    /* color passed via stage_in from vertex */
) {
    return vertex_color;
}
```

The vertex shader converts pixel coordinates to Metal's normalized device
coordinates (NDC). Metal NDC is [-1, 1] with center origin and Y-up. The
IR uses top-left origin with Y-down. The formula:

```
ndc.x = (pixel_x / width) * 2.0 - 1.0
ndc.y = 1.0 - (pixel_y / height) * 2.0
```

maps (0, 0) → (-1, 1) (top-left) and (width, height) → (1, -1)
(bottom-right), which is correct.

**Text shader** — textured quads:

```metal
struct TextVertex {
    float2 position;    // pixel coordinates
    float2 texcoord;    // UV coordinates into glyph atlas
};

vertex TextVertexOut text_vertex(
    const device TextVertex* vertices [[buffer(0)]],
    constant float2& viewport_size [[buffer(1)]],
    uint vid [[vertex_id]]
) {
    // same NDC transform as rect_vertex
    // pass through texcoord
}

fragment float4 text_fragment(
    TextVertexOut in [[stage_in]],
    texture2d<float> glyph_texture [[texture(0)]],
    constant float4& text_color [[buffer(0)]]
) {
    float alpha = glyph_texture.sample(sampler, in.texcoord).r;
    return float4(text_color.rgb, text_color.a * alpha);
}
```

#### Step 5: Vertex generation

Each draw instruction is converted to vertices:

**DrawRect** → 6 vertices (2 triangles):

```
(x, y) ─────── (x+w, y)
  │  ╲              │
  │    ╲            │
  │      ╲          │
  │        ╲        │
  │          ╲      │
  │            ╲    │
  │              ╲  │
(x, y+h) ──── (x+w, y+h)

Triangle 1: (x, y), (x+w, y), (x, y+h)
Triangle 2: (x+w, y), (x+w, y+h), (x, y+h)
```

Each vertex carries the fill color from the `DrawRect` instruction.

**DrawText** → rasterize glyphs via CoreText, then draw textured quad:

1. Create a `CTFont` from `font_family` and `font_size`
2. Create a `CTLine` from the text value
3. Get the typographic bounds to determine the bounding box
4. Create a bitmap context at the glyph bounds size
5. Draw the line into the bitmap context
6. Upload the bitmap to a `MTLTexture`
7. Emit 6 vertices for the textured quad at the text position

The text position (x, y) from the IR is the anchor point. The `align`
field controls horizontal positioning:

- `"start"` — left edge at x
- `"middle"` — center at x
- `"end"` — right edge at x

**DrawGroup** → recursive descent. Groups have no visual representation;
they just sequence their children. The renderer walks into the group's
children array and processes each instruction.

#### Step 6: Command buffer and encoding

```
let command_buffer = command_queue.commandBuffer()
let encoder = command_buffer.renderCommandEncoder(pass)
encoder.setRenderPipelineState(rect_pipeline)
encoder.setVertexBuffer(vertex_buffer, offset: 0, index: 0)
encoder.setVertexBytes(&viewport_size, length: 8, index: 1)
encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: N)
// ... repeat for text quads with text_pipeline ...
encoder.endEncoding()
command_buffer.commit()
command_buffer.waitUntilCompleted()
```

All rects can be batched into a single draw call (they share the same
shader). Text quads use a different pipeline state (textured shader) and
may need separate draw calls per glyph texture, or a texture atlas.

#### Step 7: Read-back

```
let bytes_per_row = width * 4
let buffer = vec![0u8; width * height * 4]
texture.getBytes(
    buffer.as_mut_ptr(),
    bytesPerRow: bytes_per_row,
    from: MTLRegion(origin: (0, 0, 0), size: (width, height, 1)),
    mipmapLevel: 0
)
PixelBuffer::from_data(width, height, buffer)
```

This is the one GPU-to-CPU transfer. After this point, everything is
CPU-side and the GPU is no longer involved.

### Color parsing

The IR stores colors as hex strings (`"#ff0000"`, `"#000000"`). The
renderer must parse these to floating-point RGBA values for Metal:

```rust
fn parse_hex_color(hex: &str) -> (f64, f64, f64, f64) {
    // "#rrggbb" → (r/255, g/255, b/255, 1.0)
    // "#rrggbbaa" → (r/255, g/255, b/255, a/255)
}
```

---

## 3. draw-instructions-metal-window

Presents a rendered scene in a native macOS window. Unlike the pixel buffer
path, this skips the read-back entirely — the Metal render pass draws
directly to the window's `CAMetalDrawable`.

### Dependencies

- `draw-instructions` — the IR types
- `draw-instructions-metal` — shared shader/vertex generation code
- `objc2-app-kit` — `NSApplication`, `NSWindow`
- `objc2-metal` — Metal framework
- `objc2-metal-kit` — `MTKView`, `MTKViewDelegate`

### Public API

```rust
/// Open a native macOS window displaying the rendered scene.
///
/// This function blocks until the window is closed. The window title
/// is taken from the scene's metadata "label" field, or defaults to
/// "Draw Instructions".
pub fn show_in_window(scene: &DrawScene);
```

### How it works

1. Create an `NSApplication` (required for any macOS GUI)
2. Create an `NSWindow` with the scene's width/height
3. Create an `MTKView` and set it as the window's content view
4. Implement `MTKViewDelegate` to render the scene on `drawInMTKView:`
5. Call `NSApplication::run()` to enter the event loop (blocks)
6. When the user closes the window, the event loop exits and `show_in_window` returns

The rendering logic (vertex generation, shaders, pipeline setup) is shared
with `draw-instructions-metal`. The only difference is the render target:
instead of an offscreen texture, the render pass targets the `MTKView`'s
`currentDrawable.texture`.

---

## 4. draw-instructions-png

Encodes a `PixelBuffer` to PNG format. This crate knows nothing about
Metal, Vulkan, or any renderer — it just takes pixels and produces PNG.

### Dependencies

- `draw-instructions-pixels` — the `PixelBuffer` and `PixelEncoder` types
- `png` crate (pure Rust PNG encoder, no system dependencies)

### Public API

```rust
pub const VERSION: &str = "0.1.0";

pub struct PngEncoder;

impl PixelEncoder for PngEncoder {
    /// Encode a pixel buffer to PNG bytes.
    fn encode(&self, buffer: &PixelBuffer) -> Vec<u8>;
}

/// Convenience function.
pub fn encode_png(buffer: &PixelBuffer) -> Vec<u8>;

/// Encode and write directly to a file.
pub fn write_png(buffer: &PixelBuffer, path: &str) -> Result<(), std::io::Error>;
```

### PNG encoding details

PNG supports RGBA natively, so no color space conversion is needed:

1. Create a `png::Encoder` writing to a `Vec<u8>`
2. Set width, height, color type (RGBA), bit depth (8)
3. Write the `PixelBuffer.data` as image data
4. Return the encoded bytes

The `png` crate handles compression (deflate), filtering, and chunk
formatting. We use default compression level (a good balance of size and
speed for the barcode use case).

---

## 5. FFI Bridge Gaps

The existing `python-bridge` and `ruby-bridge` crates need additions
before the Metal/PNG native extensions can be built.

### Current bridge capabilities

| Capability | Python bridge | Ruby bridge |
|-----------|---------------|-------------|
| Strings | `str_to_py` / `str_from_py` | `str_to_rb` / `str_from_rb` |
| String lists | `vec_str_to_py` / `vec_str_from_py` | `vec_str_to_rb` / `vec_str_from_rb` |
| Booleans | `bool_to_py` | (via `QTRUE`/`QFALSE`) |
| Integers | `usize_to_py` (unsigned only) | (none) |
| Sets | `set_str_to_py` / `set_str_from_py` | (none) |
| Classes | `PyType_FromSpec` wrappers | `define_class_under` + `wrap_data` |
| Modules | `PyModule_Create2` | `define_module` |
| Errors | `set_error` + exception classes | `raise_error` + variants |

### Additions needed

| Function | Crate | Wraps | Purpose |
|----------|-------|-------|---------|
| `bytes_to_py(data: &[u8])` | python-bridge | `PyBytes_FromStringAndSize` | Return PNG bytes to Python as `bytes` |
| `bytes_from_py(obj) -> Option<Vec<u8>>` | python-bridge | `PyBytes_AsStringAndSize` | Accept bytes from Python |
| `bytes_to_rb(data: &[u8])` | ruby-bridge | `rb_str_new` + force ASCII-8BIT encoding | Return PNG bytes to Ruby as binary `String` |
| `i32_to_py(n: i32)` | python-bridge | `PyLong_FromLong` | Scene width/height/coordinates |
| `i32_from_py(obj) -> Option<i32>` | python-bridge | `PyLong_AsLong` | Parse width/height from Python |
| `i32_to_rb(n: i32)` | ruby-bridge | `INT2FIX` / `rb_int2inum` | Scene width/height/coordinates |
| `i32_from_rb(obj) -> Option<i32>` | ruby-bridge | `FIX2INT` / `rb_num2int` | Parse width/height from Ruby |
| `dict_str_str_to_py(map: &BTreeMap<String, String>)` | python-bridge | `PyDict_New` + `PyDict_SetItemString` | Metadata conversion |
| `dict_str_str_from_py(obj) -> Option<BTreeMap<String, String>>` | python-bridge | `PyDict_Next` iteration | Parse metadata from Python |
| `hash_str_str_to_rb(map: &BTreeMap<String, String>)` | ruby-bridge | `rb_hash_new` + `rb_hash_aset` | Metadata conversion |
| `hash_str_str_from_rb(obj) -> Option<BTreeMap<String, String>>` | ruby-bridge | `rb_hash_foreach` | Parse metadata from Ruby |

These follow the same zero-dependency, `extern "C"` pattern as existing
bridge functions. Each addition is ~10-20 lines of code.

---

## 6. Native Extensions

After the bridge gaps are filled, native extension crates expose the
Metal renderer and PNG encoder to Python and Ruby.

### Python native extension

```
code/packages/python/draw-instructions-metal-native/
  Cargo.toml   (cdylib, depends on python-bridge + draw-instructions-metal)
  src/lib.rs   (PyInit_draw_instructions_metal_native)

code/packages/python/draw-instructions-png-native/
  Cargo.toml   (cdylib, depends on python-bridge + draw-instructions-png)
  src/lib.rs   (PyInit_draw_instructions_png_native)
```

**Python API:**

```python
import draw_instructions_metal_native as metal
import draw_instructions_png_native as png_enc

# Scene as a dict matching the DrawScene structure
scene = {
    "width": 400, "height": 200, "background": "#ffffff",
    "instructions": [
        {"kind": "rect", "x": 10, "y": 10, "width": 4, "height": 120,
         "fill": "#000000", "metadata": {}},
        {"kind": "text", "x": 200, "y": 150, "value": "HELLO",
         "fill": "#000000", "font_family": "monospace", "font_size": 16,
         "align": "middle", "metadata": {}},
    ],
    "metadata": {"label": "Code39 barcode"}
}

# Render to pixel buffer (opaque object wrapping PixelBuffer)
pixels = metal.render(scene)

# Encode to PNG bytes
png_bytes = png_enc.encode(pixels)

# Write PNG to file
png_enc.write(pixels, "/tmp/barcode.png")

# Windowed display (blocks until window closed)
metal.show_in_window(scene)
```

### Ruby native extension

```
code/packages/ruby/draw_instructions_metal_native/
  ext/draw_instructions_metal_native/
    Cargo.toml    (cdylib)
    src/lib.rs    (Init_draw_instructions_metal_native)

code/packages/ruby/draw_instructions_png_native/
  ext/draw_instructions_png_native/
    Cargo.toml    (cdylib)
    src/lib.rs    (Init_draw_instructions_png_native)
```

**Ruby API:**

```ruby
require 'draw_instructions_metal_native'
require 'draw_instructions_png_native'

scene = {
  width: 400, height: 200, background: "#ffffff",
  instructions: [
    { kind: "rect", x: 10, y: 10, width: 4, height: 120,
      fill: "#000000", metadata: {} },
  ],
  metadata: { label: "Code39 barcode" }
}

pixels = CodingAdventures::DrawInstructionsMetalNative.render(scene)
png_bytes = CodingAdventures::DrawInstructionsPngNative.encode(pixels)
CodingAdventures::DrawInstructionsPngNative.write(pixels, "/tmp/barcode.png")
CodingAdventures::DrawInstructionsMetalNative.show_in_window(scene)
```

### Scene dict → DrawScene reconstruction

The native extension code must reconstruct a Rust `DrawScene` from the
Python dict or Ruby hash. This involves:

1. Extract `width`, `height` (integers) and `background` (string)
2. Extract `metadata` (string→string dict/hash)
3. Walk the `instructions` list, and for each:
   - Read `kind` to determine the instruction type
   - Extract the type-specific fields
   - Recursively handle `children` for groups
4. Build the `DrawScene` struct

This is straightforward dict traversal using the bridge's `str_from_py`,
`i32_from_py`, `dict_str_str_from_py`, and list iteration functions.

### PixelBuffer across the FFI boundary

The `PixelBuffer` returned by `metal.render()` is wrapped as an opaque
Python/Ruby object using the bridge's `wrap_data<T>()` mechanism (the same
pattern used by `bitset-native` for `Bitset`). The PNG encoder's `encode`
function unwraps it to get the Rust struct back.

---

## 7. Limitations

- **macOS only** — Metal is an Apple-only API. The native extensions
  should raise a clear error on non-macOS platforms.
- **Text rendering depends on CoreText** — font availability and rendering
  quality match the host system. Monospace fonts are universally available.
- **No GPU benefit for simple barcodes** — a Code39 barcode is ~50
  rectangles and one text label. The GPU won't be faster than CPU
  rasterization for this. The value is in validating the architecture for
  future complex scenes (large tables, interactive visualizations).
- **Window mode blocks** — `show_in_window()` runs a macOS event loop and
  blocks until the window is closed. Callers should be aware of this.
- **Requires macOS 10.14+** (Mojave) — this is when
  `MTLCreateSystemDefaultDevice()` became reliable. Any Mac from 2012
  or later running a supported macOS version has Metal.

## 8. Testing Strategy

| Crate | Test approach |
|-------|--------------|
| `draw-instructions-pixels` | Unit tests: `new`, `from_data`, `pixel_at`, `set_pixel`, bounds checks, byte count |
| `draw-instructions-metal` | Integration test on macOS: render a scene with one red rect on white background, verify specific pixel values (red at rect location, white elsewhere) |
| `draw-instructions-png` | Unit test: encode a known pixel buffer, verify PNG magic bytes (`\x89PNG`), decode with `png` crate and compare pixels |
| `draw-instructions-metal-window` | Manual test only (opens a window) — no automated test |
| Native extensions (Python) | Integration test: create scene dict, render, encode to PNG, verify file exists and has PNG magic bytes |
| Native extensions (Ruby) | Integration test: same as Python but via Ruby |

Metal integration tests require a macOS CI runner with GPU access. GitHub
Actions `macos-latest` runners have Metal support.

## 9. Future Extensions

- **V2 instructions**: Add `DrawLine` and `DrawClip` to the Metal
  renderer once the V2 IR lands in `draw-instructions`
- **Stroke rendering**: Rects with stroke require either a second draw
  call with line topology or expanded outline quads
- **Additional encoders**: `draw-instructions-jpeg`, `draw-instructions-webp`
  as separate crates consuming `PixelBuffer`
- **Additional renderers**: `draw-instructions-vulkan` (cross-platform),
  `draw-instructions-direct2d` (Windows), `draw-instructions-gdi`
  (older Windows)
- **Interactive window**: Mouse hover to inspect metadata (hit testing
  against rect bounds), zoom/pan
- **Instanced rendering**: Batch all rects into a single draw call with
  per-instance data (color, position, size) for large scenes
- **Multi-language PixelBuffer**: Implement `draw-instructions-pixels` in
  Python, Ruby, TypeScript, etc. so the pixel pipeline concept exists in
  every language (even if only the Rust path uses GPU acceleration)
