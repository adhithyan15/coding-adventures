# Changelog — @coding-adventures/layout-to-paint

## [0.1.0] — 2026-04-04

### Added
- Initial implementation of `layout_to_paint(nodes, options) → PaintScene`
- `colorToCss(color) → string` — converts `Color` to CSS `rgba(r,g,b,a)` string
- `LayoutToPaintOptions` — width, height, optional background color, optional devicePixelRatio
- `PaintExt` interface for `ext["paint"]` visual decoration metadata
- Text content → `PaintGlyphRun` with Unicode code points as glyph IDs
- Image content → `PaintImage` with passthrough src and fit metadata
- Background color → `PaintRect` (fill) emitted before children
- Border → `PaintRect` (stroke only, no fill)
- Corner radius on background/border rect
- Opacity < 1.0 → wraps node instructions in `PaintLayer`
- Corner radius on containers with children → wraps children in `PaintClip`
- Device pixel ratio applied to all coordinates, sizes, and font sizes
- Recursive coordinate accumulation: child absolute position = sum of all ancestor positions
- Comprehensive tests with >80% coverage
