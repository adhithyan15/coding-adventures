# Changelog — image-codec-nef

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-05-30

### Added

- Initial implementation of the Nikon NEF RAW image codec (IC12).
- `decode_nef(bytes: &[u8]) -> Result<PixelContainer, String>`:
  - Validates TIFF header and minimum file length.
  - Reads Make tag (271) from IFD0; rejects files where Make is present but
    does not contain "NIKON" (case-insensitive). Missing Make is allowed
    (synthetic / test files).
  - Finds the CFA sub-IFD by scanning for `photometric == 32803`.
  - Reads `bits_per_sample` to choose 12-bit (white=4095) or 14-bit
    (white=16383) decode parameters.
  - Delegates to `image_codec_tiff::decode_tiff_with_opts` with Nikon D70
    colour matrix (generic), D65 white balance multipliers, and appropriate
    black/white levels.
  - Returns a descriptive error for Nikon compressed format (34713), noting
    that v0.1 does not support Huffman/DPCM decompression.
- `encode_nef(pixels: &PixelContainer) -> Vec<u8>`:
  - Encodes as standard uncompressed TIFF (wraps `image_codec_tiff::encode_tiff`).
  - Produces files that `decode_nef` can round-trip (no Make tag = allowed).
- `NefCodec` struct implementing `paint_instructions::ImageCodec`:
  - `mime_type()` returns `"image/x-nikon-nef"`.
  - `encode()` and `decode()` delegate to `encode_nef` / `decode_nef`.
- Hardcoded Nikon colour constants:
  - `NIKON_COLOR_MATRIX`: 3×3 camera-to-sRGB (Nikon D70 representative,
    from dcraw.c).
  - `NIKON_BLACK_LEVEL_12BIT = 0`, `NIKON_WHITE_LEVEL_12BIT = 4095`.
  - `NIKON_BLACK_LEVEL_14BIT = 0`, `NIKON_WHITE_LEVEL_14BIT = 16383`.
- 14 unit tests covering:
  - Version constant and MIME type.
  - Round-trip encode/decode (2×2, 4×4 gradient, via trait).
  - Error on empty and short files.
  - Wrong Make rejection (CANON → Err).
  - NIKON and NIKON CORPORATION Make acceptance.
  - Colour matrix shape and diagonal dominance.
  - 12-bit and 14-bit level constants.
  - CFA IFD discovery in multi-IFD files.

### Divergence from spec

- The spec (IC12) lists `makernote.rs`, `compressed.rs`, `uncompressed.rs`,
  `color_matrices.rs`, `color.rs` as separate files. In v0.1 these are
  collapsed into `decoder.rs` + inline constants in `lib.rs`, because
  Nikon compression (34713) is not implemented and the colour pipeline is
  delegated to `image-codec-tiff`. The spec will be updated to reflect this
  simplified structure.
- White balance uses D65 default (all multipliers = 1.0). The spec calls for
  MakerNote WB extraction, which requires RC4 decryption and is deferred to
  a future version.
- Only the generic Nikon D70 colour matrix is included. Per-model lookup
  tables are deferred.

### Dependencies

- `pixel-container 0.1.0`
- `paint-instructions 0.1.0`
- `image-codec-tiff 0.1.0`
