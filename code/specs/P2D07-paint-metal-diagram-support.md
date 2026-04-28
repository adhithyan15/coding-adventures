# P2D07 — Paint Metal: Diagram Rendering Support

## Overview

`paint-metal` (the Rust Metal GPU renderer introduced in P2D01) currently handles
only `PaintRect`, `PaintLine`, `PaintGroup`, and `PaintClip`. The DG01 diagram
pipeline lowers DOT graphs into `PaintScene` instructions that include
`PaintEllipse`, `PaintPath`, and `PaintGlyphRun` — all currently stubs that
are silently skipped.

This spec defines exactly what must be implemented in `paint-metal` and
`paint-instructions` to achieve end-to-end DOT diagram rendering on Apple Metal.

```text
PaintScene (DG01 output)
  ├── PaintRect          ✅ already implemented
  ├── PaintLine          ✅ already implemented
  ├── PaintGroup         ✅ already implemented
  ├── PaintClip          ✅ (partial, sufficient for diagrams)
  ├── PaintEllipse       ← this spec (ellipse nodes)
  ├── PaintPath          ← this spec (edges, arrowheads, diamond nodes)
  └── PaintGlyphRun      ← this spec (node and edge labels)
```

---

## Part A — `paint-instructions` changes

### A1. Dashed stroke fields

Add optional dash-pattern support to all strokable instructions.

```rust
// Add to PaintPath:
pub stroke_dash:        Option<Vec<f64>>,  // dash pattern, e.g. [6.0, 3.0]
pub stroke_dash_offset: Option<f64>,        // offset into the pattern

// Add to PaintLine:
pub stroke_dash:        Option<Vec<f64>>,
pub stroke_dash_offset: Option<f64>,

// Add to PaintRect:
pub stroke_dash:        Option<Vec<f64>>,
pub stroke_dash_offset: Option<f64>,

// Add to PaintEllipse:
pub stroke_dash:        Option<Vec<f64>>,
pub stroke_dash_offset: Option<f64>,
```

`stroke_dash = None` means a solid stroke (current behavior, unchanged).
`stroke_dash = Some([6.0, 3.0])` means 6px dash, 3px gap, repeating.
`stroke_dash_offset` shifts the start position within the pattern.

These fields are required by DG00 §"Must-have paint additions" and enable:
- Mermaid dashed links
- UML dependency edges
- Lane separators in waveform diagrams

Backends that do not yet implement dash support MUST silently fall back to
a solid stroke rather than erroring.

---

## Part B — `paint-metal` implementation

### B1. `PaintEllipse` — triangle fan tessellation

Metal does not have a native ellipse primitive. The implementation tessellates
the ellipse into a fan of `N = 64` triangles sharing a common centre point.

```text
Ellipse with centre (cx, cy), radii (rx, ry):

For i in 0..N:
    θ₀ = 2π × i / N
    θ₁ = 2π × (i+1) / N
    v0 = (cx, cy)                              centre
    v1 = (cx + rx×cos(θ₀), cy + ry×sin(θ₀))  outer edge start
    v2 = (cx + rx×cos(θ₁), cy + ry×sin(θ₁))  outer edge end

Each triangle [v0, v1, v2] is one GPU primitive.
```

For **fill**: all 64 triangles share the fill colour.

For **stroke**: produce a second ring of `N` quads (each a pair of triangles)
that form the outline ring between radius `r - stroke_width/2` and
`r + stroke_width/2`. For non-circular ellipses, the stroke width is
approximate (constant in parameter space).

If both `fill` and `stroke` are set, emit fill triangles first, then stroke
ring on top.

If `fill` is `None` or `"none"`, skip fill triangles.
If `stroke` is `None` or `"none"`, skip stroke ring.

**Vertex format:** same as existing `PaintRect` — position (x, y) + colour (r, g, b, a)
as `f32`. The existing `solid_colour_pipeline` can render ellipse triangles
without any shader changes.

### B2. `PaintPath` — CPU tessellation

`PaintPath` is the most complex instruction. The CPU tessellates it into
triangles before uploading to the GPU.

#### B2a. Fill

Use an **ear-clipping polygon triangulation** for closed filled paths
(i.e., paths that end with `PathCommand::Close`).

Algorithm:
1. Walk all `PathCommand`s to produce a flat `Vec<(f64, f64)>` of vertices.
   - `MoveTo` → start a new sub-path
   - `LineTo` → append a vertex
   - `QuadTo` → subdivide into ≈8 `LineTo` segments via de Casteljau
   - `CubicTo` → subdivide into ≈16 `LineTo` segments
   - `ArcTo` → approximate with ≈16 `LineTo` segments
   - `Close` → close the polygon (connect last point to first)
2. Apply ear-clipping on the resulting polygon to produce `(N-2)` triangles.
3. Upload as fill-coloured triangles to the existing pipeline.

For the diagram use case (arrowheads, diamonds), paths are simple convex
polygons (3–6 vertices). The ear-clipping algorithm handles these trivially.

#### B2b. Stroke

For **open paths** (no `Close`) and optionally for closed paths when a stroke
colour is set, produce a "tube" of quads along the polyline.

Algorithm (Miter-less rounded caps):
1. Walk the polyline vertices produced in step B2a.
2. For each segment `(p0 → p1)`:
   - Compute perpendicular normal `n = perp(normalise(p1 - p0)) × stroke_width / 2`
   - Emit a quad: `[p0−n, p0+n, p1+n, p1−n]` → two triangles
3. For each interior join (shared vertex between two segments):
   - Bevel join: clip overlapping quads at the mid-angle between segments.
     For v1 diagrams the simple overlapping-quad approach is acceptable.
4. For line caps (`stroke_cap`):
   - `"round"` → semi-circle fan at each end (using the ellipse fan algorithm
     with `rx = ry = stroke_width/2`)
   - `"square"` → extend each end quad by `stroke_width/2`
   - `"butt"` (default) → no extension

#### B2c. Dash pattern

If `stroke_dash` is `Some(pattern)`, sub-divide each segment into on/off
intervals before emitting stroke quads. Walk the segment length, alternating
between "draw" and "skip" intervals from the pattern. `stroke_dash_offset`
shifts the starting phase.

#### B2d. Integration

`PaintPath` is tessellated on the CPU during the vertex-collection pass
(alongside `PaintRect` and `PaintLine`). The resulting triangles go into the
same vertex buffer and are rendered by the existing `solid_colour_pipeline`.

No new Metal shaders or pipeline states are required for phases B1 and B2.

### B3. `PaintGlyphRun` — CoreText integration

The `glyph_run_overlay` module already implements CoreText-based text rendering
as a post-processing overlay. It:

1. Renders the `PaintScene` (rects, lines) to a `PixelContainer` via the GPU.
2. Wraps the pixel data in a `CGBitmapContext`.
3. Calls `CTFontDrawGlyphs` for each `PaintGlyphRun` with `font_ref` starting
   `"coretext:"`.
4. The glyph pixels are composited directly into the bitmap.

**What this spec adds:** the `diagram-to-paint` crate (DG01) emits
`PaintGlyphRun` with `font_ref = "coretext:Helvetica@<size>"` and glyph IDs
derived from character codepoints (one glyph per character, no ligatures).

For node labels and edge labels in v1, no complex shaping is required — DOT
labels are plain ASCII. `CTFontGetGlyphsForCharacters` maps `UniChar` to
`CGGlyph` directly.

The `diagram-to-paint` crate therefore produces `PaintGlyphRun` with:

```rust
PaintGlyphRun {
    font_ref: format!("coretext:Helvetica@{}", font_size),
    font_size,
    x: label_x,
    y: label_y,
    glyphs: text.chars().enumerate().map(|(i, ch)| GlyphEntry {
        glyph_id: ch as u32,   // Unicode codepoint; CoreText resolves to CGGlyph
        x_offset: i as f64 * approx_char_advance,
        y_offset: 0.0,
    }).collect(),
    fill: style.text_color.clone(),
}
```

The `glyph_run_overlay` code is already capable of rendering this — the
integration is complete once `diagram-to-paint` emits the correct `font_ref`
scheme.

---

## Part C — Coordinate system

The existing `glyph_run_overlay` uses a Y-flip to convert from `PaintScene`
(Y-down) to CoreText (Y-up):

```rust
// existing code in draw_one_glyph_run:
CGContextTranslateCTM(ctx, 0.0, image_height);
CGContextScaleCTM(ctx, 1.0, -1.0);
```

The `diagram-to-paint` label positions follow the same Y-down convention as
all other paint instructions. No changes are needed here.

---

## Part D — Testing

### D1. Ellipse unit tests

- `PaintEllipse` with fill only → pixel at centre should match fill colour.
- `PaintEllipse` with stroke only → pixel at radius should match stroke colour;
  centre should be background.
- `PaintEllipse` with both → centre = fill, ring = stroke.

### D2. Path unit tests

- Filled triangle (3-vertex closed path) → pixels inside triangle = fill colour.
- Diamond (4-vertex closed path) → pixels at corners and centre correct.
- Open polyline stroke → pixels along centre of stroke = stroke colour.
- Arrowhead (same as filled triangle) → covered by D2 filled triangle test.

### D3. GlyphRun integration test

- Single ASCII character rendered at a known position → expected pixel region
  is non-background-white (exact pixels are font-dependent; test for non-blank).

### D4. Full diagram smoke test

Parse a three-node DOT graph through the complete pipeline
(`parse_to_diagram` → `layout_graph_diagram` → `diagram_to_paint` → `render`)
and assert:

- `PixelContainer` is non-empty.
- Width and height match the layout dimensions.
- At least one non-white pixel exists (graph was rendered).

---

## Out of scope for this spec

- `PaintLayer` (offscreen texture + opacity composite) — deferred to P2D08.
- `PaintGradient` (MSL gradient shader) — deferred to P2D08.
- `PaintImage` — deferred to P2D08.
- Miter joins — bevel joins are sufficient for v1 diagrams.
- Complex text shaping (ligatures, BiDi, kerning) — CoreText handles Latin
  ASCII correctly without explicit shaping for diagram labels.
- Radial ellipse stroke exactness — constant parameter-space width acceptable.
