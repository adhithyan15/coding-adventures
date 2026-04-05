# Changelog — paint-codec-png

## 0.1.0 — 2026-04-05

Initial release.

### Added

- `PngCodec` struct implementing the `ImageCodec` trait from `paint-instructions`
- `PngCodec::encode()` — encodes a `PixelContainer` to PNG bytes (fully implemented)
- `PngCodec::decode()` — returns `Err` until inflate support lands in the workspace `deflate` crate
- `encode_png(pixels: &PixelContainer) → Vec<u8>` — convenience function
- `decode_png(bytes: &[u8]) → Result<PixelContainer, String>` — convenience function
- `write_png(pixels: &PixelContainer, path: &str) → io::Result<()>` — write PNG to file
- `PngCodec::mime_type()` returns `"image/png"`
- Tests: magic bytes, IHDR structure, larger-image encoding, decode error paths

### Known limitations

- `decode()` is not yet implemented — inflate is needed in the `deflate` crate
