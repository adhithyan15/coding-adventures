# Changelog

## 0.1.0 — 2026-04-12

### Added

- Initial release
- `render(scene: &PaintScene) -> PixelContainer` — main API
- PaintRect support via GDI `FillRect`
- PaintLine support via GDI `CreatePen` + `MoveToEx`/`LineTo`
- PaintGroup support (recursive dispatch into children)
- PaintClip support via `SaveDC`/`IntersectClipRect`/`RestoreDC`
- Hex colour parser (#rgb, #rrggbb, #rrggbbaa, "transparent")
- BGRA→RGBA pixel conversion from DIBSection memory
- Top-down DIBSection for correct coordinate system (no Y-flip)
- Barcode-pattern rendering test (alternating black/white bars)
- QR-like checkerboard rendering test
