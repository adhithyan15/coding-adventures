# Changelog — image-codec-arw

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-05-30

### Added

- Initial implementation of the Sony ARW RAW image codec (IC13).
- `decode_arw(bytes: &[u8]) -> Result<PixelContainer, String>`:
  - Validates TIFF header and minimum file length (< 8 bytes → Err).
  - Reads Make tag (271) from IFD0; rejects files where Make is present but
    does not contain "SONY" (case-insensitive). Missing Make is allowed
    (synthetic / test files).
  - Finds the CFA sub-IFD by scanning for `photometric == 32803`.
    Falls back to IFD index 0 for plain TIFF round-trip files.
  - Delegates to `image_codec_tiff::decode_tiff_with_opts` with:
    - Sony A7R II colour matrix (generic from dcraw.c).
    - D65 white balance multipliers [1.0, 1.0, 1.0].
    - Black level = 200 (ARW 2.x default).
    - White level = 16383 (2^14 − 1 for 14-bit sensors).
  - Propagates TIFF decoder errors with "ARW: " prefix.
- `encode_arw(pixels: &PixelContainer) -> Vec<u8>`:
  - Encodes as standard uncompressed TIFF (wraps `image_codec_tiff::encode_tiff`).
  - Produces files that `decode_arw` can round-trip (no Make tag = allowed).
- `ArwCodec` struct implementing `paint_instructions::ImageCodec`:
  - `mime_type()` returns `"image/x-sony-arw"`.
  - `encode()` and `decode()` delegate to `encode_arw` / `decode_arw`.
- Hardcoded Sony colour constants:
  - `SONY_COLOR_MATRIX`: 3×3 camera-to-sRGB (Sony A7R II representative,
    from dcraw.c): `[[1.318, -0.398, 0.080], [-0.213, 1.586, -0.373],
    [0.047, -0.474, 1.427]]`.
  - `SONY_BLACK_LEVEL = 200` (ARW 2.x default).
  - `SONY_WHITE_LEVEL = 16383` (2^14 − 1).
- 13 unit tests covering:
  - Version constant and MIME type.
  - Round-trip encode/decode (2×2, 4×4, via trait).
  - Error on empty and short files.
  - Wrong Make rejection (NIKON → Err).
  - SONY Make acceptance and missing Make acceptance.
  - Colour matrix shape and diagonal dominance.
  - Black/white level constants.
  - CFA IFD discovery.

### Divergence from spec

- The spec (IC13) lists `makernote.rs`, `compressed.rs`, `uncompressed.rs`,
  `color_matrices.rs`, `color.rs` as separate files. In v0.1 these are
  collapsed into `decoder.rs` + inline constants in `lib.rs`, because:
  - Sony compression (32767) is not implemented in v0.1; the TIFF decoder
    handles what it can and returns Err otherwise.
  - The colour pipeline is fully delegated to `image-codec-tiff`.
  The spec will be updated to reflect this simplified v0.1 structure.
- White balance uses D65 default. MakerNote SonyMakerNote2 extraction is
  deferred to a future version.
- Only the generic Sony A7R II colour matrix is included. Per-model lookup
  tables are deferred.
- ARW 3.0 detection: rather than proactive ARW version detection, v0.1
  relies on the TIFF decoder to return Err if the compression is unsupported.

### Dependencies

- `pixel-container 0.1.0`
- `paint-instructions 0.1.0`
- `image-codec-tiff 0.1.0`
