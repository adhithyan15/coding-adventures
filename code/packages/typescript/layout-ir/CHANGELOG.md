# Changelog — @coding-adventures/layout-ir

## 0.1.0 — 2026-04-04

Initial release.

### Added

- `SizeValue` discriminated union: `fixed(v)`, `fill`, `wrap`
- `Edges` interface for padding and margin (top/right/bottom/left)
- `Color` interface with RGBA components (0–255)
- `FontSpec` interface: family, size, weight, italic, lineHeight
- `TextAlign` type: `"start" | "center" | "end"`
- `ImageFit` type: `"contain" | "cover" | "fill" | "none"`
- `TextContent` interface: text leaf node content
- `ImageContent` interface: image leaf node content
- `NodeContent` union type
- `LayoutNode` interface: the central IR node with open `ext` bag
- `Constraints` interface: available space for layout algorithms
- `PositionedNode` interface: layout output with resolved x/y/width/height
- `MeasureResult` interface: output of `TextMeasurer.measure()`
- `TextMeasurer` interface: injectable text measurement abstraction

Builder helpers:
- `size_fixed(v)`, `size_fill()`, `size_wrap()`
- `edges_all(v)`, `edges_xy(x, y)`, `edges_zero()`
- `rgba(r, g, b, a)`, `rgb(r, g, b)`, `color_transparent()`
- `font_spec(family, size)`, `font_bold(spec)`, `font_italic(spec)`
- `constraints_fixed(w, h)`, `constraints_width(w)`,
  `constraints_unconstrained()`, `constraints_shrink(c, dw, dh)`
- `node(opts)`, `leaf_text(content, opts?)`, `leaf_image(content, opts?)`,
  `container(children, opts?)`
- `positioned(x, y, width, height, opts)`
