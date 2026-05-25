# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-05-25

### Added

- `decode_png_rgba(data)` — full PNG decoder; handles RGB (color_type 2) and
  RGBA (color_type 6) color types, all 5 PNG filter types (None/Sub/Up/Average/Paeth),
  CRC-32 verification per chunk, and multi-IDAT stream reassembly.  Uses
  `deflate::zlib_decompress` (deflate v0.2.0) for decompression.  Returns
  `(width, height, rgba_bytes)` as RGBA8.
- `paeth_predictor(a, b, c)` — RFC 2083 §6.6 Paeth predictor function
  (internal, used by filter-type 4 reconstruction).
- 7 new tests: `decode_roundtrip_rgba`, `decode_roundtrip_1x1`,
  `decode_invalid_magic`, `decode_empty_input`, `decode_large_image`,
  `decode_filter_none`, `decode_bad_crc`.

### Changed

- `VERSION` bumped from `"0.1.0"` to `"0.2.0"`.
- Crate description updated to "encoder/decoder" in module docs.

## [0.1.0] - 2026-04-01

### Added

- `encode_png_rgba()` — encode RGBA pixels to PNG bytes
- `write_png_rgba()` — encode and write PNG to file
- CRC-32 implementation with compile-time lookup table
- PNG chunk writer (IHDR, IDAT, IEND)
