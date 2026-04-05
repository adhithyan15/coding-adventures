# UI02 — Layout IR: The Universal Layout Intermediate Representation

## Overview

Layout IR is the **universal intermediate representation for layout** in the
coding-adventures stack. It sits between producers that know what content to
display and layout algorithms that know how to arrange and size it.

```
Producer (Mosaic IR, DocumentAST, LaTeX IR, spreadsheet, game UI)
    ↓  front-end converter (mosaic-ir-to-layout, document-ast-to-layout, ...)
  LayoutNode tree                         ← this spec (UI02)
    ↓  layout algorithm (layout-flexbox, layout-block, layout-grid, layout-tex)
  PositionedNode tree
    ↓  layout-to-paint (UI04)
  PaintScene                              ← feeds into paint-vm (P2D01)
    ↓  renderer (paint-vm-canvas, paint-vm-svg, paint-vm-metal, ...)
  pixels / vectors / text
```

### Design principles

**The IR knows nothing about its producer.** A `LayoutNode` tree built from a
Mosaic component looks identical in structure to one built from a Markdown
document. The layout algorithm only sees `LayoutNode`.

**The IR knows nothing about its layout algorithm.** Nodes carry properties that
any algorithm may find useful. Each algorithm reads what it needs and ignores the
rest. If you pass a flexbox-annotated tree to the grid algorithm, the grid
algorithm will run fine — the output may not be what you intended, but that is
the caller's responsibility.

**Extension over restriction.** The core node carries the minimal set of
properties that every algorithm needs. Each layout package extends the node
through an open `ext` map, adding its own schema without modifying the core
type. The core type never gains algorithm-specific fields.

**No smartness.** The IR does not validate that the right layout algorithm was
chosen, does not auto-detect algorithm from content, and does not produce
warnings when properties are ignored. It is a dumb data structure.

---

## Package: `layout-ir`

This package exports pure types and a small set of builder helper functions.
It has **zero runtime dependencies** and zero I/O. Every other layout and paint
package depends on this one.

---

## Core Types

### `SizeValue`

A size value for width or height. Three variants:

```
SizeValue =
  | { kind: "fixed",   value: float }   // exact logical units
  | { kind: "fill" }                    // fill available space (like CSS flex: 1)
  | { kind: "wrap" }                    // shrink to fit content (like CSS fit-content)
```

"Logical units" are abstract — the layout engine works in them throughout. The
renderer maps logical units to physical pixels by applying a device pixel ratio
or point scale.

---

### `Edges`

Four-sided spacing value, used for both padding and margin:

```
Edges {
  top:    float   // default 0
  right:  float   // default 0
  bottom: float   // default 0
  left:   float   // default 0
}
```

Builder helpers:
- `edges_all(v)` → `{ top: v, right: v, bottom: v, left: v }`
- `edges_xy(x, y)` → `{ top: y, right: x, bottom: y, left: x }`
- `edges_zero()` → `{ top: 0, right: 0, bottom: 0, left: 0 }`

---

### `Color`

An RGBA color value with components in the range 0–255:

```
Color {
  r: int   // 0–255
  g: int   // 0–255
  b: int   // 0–255
  a: int   // 0–255, 255 = fully opaque
}
```

Builder helpers:
- `rgba(r, g, b, a)` → `Color`
- `rgb(r, g, b)` → `Color` with `a = 255`
- `color_transparent()` → `{ r: 0, g: 0, b: 0, a: 0 }`

---

### `FontSpec`

A fully specified font descriptor. All fields are concrete values — no CSS
shorthand, no cascade, no inheritance. Every `TextContent` carries a complete
`FontSpec`.

```
FontSpec {
  family:      string    // font family name, e.g. "Helvetica", "Arial"
                         // empty string = system default UI font
  size:        float     // in logical units (same coordinate space as layout)
  weight:      int       // 100–900, CSS font-weight scale
  italic:      bool      // true = italic / oblique
  lineHeight:  float     // line height multiplier, e.g. 1.5 = 150% of size
                         // must be > 0
}
```

Builder helpers:
- `font_spec(family, size)` → FontSpec with weight=400, italic=false, lineHeight=1.2
- `font_bold(spec)` → copy with weight=700
- `font_italic(spec)` → copy with italic=true

**Note on units:** `size` is in logical units, not CSS pixels or typographic
points. The renderer is responsible for converting to physical units using the
device pixel ratio or a platform-specific scale factor. A layout engine that
calls a `TextMeasurer` passes the `FontSpec` as-is; the `TextMeasurer`
implementation knows the physical scale and applies it internally before
returning measurement results in logical units.

---

### `TextAlign`

Horizontal alignment of text within its containing box:

```
TextAlign = "start" | "center" | "end"
```

"start" and "end" are logical (respect writing direction). Renderers map them
to "left"/"right" for LTR and "right"/"left" for RTL. This spec covers LTR only;
RTL support is a future extension.

---

### `ImageFit`

How an image fills its containing box:

```
ImageFit = "contain" | "cover" | "fill" | "none"
```

Mirrors CSS `object-fit`. `contain` — letterbox. `cover` — crop. `fill` —
stretch. `none` — natural size, clipped if larger.

---

### `TextContent`

Inline text content carried by a leaf node:

```
TextContent {
  kind:      "text"
  value:     string       // the text string to render
  font:      FontSpec
  color:     Color
  maxLines:  int?         // null = unlimited; wraps at containing width
  textAlign: TextAlign    // default "start"
}
```

---

### `ImageContent`

Image content carried by a leaf node:

```
ImageContent {
  kind:    "image"
  src:     string     // URL, data URI, or opaque renderer-specific handle
  fit:     ImageFit   // default "contain"
}
```

---

### `LayoutNode`

The core layout node. This is the central type of the entire layout system.

```
LayoutNode {
  // ─── Identity ───────────────────────────────────────────────
  id:        string?    // optional stable identifier for diffing and debugging

  // ─── Content ────────────────────────────────────────────────
  // Leaf nodes carry content. Container nodes have children instead.
  // A node may have content OR children, not both.
  content:   TextContent | ImageContent | null

  // ─── Children ───────────────────────────────────────────────
  children:  LayoutNode[]    // empty list for leaf nodes

  // ─── Size hints ─────────────────────────────────────────────
  // All optional. null means "no constraint from this property".
  // The layout algorithm decides what to do with missing hints.
  width:     SizeValue?
  height:    SizeValue?
  minWidth:  float?
  maxWidth:  float?
  minHeight: float?
  maxHeight: float?

  // ─── Spacing ─────────────────────────────────────────────────
  padding:   Edges?      // space inside the node's border
  margin:    Edges?      // space outside the node's border

  // ─── Extension bag ───────────────────────────────────────────
  // Each layout algorithm defines its own extension schema.
  // Fields are namespaced by algorithm (e.g. ext["flex"], ext["grid"]).
  // Algorithms read their own namespace and ignore everything else.
  // Unknown keys are silently ignored — no validation, no errors.
  ext:       map<string, any>    // {} by default
}
```

**Content vs children invariant:** A node with non-null `content` should have
an empty `children` list. A node with children should have null `content`. The
layout engine does not enforce this invariant — it reads `content` for leaf
measurement and `children` for subtree recursion. If both are set, behavior is
algorithm-defined (most algorithms will ignore `content` on container nodes).

---

### `Constraints`

The available space passed into a layout call. Represents the maximum size the
layout is allowed to occupy:

```
Constraints {
  minWidth:  float    // minimum available width, usually 0
  maxWidth:  float    // maximum available width; Float.MAX_VALUE = unconstrained
  minHeight: float    // minimum available height, usually 0
  maxHeight: float    // maximum available height; Float.MAX_VALUE = unconstrained
}
```

Builder helpers:
- `constraints_fixed(w, h)` → fixed box
- `constraints_width(w)` → fixed width, unconstrained height
- `constraints_unconstrained()` → Float.MAX_VALUE in all dimensions
- `constraints_shrink(c, horizontal, vertical)` → reduce constraints by amounts

---

### `PositionedNode`

The output of a layout pass. Every node now has a concrete position and size in
the same logical unit coordinate space as the input `LayoutNode`.

```
PositionedNode {
  // ─── Resolved geometry ──────────────────────────────────────
  x:         float    // left edge, relative to parent's content area origin
  y:         float    // top edge, relative to parent's content area origin
  width:     float    // resolved width in logical units
  height:    float    // resolved height in logical units

  // ─── Content and identity (carried through from LayoutNode) ──
  id:        string?
  content:   TextContent | ImageContent | null

  // ─── Children ───────────────────────────────────────────────
  children:  PositionedNode[]

  // ─── Extension bag (carried through unchanged) ───────────────
  ext:       map<string, any>
}
```

`x` and `y` are **relative to the parent's content area origin** (i.e. the
parent's top-left corner after padding is applied). `layout-to-paint` performs
the recursive accumulation to absolute coordinates when building `PaintScene`.

---

### `TextMeasurer` interface

Every layout algorithm takes a `TextMeasurer` as a parameter. This interface
is defined here in `layout-ir` so that all packages share the same contract
without creating circular dependencies.

```
TextMeasurer {
  measure(
    text:     string,
    font:     FontSpec,
    maxWidth: float | null    // null = unconstrained (single line)
  ) → MeasureResult
}

MeasureResult {
  width:     float    // measured width in logical units
  height:    float    // measured height in logical units
  lineCount: int      // number of lines if text wraps within maxWidth
}
```

The `TextMeasurer` returns measurements in **logical units**. The implementation
is responsible for any internal conversion between logical units and physical
device units before returning.

Implementations live in separate packages (`layout-text-measure-estimated`,
`layout-text-measure-canvas`, `layout-text-measure-rs`) so that the layout
algorithms never import a backend.

---

## Extension schemas

Each layout algorithm package defines its own extension schema. The schema
describes which keys in `ext` it reads, what types they have, and what defaults
it uses when a key is absent.

Extension keys are namespaced strings. The convention is:

```
ext["flex"]   → FlexExt   (defined in layout-flexbox, spec UI03)
ext["block"]  → BlockExt  (defined in layout-block, spec UI07)
ext["grid"]   → GridExt   (defined in layout-grid, spec UI08)
ext["tex"]    → TexExt    (defined in layout-tex, spec UI10)
```

A node may carry extension data for multiple algorithms — for example, a node
that is a flex child inside a flex container AND itself lays out its children
as a grid would carry both `ext["flex"]` (flex child properties like `grow`)
and `ext["grid"]` (grid container properties like `templateColumns`).

---

## Builder helpers summary

The package exports these constructor helpers alongside the types:

```
// Edges
edges_all(v: float) → Edges
edges_xy(x: float, y: float) → Edges
edges_zero() → Edges

// Color
rgba(r, g, b, a: int) → Color
rgb(r, g, b: int) → Color
color_transparent() → Color

// FontSpec
font_spec(family: string, size: float) → FontSpec
font_bold(spec: FontSpec) → FontSpec
font_italic(spec: FontSpec) → FontSpec

// SizeValue
size_fixed(v: float) → SizeValue
size_fill() → SizeValue
size_wrap() → SizeValue

// Constraints
constraints_fixed(w: float, h: float) → Constraints
constraints_width(w: float) → Constraints
constraints_unconstrained() → Constraints
constraints_shrink(c: Constraints, dw: float, dh: float) → Constraints

// LayoutNode
node(opts) → LayoutNode     // all fields optional, sensible defaults
leaf_text(content: TextContent, opts?) → LayoutNode
leaf_image(content: ImageContent, opts?) → LayoutNode
container(children: LayoutNode[], opts?) → LayoutNode
```

---

## What this package does NOT contain

- No layout algorithm logic
- No paint or render logic
- No font loading or text measurement
- No dependency on any backend (Canvas, Metal, DOM, SVG)
- No validation that ext fields are correct for any algorithm
- No version negotiation or schema evolution logic

All of the above live in downstream packages.

---

## Packages that depend on `layout-ir`

Every layout, paint bridge, front-end converter, and text measurer package
depends on `layout-ir`. It is the shared vocabulary of the entire layout and
paint subsystem.
