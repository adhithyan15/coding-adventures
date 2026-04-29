# P2D06 — Paint VM Native Backends: Direct2D, GDI, Cairo

## Overview

This spec covers three Rust crates that render PaintScene (P2D00) instructions
through native platform rendering APIs on Windows and Linux. Each crate plugs
into the PaintVM dispatch-table architecture (P2D01) — it registers handlers for
the 10 instruction kinds and translates them into the platform's drawing calls.

For the larger backend convergence roadmap, including Cairo, Skia, Vulkan,
OpenGL, WGPU, CoreGraphics, capability reporting, and automatic backend
selection, see `P2D09-paint-vm-backend-convergence.md`.

Think of it like hiring three different sign painters. You hand each of them the
same blueprint (a PaintScene). One works in oil paint (Direct2D — rich, modern,
GPU-accelerated). One works in house paint (GDI — available everywhere, gets the
job done, but no fine detail). One works in watercolor (Cairo — beautiful on
Linux, cross-platform, mature). The blueprint is identical; the tools and
techniques differ.

```
                        PaintScene (P2D00)
                             |
                             v
                    PaintVM dispatch (P2D01)
                   /         |         \
                  v          v          v
            Direct2D       GDI        Cairo
            (P2D06)      (P2D07)     (P2D08)
            Windows      Windows      Linux
            modern       fallback     future
```

The existing Metal backend (`paint-metal`) already demonstrates this pattern for
macOS/iOS. These three crates complete the picture for Windows and Linux:

| Crate              | Platform      | API           | Status          |
|--------------------|---------------|---------------|-----------------|
| paint-metal        | macOS / iOS   | Metal GPU     | Partial (exists)|
| paint-vm-direct2d  | Windows       | Direct2D      | This spec       |
| paint-vm-gdi       | Windows       | GDI (Win32)   | This spec       |
| paint-vm-cairo     | Linux / GTK   | Cairo + Pango | Design only     |

---

## Where It Fits

```
Layer 0 (data)       paint-instructions    PaintScene, PaintInstruction, PixelContainer
Layer 1 (VM)         paint-vm              dispatch-table VM framework
Layer 2 (backends)   paint-vm-canvas       TypeScript — browser Canvas2D
                     paint-vm-svg          TypeScript — SVG string output
                     paint-metal           Rust — Apple Metal GPU
                  →  paint-vm-direct2d     Rust — Windows Direct2D  ← THIS SPEC
                  →  paint-vm-gdi          Rust — Windows GDI       ← THIS SPEC
                  →  paint-vm-cairo        Rust — Linux Cairo       ← THIS SPEC (design only)
Layer 3 (codecs)     paint-codec-png       PixelContainer ↔ PNG bytes
```

Every backend depends on `paint-instructions` for the PaintScene/PaintInstruction
types and `PixelContainer` for pixel output. The backends do NOT depend on each
other — they are siblings, not a chain.

The coordinate system across all backends is **top-left origin, Y-down**. This
matches Direct2D natively, matches GDI natively, and matches Cairo natively. No
coordinate flipping is needed (unlike Metal, which is Y-up and requires a
projection matrix flip).

---

## Concepts

### The Handler Pattern

Each backend implements the same 10 handlers. A handler is a function that takes
one instruction and a platform-specific rendering context, then issues the
platform's drawing calls. This is the same pattern used in `paint-metal`:

```rust
fn handle_rect(instruction: &PaintRect, ctx: &mut PlatformContext) {
    // Translate PaintRect fields into platform API calls
}
```

The dispatch function matches on the `PaintInstruction` enum:

```rust
fn dispatch(instruction: &PaintInstruction, ctx: &mut PlatformContext) {
    match instruction {
        PaintInstruction::Rect(r)      => handle_rect(r, ctx),
        PaintInstruction::Ellipse(e)   => handle_ellipse(e, ctx),
        PaintInstruction::Path(p)      => handle_path(p, ctx),
        PaintInstruction::GlyphRun(g)  => handle_glyph_run(g, ctx),
        PaintInstruction::Group(g)     => handle_group(g, ctx),   // recurses
        PaintInstruction::Layer(l)     => handle_layer(l, ctx),   // recurses
        PaintInstruction::Line(l)      => handle_line(l, ctx),
        PaintInstruction::Clip(c)      => handle_clip(c, ctx),    // recurses
        PaintInstruction::Gradient(g)  => handle_gradient(g, ctx),
        PaintInstruction::Image(i)     => handle_image(i, ctx),
    }
}
```

Container instructions — `group`, `layer`, and `clip` — hold child instructions.
Their handlers set up context (push transform, push layer, set clip region), then
recursively call `dispatch` on each child, then tear down context (pop transform,
pop layer, restore clip). This is a pre-order tree walk:

```
group (transform=[translate 100, 0])
  ├── rect (fill=red, x=0, y=0, w=50, h=50)
  └── group (transform=[rotate 45deg])
        └── ellipse (fill=blue, cx=0, cy=0, rx=30, ry=20)

Dispatch order:
  1. handle_group → push transform [translate 100,0]
  2.   handle_rect → draw red rect at (100,0) in world space
  3.   handle_group → push transform [rotate 45deg] (compounds with parent)
  4.     handle_ellipse → draw blue ellipse, rotated 45deg, translated (100,0)
  5.   pop transform [rotate 45deg]
  6. pop transform [translate 100,0]
```

### COM and Direct2D Initialization

Direct2D is a COM (Component Object Model) API. COM is Microsoft's way of
providing language-neutral interfaces to system services. In practice, it means:

1. You create "factory" objects that manufacture other objects.
2. Every object is reference-counted (like `Arc` in Rust).
3. Methods return `HRESULT` error codes instead of using exceptions.

The `windows` crate wraps all of this in safe Rust. The initialization sequence:

```rust
// Step 1: Create the Direct2D factory (entry point to all of Direct2D)
let d2d_factory: ID2D1Factory = D2D1CreateFactory(
    D2D1_FACTORY_TYPE_SINGLE_THREADED,
    None,  // factory options
)?;

// Step 2: Create the DirectWrite factory (for text rendering)
let dwrite_factory: IDWriteFactory = DWriteCreateFactory(
    DWRITE_FACTORY_TYPE_SHARED,
)?;

// Step 3: Create a render target (where pixels go)
// Option A: Window (HWND) render target — draws directly to a window
let render_target: ID2D1HwndRenderTarget = d2d_factory.CreateHwndRenderTarget(
    &render_target_properties,
    &hwnd_render_target_properties,  // includes the HWND and pixel size
)?;

// Option B: Bitmap render target — draws to an offscreen bitmap
let bitmap_target: ID2D1BitmapRenderTarget =
    render_target.CreateCompatibleRenderTarget(
        Some(&D2D_SIZE_F { width: w, height: h }),
        None, None,
        D2D1_COMPATIBLE_RENDER_TARGET_OPTIONS_NONE,
    )?;
```

Think of the factory as a hardware store. You go there to buy brushes, canvases,
and geometry templates. The render target is the canvas you actually paint on.

### GDI Device Context

GDI predates COM. It uses a simpler model: you get a **device context** (HDC),
which is an opaque handle to a drawing surface. You select objects (pens,
brushes, fonts) "into" the DC, draw with them, then select them back out.

```rust
// GDI's "select in, draw, select out" pattern:
let old_brush = SelectObject(hdc, new_brush);
Rectangle(hdc, left, top, right, bottom);
SelectObject(hdc, old_brush);
DeleteObject(new_brush);
```

This is like a physical painting station: you clip a brush onto the easel
(SelectObject), paint with it (Rectangle), then swap it out and put the old
brush back. You must clean up (DeleteObject) or you leak GDI handles — a
classic Windows bug that causes mysterious rendering failures after ~10,000
leaked handles.

### Cairo Context

Cairo uses a **stateful context** model similar to HTML Canvas. You set state
(color, line width, transform), define a path, then stroke or fill it:

```c
cairo_set_source_rgb(cr, 1.0, 0.0, 0.0);   // red
cairo_rectangle(cr, 10, 20, 100, 50);       // define path
cairo_fill(cr);                              // consume path, fill it
```

Cairo's `cairo_save()` / `cairo_restore()` pair saves and restores the entire
graphics state (transform, clip, color, line width) — exactly like
Canvas2D's `save()` / `restore()`, and exactly what PaintGroup and PaintClip
need.

### Color Parsing

All three backends need to convert PaintInstruction's color representation
(CSS-style strings: `#rrggbb`, `#rrggbbaa`, named colors like `red`) into
platform-native color structs:

| Platform  | Color type                                    | Components        |
|-----------|-----------------------------------------------|-------------------|
| Direct2D  | `D2D1_COLOR_F { r, g, b, a }`                | f32, 0.0..1.0     |
| GDI       | `COLORREF` (0x00BBGGRR)                       | u8, note BGR order |
| Cairo     | `cairo_set_source_rgba(cr, r, g, b, a)`       | f64, 0.0..1.0     |

Note GDI's quirk: the byte order is **BGR**, not RGB. The hex color `#FF8800`
(orange) becomes `COLORREF(0x000088FF)`. This has been a source of subtle bugs
in Windows programs for 30 years.

---

## Public API

### Shared Entry Points

All three crates expose the same two public functions, following the pattern
established by `paint-metal`:

```rust
/// Render a PaintScene to the given platform render target.
///
/// This is the immediate-mode path: every instruction in the scene
/// is dispatched to its handler, which issues drawing calls against
/// the platform API through the render target.
pub fn render(scene: &PaintScene, target: &mut T) { ... }

/// Render a PaintScene to an offscreen pixel buffer.
///
/// Creates a temporary offscreen render target, renders the scene,
/// then copies the result into a PixelContainer (RGBA8, premultiplied).
/// The `scale` parameter controls DPI scaling (1.0 = 96 DPI on Windows).
pub fn render_to_pixels(scene: &PaintScene, scale: f64) -> PixelContainer { ... }
```

The `render_to_pixels` function is the primary integration point for testing and
for the codec pipeline (render → PixelContainer → PNG via paint-codec-png).

### P2D06 — paint-vm-direct2d

**Crate:** `paint-vm-direct2d`
**Platform:** Windows (requires Windows 7+)
**Dependencies:** `windows` crate with features:
- `Win32_Graphics_Direct2D`
- `Win32_Graphics_Direct2D_Common`
- `Win32_Graphics_DirectWrite`
- `Win32_Graphics_Imaging`

**Context type:** `ID2D1HwndRenderTarget` (windowed) or `ID2D1BitmapRenderTarget` (offscreen)

#### Handler Mapping

| Instruction  | Direct2D API                                                                        |
|-------------|--------------------------------------------------------------------------------------|
| rect        | `FillRectangle` / `DrawRectangle` with `ID2D1SolidColorBrush`; use `ID2D1RoundedRectangleGeometry` when `corner_radius > 0` |
| ellipse     | `FillEllipse` / `DrawEllipse`                                                       |
| path        | `ID2D1PathGeometry` with `ID2D1GeometrySink`: `BeginFigure`, `AddLine`, `AddBezier`, `EndFigure` map directly to PaintPath segments (MoveTo, LineTo, BezierTo, Close) |
| glyph_run   | `DrawGlyphRun` using `IDWriteFactory` + `IDWriteTextFormat` for font selection      |
| group       | `GetTransform` / `SetTransform` (save current, multiply by `PaintGroup.transform`, restore after children) |
| layer       | `PushLayer` / `PopLayer` with `ID2D1Layer` for offscreen compositing with opacity   |
| line        | `DrawLine` with `ID2D1StrokeStyle` for line caps                                    |
| clip        | `PushAxisAlignedClip` / `PopAxisAlignedClip` (axis-aligned rect clip)               |
| gradient    | `CreateLinearGradientBrush` / `CreateRadialGradientBrush` with `ID2D1GradientStopCollection` for stop colors/positions |
| image       | `CreateBitmapFromMemory` (load RGBA8 `PixelContainer` data into `ID2D1Bitmap`) then `DrawBitmap` |

#### COM Initialization Sequence

```
D2D1CreateFactory(SINGLE_THREADED)
    → ID2D1Factory
         |
         ├── CreateHwndRenderTarget(hwnd, size)
         |       → ID2D1HwndRenderTarget
         |
         └── (or) CreateWicBitmapRenderTarget(bitmap)
                 → ID2D1RenderTarget (offscreen)

DWriteCreateFactory(SHARED)
    → IDWriteFactory
         └── CreateTextFormat(font_family, size, weight, style)
                 → IDWriteTextFormat
```

Every frame follows the `BeginDraw` / `EndDraw` protocol:

```rust
render_target.BeginDraw();
render_target.Clear(&background_color);
for instruction in &scene.instructions {
    dispatch(instruction, &mut ctx);
}
render_target.EndDraw(None, None)?;
```

If `EndDraw` returns `D2DERR_RECREATE_TARGET`, the render target (and all
device-dependent resources like brushes and bitmaps) must be recreated. This
happens when the GPU driver resets, the display mode changes, or the window
moves to a different monitor. The crate handles this by discarding all cached
resources and re-initializing.

### P2D07 — paint-vm-gdi

**Crate:** `paint-vm-gdi`
**Platform:** Windows (all versions, back to Win95 in theory)
**Dependencies:** `windows` crate with features:
- `Win32_Graphics_Gdi`
- `Win32_UI_WindowsAndMessaging`

**Context type:** `GdiContext { hdc: HDC, width: u32, height: u32 }`

#### Handler Mapping

| Instruction  | GDI API                                                                              |
|-------------|--------------------------------------------------------------------------------------|
| rect        | `Rectangle()` with `HBRUSH` from `CreateSolidBrush`, or `FillRect()`               |
| ellipse     | `Ellipse()`                                                                          |
| path        | `BeginPath` / `MoveToEx` / `LineTo` / `PolyBezierTo` / `EndPath` / `StrokeAndFillPath` |
| glyph_run   | `ExtTextOutW` with `HFONT` from `CreateFontW`                                      |
| group       | `SaveDC` / `RestoreDC` + `SetWorldTransform` for the 2x3 affine matrix             |
| layer       | `CreateCompatibleDC` + `CreateCompatibleBitmap` → render children offscreen → `AlphaBlend` back to main DC |
| line        | `MoveToEx` / `LineTo` with `HPEN` from `CreatePen`                                 |
| clip        | `IntersectClipRect`                                                                  |
| gradient    | `GradientFill` (axis-aligned linear only; radial gradients fall back to solid color at the midpoint) |
| image       | `CreateDIBSection` → copy `PixelContainer` RGBA data → `AlphaBlend` or `StretchBlt` |

#### GDI Resource Management

GDI requires explicit cleanup of every created object. Failure to call
`DeleteObject` leaks GDI handles, and Windows has a system-wide limit of
~10,000 per process. The crate uses RAII wrappers:

```rust
/// RAII wrapper for a GDI object. Calls DeleteObject on drop.
struct GdiObject<T>(T);

impl<T> Drop for GdiObject<T> {
    fn drop(&mut self) {
        unsafe { DeleteObject(self.0); }
    }
}
```

#### Limitations vs Direct2D

GDI is the fallback renderer for situations where Direct2D is unavailable
(remote desktop sessions, very old hardware, headless servers). It has
significant limitations:

| Capability            | Direct2D        | GDI                       |
|----------------------|-----------------|---------------------------|
| Path antialiasing    | Yes (built-in)  | No (jagged edges)         |
| Radial gradients     | Yes             | No (falls back to solid)  |
| Blend modes          | Full set        | SRC_OVER only             |
| Hardware acceleration| Yes (GPU)       | No (CPU only)             |
| Subpixel positioning | Yes             | No (integer coordinates)  |
| Layer opacity        | Native          | Manual (offscreen blit)   |
| Rounded rectangles   | Native geometry | Manual (RoundRect or arcs)|

### P2D08 — paint-vm-cairo (Design Only)

**Crate:** `paint-vm-cairo`
**Platform:** Linux (GTK), BSDs, macOS (via Homebrew)
**Dependencies:** `cairo-rs` crate, `pango` crate (for text)
**Status:** Design only. Implementation deferred until Linux platform support begins.

**Context type:** `cairo::Context` (wraps `*mut cairo_t`)

#### Handler Mapping

| Instruction  | Cairo API                                                                            |
|-------------|--------------------------------------------------------------------------------------|
| rect        | `cairo_rectangle` + `cairo_fill` / `cairo_stroke`; rounded corners via four `cairo_arc` calls at corners |
| ellipse     | `cairo_save` + `cairo_scale` (to squash a circle into an ellipse) + `cairo_arc` + `cairo_restore` |
| path        | `cairo_move_to`, `cairo_line_to`, `cairo_curve_to` (cubic bezier), `cairo_arc`, `cairo_close_path` |
| glyph_run   | `pango_cairo_show_layout` via `PangoLayout` for shaped, positioned text             |
| group       | `cairo_save` / `cairo_restore` + `cairo_transform` (multiply current matrix)        |
| layer       | `cairo_push_group` / `cairo_pop_group_to_source` + `cairo_paint_with_alpha`         |
| line        | `cairo_move_to` + `cairo_line_to` + `cairo_stroke`                                  |
| clip        | `cairo_rectangle` + `cairo_clip`                                                     |
| gradient    | `cairo_pattern_create_linear` / `cairo_pattern_create_radial` + `cairo_pattern_add_color_stop_rgba` |
| image       | `cairo_image_surface_create_for_data` (wrap RGBA8 `PixelContainer`) + `cairo_set_source_surface` + `cairo_paint` |

Cairo's state-stack model (`save`/`restore`) maps cleanly to PaintGroup and
PaintClip, just as Canvas2D does in the TypeScript backend. The Cairo backend
should be the simplest of the three to implement when the time comes.

---

## Testing Strategy

### 1. Per-Handler Unit Tests

Construct a minimal PaintScene containing a single instruction type, render it
to a `PixelContainer` via `render_to_pixels`, then verify specific pixels:

```rust
#[test]
fn red_rect_at_origin() {
    let scene = PaintScene {
        width: 100.0,
        height: 100.0,
        instructions: vec![
            PaintInstruction::Rect(PaintRect {
                x: 0.0, y: 0.0, width: 50.0, height: 50.0,
                fill: Some("#ff0000".into()),
                ..Default::default()
            }),
        ],
    };
    let pixels = render_to_pixels(&scene, 1.0);
    // Pixel at (25, 25) should be red
    assert_eq!(pixels.get(25, 25), [255, 0, 0, 255]);
    // Pixel at (75, 75) should be transparent/background
    assert_eq!(pixels.get(75, 75), [0, 0, 0, 0]);
}
```

One test per instruction kind, covering: filled, stroked, both, edge-at-boundary.

### 2. Golden Image Tests

Render a set of reference PaintScenes (stored as JSON fixtures) and compare the
output pixel-by-pixel against known-good PNG files. Allow a per-pixel tolerance
of +/- 2 in each channel to account for antialiasing differences across GPU
drivers.

Golden images are committed to the repo under `code/fixtures/golden/paint-vm/`.

### 3. Cross-Backend Comparison

Render the same PaintScene through Direct2D and through the existing Canvas
backend (via paint-vm-canvas in a headless browser). Compare the two
PixelContainers with a structural similarity metric (SSIM > 0.95). This catches
semantic errors where a handler produces plausible-looking but incorrect output.

### 4. Stress Tests

- **Large scene:** 10,000 rects with random positions and colors — must complete
  within 500ms on an integrated GPU.
- **Deep nesting:** 100 levels of nested groups — must not stack overflow.
- **Many gradients:** 1,000 gradient instructions with 10 stops each — must not
  leak GPU resources.

### 5. Edge Cases

- Empty scene (zero instructions) — must produce a blank PixelContainer.
- Zero-size rect (width=0 or height=0) — must not crash, should be a no-op.
- Fully transparent fill (`#00000000`) — must not produce visible pixels.
- Scene with only container instructions (groups with no leaves) — must not crash.
- Invalid gradient stop positions (e.g., all at 0.0) — must not crash, behavior
  is implementation-defined.

---

## Scope

### In Scope

- All 10 PaintVM instruction handlers for Direct2D (P2D06) and GDI (P2D07).
- Cairo handler mapping and design (P2D08) — design only, no implementation.
- `render()` and `render_to_pixels()` public API for Direct2D and GDI.
- COM initialization and teardown for Direct2D.
- RAII resource management for GDI handles.
- Color parsing (CSS hex → platform color structs).
- Per-handler unit tests and golden image tests.

### Out of Scope

- **Animated rendering / frame loops** — the backends render single frames on
  demand. Continuous rendering is the window manager's responsibility (BR01).
- **Input handling** — mouse, keyboard, touch. That belongs to the platform
  shell, not the paint backend.
- **Window management** — creating windows, handling resize, DPI changes. The
  render target is provided by the caller; backends do not own windows.
- **Font loading / management** — backends use system fonts via DirectWrite or
  Pango. Custom font loading is a separate concern.
- **PDF output** — Cairo can render to PDF surfaces, but that is a separate
  backend (not covered here).

### Venture v0.1 Minimum Viable Set

For the initial Venture browser (HTML 1.0 rendering), only a subset of handlers
is required. The rest can be stubs that log a warning and skip the instruction:

| Handler    | v0.1 Status | Why                                          |
|-----------|-------------|----------------------------------------------|
| rect      | Required    | Background colors, borders, block elements   |
| glyph_run | Required    | All text rendering                           |
| group     | Required    | Transform stacking for layout                |
| line      | Required    | Underlines, borders, `<hr>`                  |
| clip      | Required    | Overflow clipping                            |
| image     | Required    | `<img>` elements                             |
| ellipse   | Deferred    | Not used in HTML 1.0                         |
| path      | Deferred    | Not used in HTML 1.0                         |
| gradient  | Deferred    | CSS gradients are post-HTML 1.0              |
| layer     | Deferred    | Opacity compositing is post-HTML 1.0         |

---

## glyph_run Routing by font_ref Scheme

**This section supersedes the brief `glyph_run` handler descriptions
in the per-backend handler mapping tables above. It was added after
the TXT-series (TXT00–TXT05) and FNT02/FNT03 were specified; those
specs establish the font-binding invariant that paint backends MUST
honor when dispatching glyph runs.**

Every `PaintGlyphRun` instruction carries a `font_ref: string`
field. The string's prefix before the first colon is the **font
binding scheme**, and it tells the paint backend which glyph-
rasterization pipeline to use. Crossing bindings is undefined
behaviour and MUST throw `UnsupportedFontBindingError`.

### The four scheme prefixes in v0.1

| Prefix            | Glyph IDs produced by         | Paint backend MUST use                                         |
|-------------------|-------------------------------|----------------------------------------------------------------|
| `font-parser:`    | TXT02 (naive) / TXT04 (full)  | FNT02 glyph-parser → FNT03 rasterizer → native image blit      |
| `coretext:`       | TXT03a CoreText shaper        | CoreText's `CTFontDrawGlyphs` (or `CGContextShowGlyphsAtPositions`) |
| `directwrite:`    | TXT03b DirectWrite shaper     | Direct2D's `ID2D1RenderTarget::DrawGlyphRun` with matching `IDWriteFontFace` |
| `pango:`          | TXT03c Pango shaper           | Cairo's `pango_cairo_show_glyph_string` with matching `PangoFont` |

Each paint backend registers a set of recognized prefixes at
creation time and rejects any other prefix with
`UnsupportedFontBindingError`. This is the same failure discipline
as PaintVM's `UnknownInstructionError` (P2D01): loud failure
beats silent wrong output.

### Dispatch algorithm for glyph_run

```
fn handle_glyph_run(run: &PaintGlyphRun, ctx: &mut PlatformContext):
    scheme, key = split_once(run.font_ref, ':')
    match scheme:
        "font-parser":
            // Device-independent path.
            // The backend must have the font bytes registered under
            // this key (caller's responsibility; typically done once
            // at startup via a shared FontRegistry).
            font = ctx.font_registry.lookup(key) or throw
            for glyph in run.glyphs:
                outline = fnt02::glyph_outline(&font, glyph.glyph_id)
                bitmap  = fnt03::rasterize(
                    &outline, font.units_per_em, run.font_size,
                    subpixel_origin = (
                      (run.x + glyph.x_offset) - floor(run.x + glyph.x_offset),
                      (run.y + glyph.y_offset) - floor(run.y + glyph.y_offset),
                    ))
                platform_blit_grayscale(
                    ctx,
                    bitmap,
                    run.x + glyph.x_offset,
                    run.y + glyph.y_offset,
                    tint = run.fill,
                )

        "coretext":
            // macOS / iOS native path.
            // key identifies a CTFontRef the backend holds.
            ct_font = ctx.coretext_registry.lookup(key) or throw
            let cgglyphs: Vec<CGGlyph> = run.glyphs.iter().map(|g| g.glyph_id as CGGlyph)
            let positions: Vec<CGPoint> = run.glyphs.iter().map(|g| CGPoint {
                x: run.x + g.x_offset,
                y: run.y + g.y_offset,
            })
            CGContextSetFillColor(ctx.cg, run.fill)
            CTFontDrawGlyphs(ct_font, cgglyphs.as_ptr(), positions.as_ptr(),
                             run.glyphs.len(), ctx.cg)

        "directwrite":
            // Windows native path. (P2D06-direct2d only.)
            dw_face = ctx.directwrite_registry.lookup(key) or throw
            let d2d_glyph_run = DWRITE_GLYPH_RUN {
                fontFace:      dw_face,
                fontEmSize:    run.font_size,
                glyphCount:    run.glyphs.len(),
                glyphIndices:  run.glyphs.map(|g| g.glyph_id as u16).as_ptr(),
                glyphAdvances: ...,  // compute from x_offset deltas
                glyphOffsets:  ...,  // per-glyph (x,y) offsets
                isSideways:    false,
                bidiLevel:     0,
            }
            ctx.d2d_render_target.DrawGlyphRun(
                D2D_POINT_2F { x: run.x, y: run.y },
                &d2d_glyph_run,
                ctx.brush_for(run.fill),
                DWRITE_MEASURING_MODE_NATURAL,
            )

        "pango":
            // Linux native path. (P2D08-cairo only.)
            pango_font = ctx.pango_registry.lookup(key) or throw
            // Build a PangoGlyphString from run.glyphs
            // and call pango_cairo_show_glyph_string().

        _:
            throw UnsupportedFontBindingError { scheme }
```

### Why the registry lookup

Each backend maintains a runtime registry mapping font_ref keys
to the live font handle it needs (a parsed FontFile for
font-parser; a CTFontRef for coretext; an IDWriteFontFace for
directwrite; a PangoFont for pango). The registry is populated
at application startup, typically by the same code that built
the FontResolver (TXT05) — both use the same underlying font
source.

The paint backend does NOT perform font resolution. It only
looks up the already-resolved handle by its ref. This keeps the
paint layer stateless with respect to font discovery — every
font a scene references must be pre-registered. A scene that
names an unregistered font fails the lookup, which raises
`UnsupportedFontBindingError` with a message identifying the
missing key. (Strictly, this is a separate error variant
`FontRefNotRegistered`, but sharing the same exception simplifies
callers.)

### Per-backend support matrix for v0.1

| Backend         | font-parser | coretext  | directwrite | pango     |
|-----------------|-------------|-----------|-------------|-----------|
| paint-metal     | Supported   | Supported | —           | —         |
| paint-vm-direct2d | Supported | —         | Supported   | —         |
| paint-vm-gdi    | Supported   | —         | —           | —         |
| paint-vm-cairo  | Supported   | —         | —           | Supported |
| paint-vm-canvas | Supported   | —         | —           | —         |
| paint-vm-svg    | Supported   | —         | —           | —         |
| paint-vm-terminal | —         | —         | —           | —         |

Every backend supports `font-parser:` (the device-independent
path) since FNT02 + FNT03 compile everywhere. Native schemes are
available only on their platforms. The terminal backend does not
render glyphs via any shaper — it uses its own codepoint-to-cell
logic and does not accept PaintGlyphRun at the pixel level. (A
future variant may support it for rendering Unicode characters in
monospace cells; outside the scope of v0.1.)

### Composition: the font-parser path in detail

The `font-parser:` dispatch deserves extra explanation because
it's the cross-platform, reproducible path — the one LaTeX-style
rendering depends on.

For each glyph in the run:

1. **Resolve the outline.** Call `fnt02::glyph_outline(&font,
   glyph.glyph_id)`. Returns a `GlyphOutline` with `MoveTo` /
   `LineTo` / `QuadTo` commands in design units.

2. **Rasterize to coverage.** Call `fnt03::rasterize(...)` with
   the font's `units_per_em`, the run's `font_size`, and a
   subpixel origin computed from the fractional part of the
   glyph's absolute position. Returns a single-channel 8-bit
   `GlyphBitmap`.

3. **Tint and composite.** The bitmap is grayscale coverage;
   multiply by `run.fill` (with alpha premultiplication) and
   alpha-blend over the target surface at the glyph's integer-
   rounded position. The platform-specific blit is:
   - Metal: upload the grayscale bitmap as an R8Unorm texture,
     draw a textured quad with `fill` as the tint color.
   - Direct2D: `CreateBitmap` with R8, `DrawBitmap` (not ideal;
     D2D prefers BGRA — a convert-and-blit path works).
   - GDI: `CreateDIBSection` with 8bpp palette, `AlphaBlend`.
   - Cairo: `cairo_mask_surface` with the coverage as the mask
     and `fill` as the source color.
   - Canvas: `createImageData` populated with tinted RGBA derived
     from the coverage, then `putImageData`.

4. **Subpixel snapping and caching.** Rasterizing the same glyph
   at the same size and same subpixel origin repeatedly is
   wasteful; backends SHOULD maintain a glyph bitmap cache keyed
   on `(font_ref, glyph_id, font_size, rounded_subpixel_origin)`.
   FNT03's open question on snapping granularity applies here —
   1/4-pixel is the recommended default.

The device-dependent paths (`coretext:`, `directwrite:`, `pango:`)
bypass FNT02 and FNT03 entirely; the OS does the outline lookup
and rasterization internally. This is why **the paint backend
must not mix bindings** — a CoreText glyph ID is not a valid
input to FNT02's `glyph_outline`, and a font-parser glyph ID is
not a valid input to `CTFontDrawGlyphs`.

---
