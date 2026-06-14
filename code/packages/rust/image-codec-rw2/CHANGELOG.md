# Changelog — image-codec-rw2

## [Unreleased]

### Changed

- **`color.rs`** — `apply_color_pipeline` now delegates to
  `image_raw_pipeline::apply_color_pipeline` (IMG07). RW2's signature already
  matched the shared API exactly (single `u32` black_level, `[f64;3]` WB),
  so no pre-processing is required. Removes the private `srgb_gamma` and
  `clamp01` helpers. Test calls updated to use `image_raw_pipeline::srgb_gamma`
  directly. All 38 unit tests + 2 doc-tests unchanged.

---

## [0.1.0] — 2026-05-29

### Added

- Initial implementation of Panasonic RW2 image codec (IC16).
- `decode_rw2`: full decode pipeline for 12-bit uncompressed RW2 files.
  - RW2 magic validation (`"II"` + version byte 85).
  - TIFF-like IFD parser for Panasonic private tags (sensor dimensions, borders,
    white balance, raw data offset). Entry count capped at 512 for safety.
  - 12-bit LE packed pixel unpacker (`unpack_12bit_le`): 2 pixels per 3 bytes.
  - Active-area crop via `SensorTopBorder`/`SensorLeftBorder`/…`RightBorder` tags.
  - RGGB bilinear Bayer demosaicing.
  - White balance from tags 0x0011 (RedBalance) and 0x0012 (BlueBalance).
  - Hardcoded Panasonic GH5 3×3 colour matrix (D65).
  - sRGB gamma (IEC 61966-2-1 piecewise function).
  - Output: `PixelContainer` (RGBA8, A=255).
- `encode_rw2`: minimal test encoder that produces valid RW2 bytes decodeable
  by `decode_rw2`. Not a production encoder — RW2 is a read-only format.
- `Rw2Codec` struct implementing `paint_instructions::ImageCodec`.
- `VERSION = "0.1.0"` public constant.
- Security guards: max sensor 4096×4096, checked buffer arithmetic, IFD entry
  count cap, all offset+length pairs bounds-checked before slicing.
- Graceful `Err` for unsupported variants: 16-bit depth, Panasonic lossless.
- 38 unit tests (14 in `lib.rs`, plus per-module tests in `header`, `unpack`,
  `bayer`, `color`, `encoder`, `decoder`).

### Limitations

- Panasonic lossless compression (GH5/S1/S5 v5+) is detected but not decoded.
- 16-bit ImageDepth is not supported.
- Single hardcoded colour matrix; no per-model lookup.
