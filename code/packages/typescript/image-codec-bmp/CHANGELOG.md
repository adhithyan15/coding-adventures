# Changelog — @coding-adventures/image-codec-bmp

## 0.1.0 — 2026-04-05

Initial release.

- `BmpCodec` class implementing `ImageCodec`
- `encodeBmp(pixels)` — 32-bit BGRA BMP, negative biHeight (top-down layout)
- `decodeBmp(bytes)` — handles both top-down (negative biHeight) and bottom-up
- RGBA ↔ BGRA byte swap per pixel (R↔B channels)
- 54-byte fixed header (BITMAPFILEHEADER + BITMAPINFOHEADER)
- Full alpha channel support via 32-bit BI_RGB
- 14 tests, all passing
