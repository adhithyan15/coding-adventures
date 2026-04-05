# Changelog — paint-metal

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
