# Changelog — coding-adventures-image-codec-bmp

## 0.1.0 — 2026-04-05

Initial release.

- `BmpCodec` — `ImageCodec` implementation for BMP
- `encode_bmp(pixels)` — encode `PixelContainer` to 32-bit BGRA BMP
- `decode_bmp(data)` — decode BMP bytes to `PixelContainer`
- Handles both top-down (negative `biHeight`) and bottom-up BMP files
- Only supports 32-bit `BI_RGB` files; raises `ValueError` for other formats
- 24 tests, 100% coverage
