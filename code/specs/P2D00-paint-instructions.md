# P2D00 — PaintInstructions: The 2D Paint Intermediate Representation

## Overview

PaintInstructions is the **universal intermediate representation** for 2D rendering
in the coding-adventures stack. It sits between producers that generate visual
content and backends that know how to render it.

```
Producer (game, chart, barcode renderer, Mermaid diagram)
  → PaintInstructions IR  (this spec, P2D00)
  → PaintVM               (dispatch-table VM, P2D01)
  → Backend (SVG, Canvas, Metal, Direct2D, Cairo, terminal)
```

### When you do NOT need PaintInstructions

If your target is a **browser** and your layout engine produces **HTML/CSS/React
elements**, you can skip PaintInstructions entirely. Go straight from layout to the
DOM:

```
Layout Engine
  → HTML + CSS + React elements
  → Browser DOM
  → Browser paints (GPU compositing, dirty tracking, accessibility — all free)
```

The browser's rendering engine (Blink, WebKit, Gecko) is itself a full paint
backend. It handles everything below layout for you: GPU rasterization, layer
compositing, dirty region tracking, scrolling, and the accessibility tree. React
is a reconciler that drives that DOM efficiently — it is already a retained-mode
scene graph with diffing built in.

PaintInstructions exists for the cases where the browser DOM is **not available**
or **not appropriate**:

- **Native apps** — Metal (macOS/iOS), Direct2D (Windows), Cairo (Linux). No DOM.
- **Server-side rendering** — generating SVG or PDF on a server process.
- **Games** — Canvas or WebGL, bypassing the DOM for performance.
- **Electron apps** — want Canvas-level control without DOM overhead.
- **Terminal renderers** — box-drawing characters, no GPU.
- **Cross-platform libraries** — one codebase targeting browser Canvas AND
  native Metal AND server-side SVG simultaneously.

If your stack is browser-only and layout-driven, the HTML/React path is the right
choice. PaintInstructions is not a replacement for the DOM — it is the abstraction
for everything else.

---

The IR is deliberately **ignorant in both directions**:

- It knows nothing about SVG. There is no `<g>` element here, no `stroke-dasharray`,
  no DOM node. A PaintGroup is not an SVG group — it's a logical grouping that
  *some backend* may choose to render as a `<g>`.
- It knows nothing about the game or chart that produced it. A barcode renderer
  emits PaintRect instructions for bars. The IR doesn't know they're bars.

This deliberate ignorance is what makes the IR composable. A new backend (say,
a PDF renderer) can consume any producer's output without touching the producer.
A new producer (say, a musical score renderer) can target any backend without
knowing anything about SVG or Metal.

### Why a new IR? What happened to DrawInstructions?

The original `draw-instructions.md` spec defined `DrawRect`, `DrawText`,
`DrawGroup`, `DrawLine`, `DrawClip`, and `DrawScene`. It was the right design
for the barcode use case — simple, focused, easy to implement.

PaintInstructions extends it in three ways:

1. **More instruction types**: ellipses, arbitrary paths, glyph runs, gradients,
   images. These are needed for charts, diagrams, and general-purpose 2D rendering.

2. **Float coordinates everywhere**: Draw* used integers (suitable for pixel-aligned
   barcodes). Paint* uses `f64`/`number` throughout. Sub-pixel accuracy matters for
   text, curves, and scaled graphics.

3. **Optional IDs for patch() diffing**: Each instruction can carry a stable `id`
   string. The PaintVM uses these IDs to diff two scenes efficiently, updating only
   what changed — the same lesson as React's `key=` prop.

The `Draw*` types in the old spec are superseded. Implementors should migrate to
`Paint*` types. The old spec is preserved for historical reference until the
implementation PR.

---

## Instruction Type Reference

Every instruction in this IR is one of the types listed in the union below.
Each type has a `kind` discriminant (a string literal) so that dispatch tables
and pattern-match arms can identify the instruction without `instanceof` checks.

```typescript
type PaintInstruction =
  | PaintRect
  | PaintEllipse
  | PaintPath
  | PaintGlyphRun
  | PaintGroup
  | PaintLayer
  | PaintLine
  | PaintClip
  | PaintGradient
  | PaintImage;
```

### PaintBase — The shared fields on every instruction

```typescript
// Every instruction extends PaintBase.
// These fields are optional on every instruction type.
interface PaintBase {
  id?: string;
  // A stable, opaque identity string for this instruction.
  // Used by PaintVM.patch() to track instructions across scene versions.
  // Recommended format: UUID v4 (e.g. "550e8400-e29b-41d4-a716-446655440000").
  // Short stable strings work too for layout anchors ("chart-title", "x-axis").
  // Instructions without an id fall back to positional diffing in patch().
  // See the "id field and patch() contract" section below.

  metadata?: Record<string, string | number | boolean>;
  // Arbitrary key/value pairs for producers and debuggers.
  // Examples: { "source": "mermaid-node-42", "layer": "background" }
  // The PaintVM ignores metadata — it's carried through unchanged.
  // Backends may expose it for dev-tools or accessibility annotations.
}
```

---

### PaintRect — Filled and/or stroked rectangle

```typescript
interface PaintRect extends PaintBase {
  kind: "rect";
  x: number;            // left edge, in user-space units (float)
  y: number;            // top edge, in user-space units (float)
  width: number;        // must be >= 0
  height: number;       // must be >= 0
  fill?: string;        // CSS color string, e.g. "#ff0000", "rgba(0,0,0,0.5)"
                        // omitted or null means no fill (transparent)
  stroke?: string;      // CSS color string for the border
                        // omitted or null means no stroke
  stroke_width?: number; // line width in user-space units; default 1.0
  corner_radius?: number; // uniform corner radius for rounded rects
                          // 0 or omitted means sharp corners
}
```

**Field notes:**

- `x, y` are the top-left corner in the current coordinate system. After a
  transform on a parent PaintGroup, these are in the group's local space.
- `fill` and `stroke` use CSS color syntax. Every backend must be able to parse
  at minimum: named colors (`"red"`, `"transparent"`), hex (`"#rgb"`, `"#rrggbb"`,
  `"#rrggbbaa"`), and `rgba(r, g, b, a)`.
- A PaintRect with `fill: "url(#grad-1)"` references a PaintGradient by id.
  See PaintGradient below.
- `corner_radius` applies uniformly to all four corners. If you need per-corner
  radii, use a PaintPath with cubic_to commands.

**Example — a blue card with a white border:**

```typescript
{
  kind: "rect",
  id: "card-bg",
  x: 10, y: 10, width: 200, height: 120,
  fill: "#2563eb",
  stroke: "#ffffff",
  stroke_width: 2,
  corner_radius: 8
}
```

---

### PaintEllipse — Filled and/or stroked ellipse or circle

```typescript
interface PaintEllipse extends PaintBase {
  kind: "ellipse";
  cx: number;           // center x (not the left edge — the center)
  cy: number;           // center y
  rx: number;           // x radius (half-width); must be >= 0
  ry: number;           // y radius (half-height); must be >= 0
  fill?: string;
  stroke?: string;
  stroke_width?: number;
}
```

**Field notes:**

- A circle is an ellipse with `rx === ry`. There is no separate PaintCircle type.
- `cx, cy` is the geometric center, not the bounding-box origin. This matches the
  SVG `<ellipse>` convention and differs from `<rect>` which uses the top-left.
- The bounding box of a PaintEllipse is:
  `{ x: cx - rx, y: cy - ry, width: 2*rx, height: 2*ry }`

**ASCII diagram — ellipse anatomy:**

```
         (cx, cy - ry)   ← top of ellipse
              |
(cx-rx, cy) --+-- (cx+rx, cy)   ← left and right extremes
              |
         (cx, cy + ry)   ← bottom of ellipse

The ellipse passes through all four extreme points.
```

---

### PaintPath — Arbitrary vector path

A PaintPath encodes a sequence of drawing commands that trace an arbitrary
2D shape. It is the most expressive instruction type — any shape that can be
expressed in SVG `<path d="...">` can be expressed here.

```typescript
// A single drawing command. Think of this as one step of a pen plotter:
// lift the pen (move_to), draw a straight line (line_to), draw a curve, close.
type PathCommand =
  | { kind: "move_to"; x: number; y: number }
  // Lift the pen and move to (x, y) without drawing.
  // Every subpath begins with a move_to.

  | { kind: "line_to"; x: number; y: number }
  // Draw a straight line from the current point to (x, y).

  | { kind: "quad_to"; cx: number; cy: number; x: number; y: number }
  // Draw a quadratic Bezier curve to (x, y) with control point (cx, cy).
  // The curve is pulled toward (cx, cy) but does not pass through it.

  | { kind: "cubic_to";
      cx1: number; cy1: number;   // first control point
      cx2: number; cy2: number;   // second control point
      x: number; y: number }      // endpoint
  // Draw a cubic Bezier curve to (x, y) with two control points.
  // This is the most common smooth curve in professional graphics.

  | { kind: "arc_to";
      rx: number; ry: number;       // ellipse radii
      x_rotation: number;           // rotation of ellipse x-axis, in degrees
      large_arc: boolean;           // choose the larger arc (true) or smaller (false)
      sweep: boolean;               // draw clockwise (true) or counterclockwise (false)
      x: number; y: number }        // endpoint
  // Draw an elliptical arc from the current point to (x, y).
  // Matches SVG arc command semantics exactly.

  | { kind: "close" }
  // Draw a straight line back to the most recent move_to point,
  // closing the current subpath. Makes fills work correctly for closed shapes.

interface PaintPath extends PaintBase {
  kind: "path";
  commands: PathCommand[];
  fill?: string;
  fill_rule?: "nonzero" | "evenodd";
  // Determines how overlapping subpaths are filled.
  // "nonzero" (default): a point is inside if the winding number is nonzero.
  // "evenodd": a point is inside if the number of path crossings is odd.
  // Matters for shapes with holes (donuts, star polygons, letters with counters).
  stroke?: string;
  stroke_width?: number;
  stroke_cap?: "butt" | "round" | "square";
  // How line endpoints are drawn.
  // "butt" (default): flat cap, exactly at the endpoint.
  // "round": semicircular cap, extending beyond the endpoint by stroke_width/2.
  // "square": square cap, extending beyond the endpoint by stroke_width/2.
  stroke_join?: "miter" | "round" | "bevel";
  // How line corners are drawn when two segments meet.
  // "miter" (default): sharp pointed join (can be very long at shallow angles).
  // "round": rounded join.
  // "bevel": flat diagonal join (truncates the miter).
}
```

#### PathCommand explained with an ASCII diagram

Here is a simple "house" shape traced step by step:

```
     C (100, 20)         ← apex of roof
    / \
   /   \
  /     \
 A       B               A = (60, 60), B = (140, 60)
 |       |
 |       |
 D-------E               D = (60, 120), E = (140, 120)
```

Commands to draw it:

```typescript
[
  { kind: "move_to", x: 60,  y: 120 },   // start at D (bottom-left)
  { kind: "line_to", x: 60,  y: 60  },   // up to A
  { kind: "line_to", x: 100, y: 20  },   // diagonal to apex C
  { kind: "line_to", x: 140, y: 60  },   // diagonal to B
  { kind: "line_to", x: 140, y: 120 },   // down to E
  { kind: "close"                    },   // back to D (closes the bottom)
]
```

#### Relationship to G2D02 (CubicBezier) and G2D03 (SvgArc)

The `cubic_to` command encodes exactly the cubic Bezier segment from G2D02:

```
cubic_to { cx1, cy1, cx2, cy2, x, y }
  ↕  corresponds to  ↕
CubicBezier {
  p0: (current point),
  p1: (cx1, cy1),   // first control point
  p2: (cx2, cy2),   // second control point
  p3: (x, y)        // endpoint
}
```

A PaintPath's cubic_to is the wire-format representation of a CubicBezier.
To apply G2D02's `split()`, `bounding_box()`, or `arc_length()` operations
to a PaintPath, extract the current point and the cubic_to fields, construct
a CubicBezier, apply the operation, then serialize back.

The `arc_to` command uses the same parameterization as G2D03's `SvgArc`:

```
arc_to { rx, ry, x_rotation, large_arc, sweep, x, y }
  ↕  corresponds to  ↕
SvgArc {
  rx, ry,
  x_axis_rotation: x_rotation,
  large_arc_flag: large_arc,
  sweep_flag: sweep,
  x1: (current point x),
  y1: (current point y),
  x2: x,
  y2: y,
}
```

G2D03's `to_beziers()` method converts an SvgArc to a sequence of CubicBezier
segments. Backends that don't natively support elliptical arcs (e.g. terminal)
can call `arc_to_beziers()` and substitute `cubic_to` commands in place of the
`arc_to`.

---

### PaintGlyphRun — Pre-shaped glyph run

```typescript
interface PaintGlyphRun extends PaintBase {
  kind: "glyph_run";
  x: number;        // baseline origin x — the left edge of the first glyph's advance
  y: number;        // baseline origin y — NOT the top of the text, but the baseline
  font_ref: string; // opaque font identifier
                    // may be a URI: "file:///fonts/NotoSans-Regular.ttf"
                    // may be a name: "NotoSans-Regular"
                    // may be a content hash: "sha256:abc123..."
                    // the backend is responsible for resolving this to a usable font
  font_size: number; // in user-space units (same units as x, y)
  glyphs: Array<{
    glyph_id: number;  // numeric glyph ID as returned by font-parser's glyph_id()
                       // NOT a Unicode codepoint — a font-internal index
    x_offset: number;  // horizontal distance from baseline origin for this glyph
                       // in user-space units (already scaled by font_size)
    y_offset: number;  // vertical offset from baseline (positive = down in screen coords)
                       // used for superscript (+) and subscript (-)
  }>;
  fill: string;     // color of the glyphs — required (no default for text)
}
```

**ASCII diagram — baseline and offsets:**

```
Baseline origin (x, y) is marked with ↓

     x_offset →
↓    |
H    e  l  l  o    ²
|              ↑
|              y_offset < 0 (raised for superscript)
|
The baseline is the line all letters "sit on".
Descenders (g, p, y, j) hang below it.
Ascenders (h, l, d, b) rise above it.
```

#### GlyphRun vs Text — why there is no PaintText

A beginner reading this spec might ask: why `PaintGlyphRun` instead of a simpler
`PaintText { x, y, text: string, font_name: string, font_size: number }`?

The answer is that *text rendering is a two-phase pipeline*:

**Phase 1 — Shaping (done by the layout engine, NOT the paint layer):**

1. Map Unicode codepoints to font glyph IDs. The character 'A' might be glyph 36
   in one font and glyph 4 in another. This requires the font's `cmap` table
   (font-parser, FNT00).
2. Apply kerning — adjust the space between specific glyph pairs ('A' and 'V'
   should sit closer together than 'A' and 'B'). This requires the font's `kern`
   or `GPOS` table.
3. Apply ligature substitution — 'fi' becomes a single ligature glyph in many
   professional fonts. This requires the font's `GSUB` table.
4. Compute advance widths — how far to move the pen after each glyph. This requires
   the font's `hmtx` table.
5. Break text into lines (line wrapping). This requires knowing the width of the
   container.

**Phase 2 — Rendering (done by the paint layer):**

1. For each glyph: look up the glyph outline (contour data) by glyph ID.
2. Draw the outline at the given position and size.

The paint layer is a **one-way pipe**: it executes, it does not query. It cannot
ask "how wide is this string?" because that would require font metric lookups,
which would make the backend stateful. A PaintVM that has registered no font
resources would deadlock.

By the time a PaintGlyphRun reaches the PaintVM, all the hard work is done.
The `glyphs` array is the resolved output of the layout phase. The paint layer
just places each glyph at its pre-computed position.

This is the same design used by PDF (glyph positions are embedded in the file),
OpenType `GDEF`/`GPOS`, and every professional typesetting system.

---

### PaintGroup — Hierarchical grouping with optional transform and opacity

```typescript
interface PaintGroup extends PaintBase {
  kind: "group";
  children: PaintInstruction[];
  // The instructions inside this group. Can be any mix of instruction types,
  // including nested PaintGroups.

  transform?: [number, number, number, number, number, number];
  // An affine2d matrix in the form [a, b, c, d, e, f].
  // Applies to all children. The transformation is:
  //   x' = a*x + c*y + e
  //   y' = b*x + d*y + f
  // This is the standard SVG/Canvas affine matrix representation.
  // See G2D01 (affine2d) for the full specification.
  // Omit for identity transform (no transformation).

  opacity?: number;
  // Uniform opacity applied to the entire group. Range 0.0 (invisible) to 1.0
  // (fully opaque). Default 1.0.
  // Note: this is group-level compositing opacity, not per-pixel alpha.
  // It renders the group to an offscreen buffer, then composites with this opacity.
}
```

**The transform matrix — a cheat sheet:**

```
Identity (no transform):          [1, 0, 0, 1, 0, 0]
Translate by (tx, ty):            [1, 0, 0, 1, tx, ty]
Scale by (sx, sy):                [sx, 0, 0, sy, 0, 0]
Rotate by θ radians:              [cos θ, sin θ, -sin θ, cos θ, 0, 0]
Flip horizontally:                [-1, 0, 0, 1, 0, 0]

Composition: apply child's matrix after parent's.
```

**Example — a chart legend translated to the top-right corner:**

```typescript
{
  kind: "group",
  id: "legend",
  transform: [1, 0, 0, 1, 480, 20],  // translate to (480, 20)
  opacity: 0.9,
  children: [
    { kind: "rect", x: 0, y: 0, width: 100, height: 60, fill: "white", stroke: "#ccc" },
    { kind: "rect", x: 8, y: 8, width: 12, height: 12, fill: "#2563eb" },
    // ... more legend items
  ]
}
```

---

### PaintLine — Straight line segment

```typescript
interface PaintLine extends PaintBase {
  kind: "line";
  x1: number;       // start point x
  y1: number;       // start point y
  x2: number;       // end point x
  y2: number;       // end point y
  stroke: string;   // required — a line with no stroke is invisible
  stroke_width: number;  // required — default is ambiguous; be explicit
  stroke_cap?: "butt" | "round" | "square";
  // Controls how the line endpoints are drawn.
  // See PaintPath.stroke_cap for the full explanation.
}
```

Note that `stroke` and `stroke_width` are **required** on PaintLine (not optional
like on PaintRect). A line is nothing but a stroke — requiring the stroke fields
catches producer bugs at schema validation time rather than producing invisible output.

---

### PaintClip — Rectangular clipping region

```typescript
interface PaintClip extends PaintBase {
  kind: "clip";
  x: number;
  y: number;
  width: number;
  height: number;
  // The clip rectangle. Any pixels of children that fall outside this rectangle
  // are not drawn. The clip is in the current coordinate space (after parent transforms).
  children: PaintInstruction[];
  // The instructions to render inside the clip region.
}
```

**Use cases:**

- Table cells: clip text so it doesn't overflow into adjacent cells.
- Scroll containers: clip content to the visible viewport.
- Spark lines: clip chart lines to the chart area.

The clipping shape is always a rectangle. For non-rectangular clipping
(e.g., clip to a circle or to a path), use a PaintPath with `clip-rule` — this is
a backend-specific extension that may be added in a future spec revision.

---

### PaintGradient — A gradient paint source

```typescript
type GradientStop = {
  offset: number;   // position along the gradient axis, range 0.0 to 1.0
                    // 0.0 = start of gradient, 1.0 = end of gradient
  color: string;    // CSS color at this stop
};

interface PaintGradient extends PaintBase {
  kind: "gradient";
  gradient_kind: "linear" | "radial";

  // For linear gradients: the gradient goes from (x1, y1) to (x2, y2).
  // Pixels perpendicular to this axis have the same color.
  x1?: number; y1?: number;   // start point of gradient axis
  x2?: number; y2?: number;   // end point of gradient axis

  // For radial gradients: the gradient goes from the center (cx, cy)
  // outward to radius r. At r, the final stop color is reached.
  cx?: number; cy?: number; r?: number;

  stops: GradientStop[];
  // At least 2 stops required. Stops should be sorted by offset ascending.
  // Behavior for unsorted stops is backend-defined.
}
```

**Important — gradients are paint sources, not rendered objects:**

A PaintGradient is NOT rendered directly. It defines a paint source that other
instructions reference by its `id`. Think of it as a variable definition that
stores a color recipe.

To use a gradient:
1. Define the PaintGradient with an `id`:
   ```typescript
   { kind: "gradient", id: "blue-fade", gradient_kind: "linear",
     x1: 0, y1: 0, x2: 200, y2: 0,
     stops: [
       { offset: 0.0, color: "#2563eb" },
       { offset: 1.0, color: "#93c5fd" },
     ] }
   ```
2. Reference it from a fill or stroke using `url(#id)` syntax:
   ```typescript
   { kind: "rect", x: 0, y: 0, width: 200, height: 50, fill: "url(#blue-fade)" }
   ```

This matches SVG's `<linearGradient id="...">` + `fill="url(#...)"` pattern.
Backends that don't support `url()` references in fill strings must resolve the
gradient inline.

**ASCII diagram — linear gradient stops:**

```
offset: 0.0                     1.0
         |                       |
gradient axis → → → → → → → → →
         |______|_________|_______|
         stop1  stop2     stop3  stop4
         (blue) (indigo)  (violet)(pink)

Between stops, color is interpolated linearly in sRGB space.
```

---

### PaintImage — Raster or SVG image

```typescript
interface PaintImage extends PaintBase {
  kind: "image";
  x: number;        // top-left x of the image rectangle
  y: number;        // top-left y
  width: number;    // rendered width (may differ from intrinsic image width)
  height: number;   // rendered height
  src: string;      // image source — one of:
                    //   URI:      "https://example.com/photo.jpg"
                    //   file URI: "file:///assets/logo.png"
                    //   data URL: "data:image/png;base64,iVBORw0K..."
                    // The backend resolves the src. The IR does not validate it.
  opacity?: number; // 0.0 (invisible) to 1.0 (fully opaque), default 1.0
}
```

---

### PaintLayer — Offscreen compositing surface

`PaintLayer` is fundamentally different from `PaintGroup`. A group is a
logical container for transform/clip inheritance — it renders directly into
the parent surface. A layer allocates a **separate offscreen buffer**, renders
its children into that buffer, applies filters to it as a unit, then composites
the result back into the parent surface using a blend mode.

This is the same model as Photoshop layers, CSS `filter` + `mix-blend-mode`,
and SVG `<filter>` elements. The key insight: filters that modify pixel values
(blur, drop shadow, color matrix) must operate on the layer as a whole after
all children have been painted — they cannot be applied per-instruction.

```typescript
interface PaintLayer extends PaintBase {
  kind: "layer";
  children: PaintInstruction[];
  // Instructions rendered into the offscreen buffer, back-to-front.
  // Children may themselves be layers (nesting is allowed).

  filters?: FilterEffect[];
  // Zero or more filter effects applied to the composited offscreen buffer.
  // Applied in array order (each filter receives the output of the previous).
  // Empty array or undefined = no filtering.

  blend_mode?: BlendMode;
  // How the layer's offscreen buffer is composited back into the parent.
  // Default: "normal" (standard alpha compositing / painter's algorithm).

  opacity?: number;
  // 0.0 (invisible) to 1.0 (fully opaque).
  // Applied after filters, before compositing.
  // Default: 1.0.

  transform?: Transform2D;
  // Optional 2D affine transform applied to the layer as a whole.
  // Same format as PaintGroup.transform.
}
```

#### FilterEffect union

```typescript
type FilterEffect =
  | { kind: "blur";         radius: number }
    // Gaussian blur. radius in user-space units.

  | { kind: "drop_shadow";  dx: number; dy: number; blur: number; color: string }
    // Drop shadow offset by (dx, dy), blurred by `blur` radius, filled with `color`.

  | { kind: "color_matrix"; matrix: number[] }
    // 4x5 color matrix (20 values, row-major). Maps [R, G, B, A, 1] → [R', G', B', A'].
    // Same matrix layout as SVG feColorMatrix type="matrix".

  | { kind: "brightness";   amount: number }
    // Multiply luminance. 1.0 = no change, 0.0 = black, 2.0 = double brightness.

  | { kind: "contrast";     amount: number }
    // Adjust contrast. 1.0 = no change, 0.0 = flat grey, 2.0 = high contrast.

  | { kind: "saturate";     amount: number }
    // Adjust color saturation. 0.0 = greyscale, 1.0 = no change, 2.0 = vivid.

  | { kind: "hue_rotate";   angle: number }
    // Rotate hue. angle in degrees. 180 = complement.

  | { kind: "invert";       amount: number }
    // Invert colors. 0.0 = no change, 1.0 = fully inverted.

  | { kind: "opacity";      amount: number };
    // Premultiplied opacity filter. 0.0 = transparent, 1.0 = opaque.
    // Distinct from PaintLayer.opacity: this is a filter in the pipeline,
    // while .opacity is applied after all filters as a final multiplier.
```

#### BlendMode

```typescript
type BlendMode =
  // Separable modes — operate per colour channel independently
  | "normal"       // Standard alpha compositing (default)
  | "multiply"     // Multiply source and destination — darkens
  | "screen"       // Invert × multiply × invert — lightens
  | "overlay"      // Multiply for darks, Screen for lights
  | "darken"       // min(src, dst) per channel
  | "lighten"      // max(src, dst) per channel
  | "color_dodge"  // Divide dst by (1 − src) — brightens
  | "color_burn"   // Invert dst, divide by src, invert — darkens
  | "hard_light"   // Overlay with src and dst swapped
  | "soft_light"   // Softer version of hard_light
  | "difference"   // |src − dst| — high contrast edges
  | "exclusion"    // Like difference but lower contrast

  // Non-separable modes — operate on the combined HSL representation
  | "hue"          // src hue + dst saturation + dst luminosity
  | "saturation"   // dst hue + src saturation + dst luminosity
  | "color"        // src hue + src saturation + dst luminosity
  | "luminosity";  // dst hue + dst saturation + src luminosity
```

#### Backend complexity tiers

Not all backends support `PaintLayer` equally. Filter and blend mode support
requires either native browser API support or full GPU compute pipelines.

| Backend        | Support level                                                          |
|----------------|------------------------------------------------------------------------|
| HTML5 Canvas   | **Native** — `ctx.filter`, `ctx.globalCompositeOperation`, `ctx.save()`/`restore()` |
| SVG            | **Native** — `<filter>`, `<feGaussianBlur>`, `<feBlend>`, `mix-blend-mode` |
| Metal / Vulkan | **Full, GPU** — allocate MTLTexture / VkImage for offscreen; apply compute shaders for filters; blend via fragment shader |
| Direct2D       | **Native** — `ID2D1Effect` for filters; composition via `D2D1_COMPOSITE_MODE` |
| Cairo          | **Partial** — `cairo_push_group()` / `cairo_pop_group_to_source()` for layers; filters require pixman or manual convolution |
| Terminal       | **Degraded** — render children as if PaintGroup (no filters, no blend mode); warn once per session |

Metal and Vulkan backends require offscreen texture allocation and per-pixel
compute shaders to implement the full filter list. These implementations live
in `paint-vm-metal` / `paint-vm-vulkan` (future P2D05+). A Canvas or SVG backend
can implement PaintLayer in ~20 lines using the browser's native compositing APIs.

#### Masking (deferred)

Path-based masking (e.g. SVG `<mask>`, Canvas `ctx.clip()` with complex path,
Metal `stencil buffer`) is **explicitly out of scope for P2D00 v0.1.0**. It will
be designed in a future spec (P2D08 or later) as a `PaintMask` instruction type
that references a layer by id. Masking is deferred because it introduces a
non-trivial dependency graph between instructions (a mask must be fully rendered
before it can be applied), which requires the VM to do a two-pass render.

---

### PaintScene — The top-level container

```typescript
interface PaintScene {
  width: number;        // viewport width in user-space units
  height: number;       // viewport height in user-space units
  background: string;   // CSS color — painted before all instructions
                        // use "transparent" for no background fill
  instructions: PaintInstruction[];
  // The ordered list of paint instructions. Rendered back-to-front (painter's
  // algorithm): earlier instructions are drawn first (further back).
  id?: string;
  // Optional scene identity for the patch() API. If two scenes have the same id,
  // PaintVM.patch() can use it to assert that they're versions of the same scene.
  metadata?: Record<string, string | number | boolean>;
}
```

---

## The id Field and patch() Contract

Every instruction has an optional `id` field. This section explains the contract.

### Why IDs matter

Imagine a bar chart that is updated 60 times per second as data streams in.
Without IDs, the PaintVM must redraw the entire chart on every frame — hundreds
of rectangles, text labels, and lines. With IDs, the VM can compare the previous
frame's scene with the new frame's scene, find the three bars that changed height,
and update only those. The other 97 bars are left untouched.

This is the retained-mode rendering model, as opposed to immediate-mode (redraw
everything). Retained mode is faster for complex, mostly-stable scenes.

### The diffing contract

```
Rules for PaintVM.patch():

1. Nodes with id are stable-identity nodes.
   - If a node with id X exists in both old and new scenes, it is the SAME node
     and may have been modified.
   - If a node with id X exists in old but not new, it was DELETED.
   - If a node with id X exists in new but not old, it was INSERTED.

2. Nodes without id are positional (ephemeral) nodes.
   - They are diffed by their position in the instructions array.
   - Position 0 in old corresponds to position 0 in new.
   - If the new scene has more instructions at a position than old, those are
     inserted. If fewer, those are deleted.
   - Positional diffing is correct for static geometry (e.g., a fixed border).
   - Positional diffing is fragile for dynamic lists — if a new item is prepended,
     every node after it is seen as "changed", causing unnecessary redraws.

3. The id is scoped to the scene, not globally unique across scenes.
   But using UUIDs eliminates any risk of cross-scene collisions.
```

### Practical advice

- Use UUID v4 for instructions generated by dynamic loops (chart bars, table rows,
  list items, graph nodes).
- Use short stable strings for structural anchors ("x-axis", "chart-title",
  "legend-box"). These are easier to read in debug output.
- Instructions you know will never change (static decorations, borders) can omit
  `id` — the positional diff will handle them correctly as long as they're always
  in the same position.

### The React key= lesson

React developers learned this lesson painfully: lists without `key=` props cause
wrong diffs, missing animations, and duplicated state. PaintInstructions has the
same issue with positional diffing. The cure is the same: assign stable IDs to
items in dynamic lists.

```
// BAD — these bars will be positionally diffed
bars.map(b => ({ kind: "rect", x: b.x, y: b.y, ... }))

// GOOD — each bar has a stable identity
bars.map(b => ({ kind: "rect", id: b.id, x: b.x, y: b.y, ... }))
```

---

## Backend Mapping Table

The following table shows how each PaintInstruction maps to common rendering backends.
This is indicative, not normative — each backend spec (P2D02–P2D05) is authoritative
for its own backend.

| PaintInstruction | SVG                          | HTML5 Canvas                        | Metal (via Rust FFI)                 | Terminal/ASCII         |
|------------------|------------------------------|-------------------------------------|--------------------------------------|------------------------|
| PaintRect        | `<rect>`                     | `ctx.fillRect()` / `ctx.strokeRect()` | `MTLRenderCommandEncoder` fill rect | `#` characters in grid |
| PaintEllipse     | `<ellipse>`                  | `ctx.ellipse()`                     | MTL path with arc segments           | `*` approximation      |
| PaintPath        | `<path d="...">`             | `Path2D` + `ctx.fill()`             | `MTLPath` + `addCurve()`             | bezier → line segments |
| PaintGlyphRun    | `<text>` + glyph positioning | `ctx.drawImage(glyph bitmap)`       | Core Text / Metal texture atlas      | Unicode codepoint map  |
| PaintGroup       | `<g transform="...">`        | `ctx.save()` + `transform()` + `ctx.restore()` | push/pop command buffer | recurse into children |
| PaintLine        | `<line>`                     | `ctx.moveTo()` + `ctx.lineTo()`     | MTL path line segment                | `-`, `|`, `/`, `\`     |
| PaintClip        | `<clipPath>` + `clip-path`   | `ctx.clip()`                        | `pushClip()` / `popClip()`           | bounds check per char  |
| PaintGradient    | `<linearGradient>` / `<radialGradient>` | `ctx.createLinearGradient()` | Metal shader gradient          | ignored / fallback     |
| PaintImage       | `<image>`                    | `ctx.drawImage()`                   | Metal texture sampling               | `[img]` placeholder    |
| PaintLayer       | `<g filter="...">` / `<filter>` | `ctx.filter` + `ctx.save()`/`restore()` | Offscreen MTLTexture + compute shaders | render as PaintGroup (filters ignored) |

---

## Non-Goals

PaintInstructions explicitly does NOT handle:

**Text layout** — Line breaking, word wrapping, justification, hyphenation, and
bidirectional text are layout concerns. By the time a PaintGlyphRun reaches the IR,
all layout decisions are finalized.

**Font loading** — The `font_ref` field is an opaque string. PaintInstructions
does not know how to load a font file, parse it, or cache it. That is the backend's
responsibility (or the producer's, if it passes pre-loaded glyph outlines).

**Animation** — The IR represents one frame. For animation, the producer calls
`execute()` each frame with updated values. The IR itself has no notion of time,
easing, tweening, or keyframes.

**Variable resolution** — All coordinates, colors, and sizes in the IR are
concrete resolved values. There are no `var(--color)` CSS variables, no
expressions, no conditionals. Variables are a producer concern.

**Scene graph diffing** — The IR is a flat (but nested) list of instructions.
The diff algorithm lives in PaintVM.patch() (P2D01). A higher-level retained
scene graph lives in P2D06.

---

## P2D Series Roadmap

| Spec  | Package             | Description                                        |
|-------|---------------------|----------------------------------------------------|
| P2D00 | paint-instructions  | IR types — this spec                               |
| P2D01 | paint-vm            | Dispatch-table VM base (execute + patch)           |
| P2D02 | paint-vm-svg        | SVG backend — renders to SVG DOM/string            |
| P2D03 | paint-vm-canvas     | HTML5 Canvas backend — renders to CanvasRenderingContext2D |
| P2D04 | paint-vm-terminal   | Terminal/ASCII backend — renders to StringBuffer   |
| P2D05 | paint-vm-metal      | Apple Metal backend via Rust FFI (includes PaintLayer offscreen, compute shaders) |
| P2D06 | scene-graph         | Retained-mode scene graph with automatic patch()   |
| P2D07 | hit-tester          | Geometry-based hit testing; returns path from root to hit node |
| P2D08 | tessellator         | Bézier → triangle lists for raw GPU backends (depends on bezier2d, point2d) |

The specs are designed so that P2D00 (this spec) is the only shared dependency.
Each backend (P2D02–P2D05) depends on P2D00 and P2D01 only. They do not depend
on each other. A project that only needs SVG rendering includes only P2D00,
P2D01, and P2D02.
