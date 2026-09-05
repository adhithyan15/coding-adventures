# Changelog — mosaic-emit-paint

## Unreleased

### Added
- Typed mosmodel/moslayout/mosstyle rendering entry points for PaintScene and
  PNG fixtures.
- UI49 `one-of` slot-state selection from explicit fixture values, including
  `.mil` slot-order precedence and deterministic first-member samples.
- Paint projection for base and active-state background, text color, border,
  corner-radius, and font-size properties.

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
