# Changelog

## 0.1.0 — Initial release

- `PixelContainer(width, height, data)` — RGBA8 row-major pixel buffer.
- `PixelOps` singleton with `pixelAt`, `setPixel`, `fillPixels`.
- `ImageCodec` interface defining `mimeType`, `encode`, `decode`.
- Test suite covering default/explicit buffers, OOB policy, row-major
  offsets, channel interleave order, `fillPixels`, and the `ImageCodec`
  contract shape.
