# Changelog — paint-metal

## 0.2.0 — 2026-04-23

### Added

- `PaintEllipse` rendering: CPU fan tessellation with 64 triangles for fill; ring of 64 quads for stroke
- `PaintPath` rendering: fan tessellation from first point for fill (correct for all convex diagram shapes); segment-to-rectangle for stroke; de Casteljau approximation of QuadTo/CubicTo with 8 linear segments each
- `PaintText` rendering: new `text_overlay` module uses `CTLineCreateWithAttributedString` + `CTLineDraw` into a CGBitmapContext wrapping the pixel buffer — no Metal texture upload needed
  - `parse_canvas_font_ref()` — parses `"canvas:family@size:weight"` font_ref format (DG03 spec)
  - `map_family_to_ps()` — maps logical CSS family names (`system-ui`, `monospace`, `serif`) to PostScript names CoreText can resolve on all Apple platforms
  - `TextAlign::Center` support via `CTLineGetTypographicBounds` width query
- `PaintRect` stroke rendering: 4 thin edge rects (top, bottom, left, right)
- `collect_geometry()` replaces `collect_vertices()` — new signature adds `texts: &mut Vec<PaintText>` so text instructions route to the CoreText overlay instead of the GPU triangle pipeline
- `emit_filled_rect()` helper for stroke edge quads
- 12 new tests covering ellipse vertex count, diamond path vertices, text collection, blue ellipse GPU render, yellow diamond GPU render, text overlay produces non-background pixels

### Changed

- `VERSION` bumped to `0.2.0`
- `render()` now orchestrates three passes: Metal GPU → PaintText CoreText overlay → PaintGlyphRun CoreText overlay

## 0.1.0 — 2026-04-05

Initial release.

### Added

- `render(scene: &PaintScene) → PixelContainer` — main public API
- Metal pipeline: device creation → offscreen RGBA8 texture → shader compile → render → pixel readback
- MSL rect shader (`RECT_SHADER_SOURCE`): solid-colour triangle rendering with pixel→NDC conversion
- `collect_vertices()` — recursive `PaintInstruction` traversal producing flat vertex arrays
- `add_rect_vertices()` — `PaintRect` → 6 triangle vertices (two right triangles)
- `add_line_vertices()` — `PaintLine` → thin rectangle perpendicular to line direction
- Group (`PaintGroup`) and Clip (`PaintClip`) recursion
- `parse_hex_color()` — CSS hex colour string to RGBA floats (supports `#rgb`, `#rrggbb`, `#rrggbbaa`, `"transparent"`)
- `create_offscreen_texture()`, `create_rect_pipeline()`, `create_buffer()`, `read_back_pixels()` Metal helpers
- Alpha blending enabled in pipeline (src-over compositing)
- Tests: colour parser, vertex generation, empty scene, red-rect-on-white full GPU render, barcode-style grid render
- arm64-only compile guard (`compile_error!` on x86_64)

### Not yet implemented

- `PaintGlyphRun` — needs CoreText rasterise + glyph texture upload
- `PaintEllipse`, `PaintPath` — need CPU-side tessellation
- `PaintLayer` — needs offscreen texture allocation and compositing pass
- `PaintGradient` — needs MSL gradient shader
- `PaintImage` — needs texture creation from `PixelContainer` or URI fetch
