# DG03 — Diagram to Paint: LayoutedGraphDiagram → PaintScene Conversion

## Overview

`diagram-to-paint` converts a `LayoutedGraphDiagram` (DG00) into a
`PaintScene` (P2D00) that can be rendered by any paint backend.

```
LayoutedGraphDiagram  (pixel-positioned graph)
  → diagram-to-paint
  → PaintScene         (renderable paint instructions)
  → PaintVM backend    (Metal, SVG, Canvas, Direct2D …)
```

---

## 1. Conversion Algorithm

Instructions are emitted in painter's-algorithm order (back to front):

1. Title text (if present)
2. All edge lines and arrowheads
3. All node shapes (filled over the edge lines)
4. All node labels

### 1.1 Title

If `diagram.title` is set, a `PaintText` instruction is emitted centered
horizontally at `y = 28`, with `font_size = title_font_size` (default 18).

### 1.2 Edges

For each `LayoutedGraphEdge`:

1. **Line**: a `PaintPath` with `MoveTo` + `LineTo` commands connecting all
   points. Stroke colour from `edge.style.stroke`, width from `stroke_width`.
   Fill is `"none"`.

2. **Arrowhead** (directed edges only): a filled `PaintPath` triangle pointing
   in the direction of the last segment of the edge. Size = 10px. Fill and
   stroke = `edge.style.stroke`.

3. **Edge label** (if present): a `PaintText` centered at `edge.label_position`.

### 1.3 Node shapes

Shape selection by `LayoutedGraphNode.shape`:

| Shape | PaintInstruction |
|-------|-----------------|
| `rect` | `PaintRect` with `corner_radius = 0` |
| `rounded_rect` | `PaintRect` with `corner_radius = style.corner_radius` |
| `ellipse` | `PaintEllipse` centered at node centre |
| `diamond` | `PaintPath` (4-vertex filled polygon: top, right, bottom, left) |

All shapes use `fill = style.fill` and `stroke = style.stroke`.

### 1.4 Node labels

A `PaintText` centered horizontally and vertically within the node bounding
box. Vertical centering uses the approximation:

```
y_baseline = node.y + node.height / 2 + font_size * 0.35
```

---

## 2. PaintText Usage

All text — titles, node labels, edge labels — is emitted as `PaintText`
instructions (P2D00 §PaintText). The `font_ref` field uses the format:

```
"canvas:<family>@<size>:<weight>"
```

For example: `"canvas:system-ui@14:400"`. Backends that cannot parse this
format should fall back to the system UI font at `font_size` points.

`text_align` is always `"center"` for all diagram text.

---

## 3. Conversion Options

| Option | Default | Description |
|--------|---------|-------------|
| `background` | `"#ffffff"` | Scene background colour |
| `font_family` | `"system-ui"` | Font family for all text |
| `title_font_size` | 18 | Font size for the diagram title |

---

## 4. Arrowhead Geometry

Given the last edge segment from `prev` to `end`:

```
direction unit vector: u = (end - prev) / |end - prev|
perpendicular:         p = (-u.y, u.x)
arrowhead size:        s = 10
base point:            b = end - u * s
left wing:             b + p * (s * 0.6)
right wing:            b - p * (s * 0.6)
```

The arrowhead is a filled triangle: `end → left_wing → right_wing → close`.

---

## 5. Public API

### `diagram_to_paint(diagram, options?) → PaintScene`

Pure function. Returns a complete `PaintScene` ready to pass to any
PaintVM backend.

---

## 6. Implementation Notes

- Zero external dependencies beyond `diagram-ir` and `paint-instructions`.
- The conversion is a single-pass walk: title → edges → node shapes → labels.
- Edge lines are emitted before node shapes so that nodes occlude the
  segment endpoints (gives a cleaner look at connection points).
- For self-loop edges with 5 waypoints, the arrowhead direction is computed
  from the last two points only (points[3] → points[4]).
