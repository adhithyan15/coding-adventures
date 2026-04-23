# Changelog — diagram-to-paint

## 0.1.0

Initial release.

- `diagram_to_paint(diagram, options) -> PaintScene` — main entry point
- `DiagramToPaintOptions` — background colour, CoreText PS font name, title font size
- Node shape rendering: Rect → PaintRect, RoundedRect → PaintRect with corner_radius,
  Ellipse → PaintEllipse, Diamond → PaintPath (4-vertex polygon)
- Node labels via PaintGlyphRun with `coretext:` font scheme
- Edge polylines via PaintPath (stroke only, round caps and joins)
- Directed edge arrowheads via filled PaintPath triangle
- Edge labels via PaintGlyphRun
- Diagram title via PaintGlyphRun centred at top of canvas
- Edges rendered before nodes (correct z-order: edges behind nodes)
