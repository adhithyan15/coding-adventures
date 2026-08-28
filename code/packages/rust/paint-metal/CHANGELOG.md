# Changelog — paint-metal

## Unreleased

- Execute shared isolated layer scopes with transparent offscreen targets,
  ping-pong filter surfaces, post-filter opacity, and all shared non-normal
  blend modes.
- Add deterministic Metal pixel acceptance for child-overlap isolation,
  ordered filters, blur spread, and destination-aware blending.
- Surface native MSL compiler and command-buffer diagnostics on failure.
- Consume `paint-vm-gpu-core`'s ordered mesh/texture/clip command stream instead
  of maintaining a Metal-only scene walker and tessellator.
- Draw decoded images and gradient ramps as Metal textures in painter order,
  preserving affine transforms, nested clips, inherited opacity, scaling, and
  source-over alpha when mixed with vector content.
- Rasterize CoreText glyph runs to transient transparent textures so glyphs,
  images, and vectors remain in one command order rather than applying text as
  a final post-readback overlay.
- Add host-neutral `GpuImageResolver` entry points for URI-backed image scenes;
  fetch, cache, security, and codec policy remain owned by the caller.
- Add real-page Venture acceptance for decoded images through the Metal path.

## 0.4.0 — 2026-08-13

- Tessellate dashed `PaintPath` strokes, including dash offsets, for Metal rendering.

## 0.3.0 — 2026-08-09

- Added CSS named-color support for backend-neutral PaintInstructions.

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
