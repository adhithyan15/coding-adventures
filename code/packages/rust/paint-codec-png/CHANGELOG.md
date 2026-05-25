# Changelog — paint-codec-png

## 0.2.0 — 2026-05-25

### Added

- `PngCodec::decode()` now fully implemented — delegates to `png::decode_png_rgba` (v0.2.0)
- Supports RGB (colour type 2) and RGBA (colour type 6) PNG files
- All 5 PNG filter types handled: None, Sub, Up, Average, Paeth
- CRC-32 validation per chunk
- Round-trip tests: `decode_roundtrip_rgba`, `decode_convenience_fn_matches_trait`, `decode_roundtrip_large`

### Removed

- Stub error message "not yet implemented" from `decode_png()`
- Placeholder test `decode_valid_png_returns_err_until_inflate_implemented`

### Changed

- VERSION bumped to `0.2.0`
- Package description updated to reflect full encode + decode capability

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
