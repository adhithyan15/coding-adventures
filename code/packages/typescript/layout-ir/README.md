# @coding-adventures/layout-ir

**Universal Layout Intermediate Representation** — the shared vocabulary
between content producers and layout algorithms in the coding-adventures stack.

## What is it?

`layout-ir` is a pure types package. It defines the data structures that flow
through the layout pipeline:

```
Producer (Mosaic IR, DocumentAST, LaTeX IR, ...)
    ↓  front-end converter
  LayoutNode tree         ← this package defines LayoutNode
    ↓  layout algorithm (layout-flexbox, layout-block, layout-grid)
  PositionedNode tree     ← this package defines PositionedNode
    ↓  layout-to-paint
  PaintScene
    ↓  renderer (Canvas, Metal, SVG, ...)
  pixels
```

This package has **zero runtime dependencies** and performs no I/O.

## Core types

| Type | Description |
|---|---|
| `LayoutNode` | Input to a layout algorithm. Carries size hints, spacing, content, and an open `ext` bag for algorithm-specific data. |
| `PositionedNode` | Output of a layout algorithm. Every node has a resolved `x`, `y`, `width`, `height`. |
| `SizeValue` | Width/height hint: `fixed(v)`, `fill`, or `wrap`. |
| `Constraints` | Available space passed into a layout call. |
| `TextMeasurer` | Interface for text measurement (injected into layout algorithms). |
| `FontSpec` | Fully-resolved font descriptor (no CSS cascade). |
| `TextContent` | Text leaf node content. |
| `ImageContent` | Image leaf node content. |

## The `ext` bag

Each layout algorithm reads its own namespace from the `ext` map on a node:

```
ext["flex"]  → FlexExt   (layout-flexbox reads this)
ext["block"] → BlockExt  (layout-block reads this)
ext["grid"]  → GridExt   (layout-grid reads this)
ext["paint"] → PaintExt  (layout-to-paint reads this)
```

A node can carry data for multiple algorithms simultaneously.

## Builder helpers

```ts
import {
  container, leaf_text, leaf_image,
  size_fixed, size_fill, size_wrap,
  edges_all, edges_xy, edges_zero,
  rgb, rgba, color_transparent,
  font_spec, font_bold, font_italic,
  constraints_fixed, constraints_width,
} from "@coding-adventures/layout-ir";

// Build a simple flex row with two text nodes
const tree = container(
  [
    leaf_text({ kind: "text", value: "Left",  font: font_spec("Arial", 14),
                color: rgb(0,0,0), maxLines: null, textAlign: "start" }),
    leaf_text({ kind: "text", value: "Right", font: font_spec("Arial", 14),
                color: rgb(0,0,0), maxLines: null, textAlign: "end" }),
  ],
  {
    width: size_fill(),
    height: size_wrap(),
    padding: edges_all(8),
    ext: { flex: { direction: "row", gap: 16 } },
  }
);
```

## Where does it fit in the stack?

See [UI02 — Layout IR spec](../../specs/UI02-layout-ir.md) for the complete
design rationale and type specifications.

Packages that depend on `layout-ir`:
- `layout-flexbox` — CSS Flexbox layout algorithm
- `layout-block` — Block and inline flow layout
- `layout-grid` — CSS Grid layout algorithm
- `layout-to-paint` — Converts `PositionedNode` tree to `PaintScene`
- `layout-text-measure-estimated` — Fixed-character-width text measurer
- `layout-text-measure-canvas` — Browser Canvas text measurer
- `mosaic-ir-to-layout` — Converts Mosaic IR to `LayoutNode` tree
- `document-ast-to-layout` — Converts DocumentAST to `LayoutNode` tree
