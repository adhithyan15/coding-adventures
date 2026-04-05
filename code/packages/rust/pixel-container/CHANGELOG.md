# Changelog — pixel-container

## 0.1.0 — 2026-04-05

Initial release.

- `PixelContainer` struct: `width`, `height`, `data` (RGBA8 row-major)
- Constructor: `new(width, height)` — blank all-zero buffer
- Constructor: `from_data(width, height, data)` — from existing buffer; panics if length mismatch
- `pixel_at(x, y)` — read RGBA, returns `(0,0,0,0)` if out of bounds
- `set_pixel(x, y, r, g, b, a)` — write RGBA, no-op if out of bounds
- `fill(r, g, b, a)` — fill entire canvas with one colour
- `ImageCodec` trait: `mime_type()`, `encode(&PixelContainer) → Vec<u8>`, `decode(&[u8]) → Result<PixelContainer, String>`
- 14 tests, all passing
