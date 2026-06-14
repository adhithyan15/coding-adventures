# Changelog — mosaic-emit-paint

## 0.1.0 — 2026-05-11

### Added
- `render_scene(source, width, height)` — Mosaic source → PaintScene
- `render_png(source, width, height)` — Mosaic source → PNG bytes  
- `render_scene_with_defaults(source)` — 400×300 canvas
- `render_png_with_defaults(source)` — 400×300 PNG
- Naive box-model layout engine: Column/Row stacking, Text baseline, Image placeholder, Divider, Icon
- SlotRef → `[slot_name]` placeholder text for static previews
- when/each → labeled placeholder boxes
- 25 unit tests
