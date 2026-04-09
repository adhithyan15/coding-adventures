# Changelog — coding-adventures-image-codec-ppm

## 0.1.0 — 2026-04-05

Initial release.

- `PpmCodec` — `ImageCodec` implementation for PPM P6
- `encode_ppm(pixels)` — encode `PixelContainer` to binary PPM (alpha dropped)
- `decode_ppm(data)` — decode PPM P6 bytes to `PixelContainer` (alpha set to 255)
- Comment lines (`#`) are skipped in the header
- Only maxval=255 is supported; raises `ValueError` otherwise
- 17 tests, 100% coverage
