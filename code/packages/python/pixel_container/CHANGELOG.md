# Changelog — coding-adventures-pixel-container

## 0.1.0 — 2026-04-05

Initial release.

- `PixelContainer` dataclass: `width`, `height`, `data: bytearray` — fixed RGBA8 layout
- `ImageCodec` ABC: `mime_type`, `encode`, `decode`
- `create_pixel_container(width, height)` — factory returning zeroed buffer
- `pixel_at(c, x, y)` — read pixel; returns `(0,0,0,0)` for OOB
- `set_pixel(c, x, y, r, g, b, a)` — write pixel; no-op for OOB
- `fill_pixels(c, r, g, b, a)` — flood fill entire buffer
- 24 tests, all passing
