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
}
```

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

```
PaintGlyphRun {
  x:        abs_x × dpr
  y:        abs_y × dpr
  text:     content.value
  font:     {
    family: content.font.family,
    size:   content.font.size × dpr,
    weight: content.font.weight,
    italic: content.font.italic
  }
  color:    content.color
  maxWidth: node.width × dpr     // clip/wrap text to node width
  align:    content.textAlign
}
```

The `y` coordinate is the **top** of the text bounding box (not the baseline).
PaintVM backends that need baseline positioning (e.g. Canvas `fillText`)
must add the ascender metric internally. This is backend responsibility.

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
- Does not load fonts, images, or other resources
- Does not validate that the `PositionedNode` tree is well-formed
- Does not clip children to their parent's bounds by default (use
  `ext["paint"]["cornerRadius"]` or add explicit clip nodes in the tree)
- Does not handle scrolling or interactive hit regions
- Does not produce accessibility metadata
