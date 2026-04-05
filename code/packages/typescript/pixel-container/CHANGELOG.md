# Changelog — @coding-adventures/pixel-container

## 0.1.0 — 2026-04-05

Initial release.

- `PixelContainer` interface: `{ width, height, data: Uint8Array }` — fixed RGBA8 layout
- `ImageCodec` interface: `{ mimeType, encode, decode }`
- `createPixelContainer(width, height)` — factory that returns a zeroed buffer
- `pixelAt(c, x, y)` — read one pixel; returns `[0,0,0,0]` for OOB coordinates
- `setPixel(c, x, y, r, g, b, a)` — write one pixel; no-op for OOB coordinates
- `fillPixels(c, r, g, b, a)` — flood the entire buffer with one colour
- 20 tests, all passing; 100% line coverage
