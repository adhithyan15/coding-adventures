# UI04 — layout-to-paint: PositionedNode Tree → PaintScene

## Overview

`layout-to-paint` is a pure translation step. It walks a `PositionedNode` tree
and produces a `PaintScene` (defined in `paint-instructions`, spec P2D00) that
can be executed by any `paint-vm` backend.

```
PositionedNode[]              ← output of any layout algorithm
    ↓  layout_to_paint()
PaintScene                    ← input to paint-vm (P2D01)
    ↓  PaintVM.execute(scene, ctx)
pixels / vectors / native draw calls
```

This package is the bridge between the layout subsystem and the paint subsystem.
It has no knowledge of algorithms, flexbox, or the original source that produced
the layout. It only knows how to turn positioned geometry + content into paint
instructions.

---

## Package: `layout-to-paint`

**Depends on:** `layout-ir`, `paint-instructions`

**Exports:** `layout_to_paint`, `LayoutToPaintOptions`

---

## Function signature

```
layout_to_paint(
  nodes:   PositionedNode[],
  options: LayoutToPaintOptions
) → PaintScene
```

### `LayoutToPaintOptions`

```
LayoutToPaintOptions {
  width:       float        // scene viewport width in logical units
  height:      float        // scene viewport height in logical units
  background:  Color?       // scene background fill; null = transparent
  devicePixelRatio: float   // default 1.0; physical pixels per logical unit
                            // applied to all coordinates and sizes before
                            // emitting PaintInstructions

  // --- Required for text content ---
  //
  // A text shaper (TXT00) that converts strings into positioned glyph
  // runs. The layout engine MUST supply one if any PositionedNode in
  // the input tree carries TextContent. If a TextContent node is
  // encountered without a shaper in options, layout_to_paint throws
  // MissingTextShaperError.
  //
  // The shaper's binding (e.g. "font-parser:<hash>", "coretext:<id>")
  // propagates into every emitted PaintGlyphRun via the font_ref field.
  // Paint backends route glyph_run dispatch on this prefix — see P2D06.
  shaper:      TextShaper?

  // A font metrics provider (TXT00) paired with the shaper. Used to
  // compute baseline positions from the top-of-bbox position that the
  // layout produces. Must be backend-compatible with the shaper (same
  // FontHandle type). Optional only when shaper is also null.
  metrics:     FontMetrics?

  // A font resolver (TXT05) that maps CSS-style font family/weight/
  // style queries to FontHandle values the shaper understands. Used
  // once per distinct TextContent.font combination per call.
  // Optional only when shaper is also null.
  resolver:    FontResolver?
}
```

The `shaper`/`metrics`/`resolver` trio MUST share a font binding —
they are a matched set. A caller using the device-independent path
passes a `font-parser`-backed trio; a caller on macOS native passes
a CoreText-backed trio (TXT03a). They are not mix-and-matchable
across backends; that would violate the font-binding invariant
(see TXT00 §"The font-binding invariant").

---

## Coordinate transformation

All coordinates and sizes in `PositionedNode` are in **logical units**. All
coordinates and sizes in `PaintInstruction` are in **physical units** (pixels
for Canvas, points for Metal).

The transformation is:

```
physical = logical × devicePixelRatio
```

Applied uniformly to: `x`, `y`, `width`, `height`, font `size`, padding,
corner radii, border widths, and shadow offsets.

`layout_to_paint` performs this multiplication once when emitting each
instruction. Downstream packages (paint-vm backends) receive physical units
and do not need to know the device pixel ratio.

---

## Algorithm

### Step 1 — Initialize scene

Create a `PaintScene` with:
- `width = options.width × devicePixelRatio`
- `height = options.height × devicePixelRatio`
- `background = options.background` (passed through as-is; paint-vm handles fill)

### Step 2 — Walk the PositionedNode tree

Perform a depth-first pre-order walk. For each node, compute its **absolute
position** by accumulating parent offsets recursively:

```
abs_x = parent_abs_x + node.x
abs_y = parent_abs_y + node.y
```

The root nodes start at `abs_x = 0, abs_y = 0`.

### Step 3 — Emit instructions per node

For each node, emit `PaintInstruction` values in this order:

1. **Background fill** — if `ext["paint"]` carries a `backgroundColor`, emit
   a `PaintRect` covering the node's full bounds
2. **Border** — if `ext["paint"]` carries `borderWidth > 0`, emit a stroked
   `PaintRect` over the full bounds
3. **Clip** — if `ext["paint"]` carries a `cornerRadius`, push a `PaintClip`
   with rounded rectangle path before children, pop after
4. **Content** — if `node.content` is non-null, emit the content instruction
   (see below)
5. **Children** — recurse for each child

### Step 4 — Content instructions

#### `TextContent` → `PaintGlyphRun`

**This subsection supersedes the earlier "top-of-bbox y / text /
maxWidth / align" description that shipped before the TXT-series
was specified.** PaintGlyphRun as defined in P2D00 is a *pre-shaped*
instruction: glyph IDs and positions are baked in by the layout
engine using the caller-supplied TextShaper. The paint backend only
rasterizes; it never sees the source string, never re-measures,
never re-wraps.

The conversion:

```
1. Resolve the font once per distinct TextContent.font combination:
     handle = resolver.resolve(FontQuery {
       family_names: [content.font.family, ...fallbacks],
       weight:       content.font.weight,
       style:        Italic if content.font.italic else Normal,
       stretch:      Normal,
     })

2. Shape the text:
     run = shaper.shape(
       content.value,
       handle,
       content.font.size × dpr,
       ShapeOptions { script: null, language: null,
                      direction: "ltr", features: {} }
     )

3. Compute baseline origin from top-of-bbox + ascender:
     units_per_em  = metrics.units_per_em(handle)
     ascent_scaled = metrics.ascent(handle) × content.font.size × dpr
                   / units_per_em
     baseline_x    = abs_x × dpr
     baseline_y    = abs_y × dpr + ascent_scaled

4. Bake per-glyph absolute x_offset from the shaper's per-glyph
   pen-relative adjustments (see TXT00 §"Relationship to P2D00
   PaintGlyphRun" — the shaper returns adjustments relative to a
   running pen; PaintGlyphRun.glyphs[i].x_offset is absolute from
   the baseline origin):

     pen = 0
     for i in 0..run.glyphs.len():
         glyph_absolute_x_offset[i] = pen + run.glyphs[i].x_offset
         pen += run.glyphs[i].x_advance

5. Emit:
     PaintGlyphRun {
       kind:      "glyph_run",
       x:         baseline_x,
       y:         baseline_y,
       font_ref:  run.font_ref,               // verbatim from shaper
       font_size: content.font.size × dpr,
       glyphs:    run.glyphs.map((g, i) => ({
         glyph_id: g.glyph_id,
         x_offset: glyph_absolute_x_offset[i],
         y_offset: g.y_offset,
       })),
       fill:      content.color,
     }
```

**Multi-line text.** If the shaper was called with a `max_width`
parameter and produced a line-broken run (layout engines may use
the `line-breaker` package — FNT04/TXT06 — for this), the layout
engine calls `shape()` once per line and emits one PaintGlyphRun
per line, stacking baseline origins vertically by
`metrics.line_gap + metrics.ascent + metrics.descent`.

**Text alignment.** The old `align` field is gone. Alignment
(`left` / `center` / `right` / `justify`) is resolved by the layout
engine *before* calling this function: the `abs_x` position of the
TextContent node already reflects the alignment choice. A centered
line at x=0 in a 200-wide container with run width 120 has
`abs_x = 40` coming in; layout-to-paint does not re-center.

**Why the rewrite.** The earlier version of this section predated
the TXT-series and carried a string-plus-font-plus-maxWidth shape
that paint backends had to re-shape at render time, forcing every
backend to embed a shaper. That design is now deprecated; all
shaping happens in the layout engine (once, using the
caller-supplied shaper) and paint backends only ever see
pre-positioned glyph IDs.

#### `ImageContent` → `PaintImage`

```
PaintImage {
  x:      abs_x × dpr
  y:      abs_y × dpr
  width:  node.width × dpr
  height: node.height × dpr
  src:    content.src
  fit:    content.fit
}
```

### Step 5 — Return PaintScene

The scene's `instructions` list is the flat, ordered sequence of all emitted
`PaintInstruction` values in pre-order traversal order. The order determines
paint order: earlier instructions are drawn first (behind later ones).

---

## `ext["paint"]` — optional per-node visual properties

`layout-to-paint` reads a special extension namespace `ext["paint"]` to
determine visual decoration properties that do not affect layout but do affect
paint output. These are set by front-end converters (e.g. `mosaic-ir-to-layout`)
when they know the node needs a background color or border.

```
PaintExt {
  backgroundColor: Color?      // fills the node's full bounds
  borderWidth:     float?      // stroke width; 0 or null = no border
  borderColor:     Color?      // stroke color; used only if borderWidth > 0
  cornerRadius:    float?      // rounded corners for clip and border
  opacity:         float?      // 0.0–1.0; null = 1.0 (fully opaque)
  shadowColor:     Color?      // drop shadow; null = no shadow
  shadowOffsetX:   float?      // shadow horizontal offset in logical units
  shadowOffsetY:   float?      // shadow vertical offset in logical units
  shadowBlur:      float?      // shadow blur radius in logical units
}
```

If `ext["paint"]` is absent or a field is null, no corresponding instruction is
emitted for that property.

---

## Opacity and layer handling

If `ext["paint"]["opacity"]` is set to a value less than 1.0:

1. Push a `PaintLayer` instruction before the node's background/content/children
2. Set the layer opacity to the specified value
3. Pop the `PaintLayer` after all children are emitted

This ensures opacity composites correctly across the entire subtree.

---

## What this package does NOT do

- Does not run any layout algorithm — positions must already be resolved
- Does not load fonts, images, or other resources (fonts are resolved
  via the caller-supplied `resolver`; this package does not fetch
  bytes, only turns FontQueries into handles)
- Does not validate that the `PositionedNode` tree is well-formed
- Does not clip children to their parent's bounds by default (use
  `ext["paint"]["cornerRadius"]` or add explicit clip nodes in the tree)
- Does not handle scrolling or interactive hit regions
- Does not produce accessibility metadata
- Does not shape or measure text directly — delegates to the
  caller-supplied `shaper` and `metrics` (TXT00 interfaces). See
  Step 4 above.

---

## Error conditions

| Error                      | When                                                             |
|----------------------------|------------------------------------------------------------------|
| `MissingTextShaperError`   | A TextContent node was encountered but options.shaper is null    |
| `MissingFontMetricsError`  | A TextContent node was encountered but options.metrics is null   |
| `MissingFontResolverError` | A TextContent node was encountered but options.resolver is null  |
| `FontBindingMismatchError` | options.shaper/metrics/resolver do not share a font binding      |
| (propagated)               | Errors from resolver.resolve(), shaper.shape(), metrics.*()      |

The first three are programmer errors — the caller built an
options bundle without the text trio but passed in text content.
`FontBindingMismatchError` is a cross-binding misuse (e.g., a
CoreText resolver paired with a font-parser shaper). In
statically-typed languages the binding-mismatch check is a
compile-time type error via the generic `<H>` parameter on all
three traits; dynamic languages check at runtime.
