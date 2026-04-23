# Changelog

## 0.1.0 — 2026-04-12

### Added

- Initial release
- `render(scene: &PaintScene) -> PixelContainer` — main API
- PaintRect support via Direct2D `FillRectangle`
- PaintLine support via Direct2D `DrawLine` with stroke width
- PaintGroup support (recursive dispatch into children)
- PaintClip support via `PushAxisAlignedClip`/`PopAxisAlignedClip`
- Hex colour parser (#rgb, #rrggbb, #rrggbbaa, "transparent")
- Offscreen rendering via WIC bitmap render target (no HWND needed)
- Premultiplied BGRA → straight RGBA pixel conversion
- COM initialization (CoInitializeEx) with automatic cleanup
- Barcode-pattern rendering test (alternating black/white bars)
- QR-like checkerboard rendering test
