# DG03 — Diagram to Paint: LayoutedGraphDiagram → PaintScene Conversion

## Overview

`diagram-to-paint` converts a `LayoutedGraphDiagram` (DG00) into a
`PaintScene` (P2D00) that can be rendered by any paint backend.

```
LayoutedGraphDiagram  (pixel-positioned graph)
  → diagram-to-paint
      ├─ node shapes    → PaintRect / PaintEllipse / PaintPath  (geometry)
      ├─ edge paths     → PaintPath                             (geometry)
      └─ all text       → PositionedNode tree
                              → layout-to-paint (UI04)
                                  → PaintGlyphRun              (real shaping)
  → PaintScene         (renderable paint instructions)
  → PaintVM backend    (Metal, SVG, Canvas, Direct2D …)
```

Text rendering is **delegated to `layout-to-paint`** via a bridge of
`PositionedNode` values. Real glyph IDs are emitted (not Unicode codepoints),
so every paint backend — including `paint-metal`'s CoreText overlay — produces
correct, readable text.

---

## 1. Conversion Algorithm

Instructions are emitted in painter's-algorithm order (back to front):

1. Canvas background rectangle
2. All edge lines and arrowheads
3. All node shapes (filled over edge lines so endpoints are hidden)
4. All text (node labels + edge labels + title) via `layout-to-paint`

### 1.1 Canvas background

A single `PaintRect` covering the full canvas (`0, 0, width, height`) with
`fill = options.background` and no stroke.

### 1.2 Edges

For each `LayoutedGraphEdge`:

1. **Line**: a `PaintPath` with `MoveTo` + `LineTo` commands connecting all
   waypoints. Stroke colour from `edge.style.stroke`, width from `stroke_width`.
   Fill is `"none"`.

2. **Arrowhead** (directed edges only): a filled `PaintPath` triangle pointing
   in the direction of the last segment of the edge. Size = 10 px.

3. **Edge label** (if present): a text `PositionedNode` (see §1.4 below)
   centered at `edge.label_position`.

#### Arrowhead geometry

Given the last edge segment from `prev` to `end`:

```
u = (end - prev) / |end - prev|          unit direction vector
p = (-u.y, u.x)                          perpendicular
s = 10                                   arrowhead size
b = end - u * s                          base point
left  = b + p * (s * 0.6)
right = b - p * (s * 0.6)
```

Triangle: `end → left → right → Close`.

### 1.3 Node shapes

Shape selection by `LayoutedGraphNode.shape`:

| Shape | PaintInstruction |
|-------|-----------------|
| `Rect` | `PaintRect` with `corner_radius = 0` |
| `RoundedRect` | `PaintRect` with `corner_radius = style.corner_radius` |
| `Ellipse` | `PaintEllipse` centered at node centre |
| `Diamond` | `PaintPath` (4-vertex filled polygon: top, right, bottom, left) |

All shapes use `fill = style.fill` and `stroke = style.stroke`.

### 1.4 Text via layout-to-paint

All text (node labels, edge labels, diagram title) is rendered by building
a tree of `PositionedNode` values (from `layout-ir`) and calling
`layout-to-paint::layout_to_paint`.

One `PositionedNode` per text item is created with:

- `x`, `y`, `width`, `height` — the text bounding box in scene coordinates
- `content = Content::Text(TextContent { ... })` — the label string, font,
  colour, and `TextAlign::Center` for centred text
- No `ext["paint"]` — layout-to-paint's box-decoration path is not used;
  node shapes are already emitted in §1.3

All text `PositionedNode`s are gathered as children of a transparent root
`PositionedNode` spanning the full canvas. A single call to `layout_to_paint`
with the TXT00 shaper/metrics/resolver trio shapes every label and returns
`PaintGlyphRun` instructions which are appended to the instruction list.

#### Text bounding boxes

| Text item | x | y | width | height |
|-----------|---|---|-------|--------|
| Node label | `node.x` | `node.y + (node.height − font_size) / 2` | `node.width` | `font_size × 1.2` |
| Edge label | `label_pos.x − 60` | `label_pos.y − font_size` | `120` | `font_size × 1.2` |
| Title | `0` | `8` | `diagram.width` | `title_font_size × 1.2` |

All text uses `TextAlign::Center` so `layout-to-paint` centres each line
within the node or available width.

---

## 2. Public API

```rust
pub struct DiagramToPaintOptions<'a, S, M, R>
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    pub background: Color,
    pub device_pixel_ratio: f64,
    pub label_font: FontSpec,
    pub title_font: FontSpec,
    pub shaper: &'a S,
    pub metrics: &'a M,
    pub resolver: &'a R,
}

pub fn diagram_to_paint<S, M, R>(
    diagram: &LayoutedGraphDiagram,
    options: &DiagramToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
```

### Default option values

| Field | Default |
|-------|---------|
| `background` | `Color { r: 255, g: 255, b: 255, a: 255 }` (white) |
| `device_pixel_ratio` | `1.0` |
| `label_font` | Helvetica, 14 pt, weight 400 |
| `title_font` | Helvetica, 18 pt, weight 700 |

---

## 3. Dependencies

| Crate | Purpose |
|-------|---------|
| `diagram-ir` | `LayoutedGraphDiagram`, `LayoutedGraphNode`, `LayoutedGraphEdge` |
| `paint-instructions` | `PaintScene`, `PaintInstruction`, `PaintRect`, `PaintEllipse`, `PaintPath` |
| `layout-ir` | `PositionedNode`, `TextContent`, `FontSpec`, `Color`, `TextAlign` |
| `layout-to-paint` | `layout_to_paint`, `LayoutToPaintOptions` |
| `text-interfaces` | `TextShaper`, `FontMetrics`, `FontResolver` trait bounds |

---

## 4. Implementation Notes

- Zero I/O, zero global state. The function is a pure transformation.
- The TXT00 shaper/metrics/resolver triple must share a font binding (same
  `Handle` associated type — Rust enforces this at compile time).
- For Ellipse/Diamond nodes, no `ext["paint"]` entry is set. The shape is
  emitted directly as `PaintEllipse`/`PaintPath`; only the label text node
  passes through `layout-to-paint`.
- For Rect/RoundedRect nodes, similarly the shape is emitted as `PaintRect`;
  only the label goes through `layout-to-paint`. This ensures correct
  painter's order: shapes before text.
- `layout-to-paint` is called **once** at the end, with a synthetic root
  `PositionedNode` whose children are all text items. The background of the
  synthetic root is transparent (`Color { a: 0 }`).
- If the shaper cannot resolve a font (e.g. Helvetica not found on the OS),
  `layout-to-paint` silently drops the text. This matches `layout-to-paint`'s
  existing contract for failed font resolution.
