# Changelog — pixel-container (Go)

## 0.1.0 — 2026-04-05

Initial release.

- `PixelContainer` struct with Width, Height, and flat RGBA8 Data slice
- `ImageCodec` interface (`MimeType`, `Encode`, `Decode`)
- `New` — allocate a zeroed pixel buffer
- `PixelAt` — read one pixel, returns (0,0,0,0) for out-of-bounds
- `SetPixel` — write one pixel, no-op for out-of-bounds
- `FillPixels` — bulk fill entire buffer
- `Validate` — internal consistency check
- 22 unit tests, 100% statement coverage
