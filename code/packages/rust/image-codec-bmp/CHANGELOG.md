# Changelog — image-codec-bmp

## 0.1.0 — 2026-04-05

Initial release.

- `BmpCodec` implementing `ImageCodec` from `pixel-container`
- `encode_bmp(&PixelContainer) → Vec<u8>` convenience function
- `decode_bmp(&[u8]) → Result<PixelContainer, String>` convenience function
- 54-byte header (BITMAPFILEHEADER + BITMAPINFOHEADER), 32-bit BGRA pixel data
- Negative `biHeight` for top-down layout (no row reversal on encode)
- Decoder handles both top-down (negative biHeight) and bottom-up (positive biHeight)
- RGBA ↔ BGRA byte swap per pixel (R↔B channels)
- 12 tests, all passing
