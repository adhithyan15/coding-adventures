# Changelog — image-codec-ppm

## 0.1.0 — 2026-04-05

Initial release.

- `PpmCodec` implementing `ImageCodec` from `pixel-container`
- `encode_ppm(&PixelContainer) → Vec<u8>` convenience function
- `decode_ppm(&[u8]) → Result<PixelContainer, String>` convenience function
- P6 binary PPM format: ASCII header + raw RGB bytes
- Alpha dropped on encode; alpha = 255 on decode
- Comment handling in decoder (`#`-prefixed lines)
- 11 tests, all passing
