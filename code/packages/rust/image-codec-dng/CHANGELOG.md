# Changelog — image-codec-dng

All notable changes to this package are documented here.
This project adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Changed

- **`color.rs`** — `invert_3x3` now delegates to
  `image_raw_pipeline::invert_3x3` (IMG07). The previous inline Cramer's rule
  implementation is replaced by a one-line wrapper so the shared crate owns the
  canonical implementation. `matrix_multiply` body is rewritten using three
  `image_raw_pipeline::mat3x3_mul` column-vector calls instead of a triple
  loop, composing the shared vector primitive. Both public APIs and all 21 unit
  tests + 2 doc-tests are unchanged.

### Dependencies

- Added `image-raw-pipeline = { path = "../image-raw-pipeline" }` (IMG07)

---

## [0.1.0] — 2026-05-30

### Added

- Initial implementation of the Adobe DNG image codec (IC10).

#### `src/lib.rs`
- `decode_dng(bytes: &[u8]) -> Result<PixelContainer, String>` — decodes a DNG
  file by extracting DNG calibration tags and delegating to `image-codec-tiff`.
- `encode_dng(pixels: &PixelContainer) -> Vec<u8>` — encodes as uncompressed
  TIFF (valid minimal DNG for round-trip tests).
- `DngCodec` struct implementing `paint_instructions::ImageCodec`:
  - `mime_type() -> "image/x-adobe-dng"`
  - `encode(&PixelContainer) -> Vec<u8>`
  - `decode(&[u8]) -> Result<PixelContainer, String>`
- `VERSION = "0.1.0"` constant.
- 21 unit tests covering: round-trips, white balance, matrix math, tag byte
  readers, error handling, and tag constant correctness.

#### `src/tags.rs`
- DNG private tag ID constants:
  `DNG_VERSION`, `UNIQUE_CAMERA_MODEL`, `BLACK_LEVEL`, `WHITE_LEVEL`,
  `COLOR_MATRIX_1`, `COLOR_MATRIX_2`, `AS_SHOT_NEUTRAL`,
  `CALIBRATION_ILLUMINANT_1`, `ACTIVE_AREA`, `FORWARD_MATRIX_1`,
  `FORWARD_MATRIX_2`.
- Inline documentation explaining each tag's TIFF type and purpose.

#### `src/color.rs`
- `wb_from_as_shot_neutral(neutrals: &[f64]) -> [f64; 3]` — converts
  AsShotNeutral triple to WB multipliers normalised so G = 1.0.
- `matrix_multiply(a, b) -> [[f64;3];3]` — 3×3 matrix multiplication.
- `camera_to_srgb_via_forward(forward) -> [[f64;3];3]` — combines ForwardMatrix
  with `XYZ_D50_TO_SRGB` to produce the camera → linear sRGB matrix.
- `invert_3x3(m) -> Option<[[f64;3];3]>` — 3×3 matrix inversion via cofactors;
  returns `None` for singular matrices (|det| < 1e-10).
- `XYZ_D50_TO_SRGB` constant — Bradford-adapted D50 → sRGB (D65) matrix.

#### `src/decoder.rs`
- `decode_dng(bytes)` — main decoder (see lib.rs description above).
- `read_srationals(val: &IfdValue) -> Vec<f64>` — reads SRATIONAL or Bytes tag.
- `read_rationals(val: &IfdValue) -> Vec<f64>` — reads RATIONAL or Bytes tag.
- `read_longs(val: &IfdValue) -> Vec<u32>` — reads LONG/SHORT/Bytes tag.
- `read_single_long(val: &IfdValue) -> Option<u32>` — first scalar from LONG/SHORT.
- `read_srationals_bytes/read_rationals_bytes/read_longs_bytes` — raw byte
  slice variants used in tests.
- IFD selection logic: chooses IFD with `NewSubfileType==0` AND photometric
  ∈ {32803 (CFA), 34892 (LinearRaw)}, falls back to IFD0.
- Colour matrix priority: ForwardMatrix1 > inv(ColorMatrix1) > identity.

#### `src/encoder.rs`
- `encode_dng(pixels: &PixelContainer) -> Vec<u8>` — delegates to
  `image_codec_tiff::encode_tiff` to produce a plain uncompressed TIFF.
  Valid as a minimal DNG since DNG is a superset of TIFF.

### Dependencies

- `pixel-container` (local path)
- `paint-instructions` (local path)
- `image-codec-tiff` (local path)

### Notes

- The encoder produces plain TIFF (no DNG private tags) — sufficient for
  round-trip testing but not a production RAW DNG.
- Matrix interpolation between two illuminants (ColorMatrix2/ForwardMatrix2)
  is not implemented in v0.1. Only illuminant 1 is used.
- ActiveArea / DefaultCrop cropping is not applied in v0.1 — the full IFD
  dimensions are decoded.
- LinearizationTable (tag 50712) is not applied in v0.1 (deferred to image-codec-tiff
  in a future version).
