# Changelog — image-codec-cr2

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.1.0] — 2026-05-30

Initial release (IC11).

### Added

#### Core API
- `decode_cr2(bytes: &[u8]) -> Result<PixelContainer, String>` — decodes a
  Canon CR2 file to RGBA8 pixels.
- `encode_cr2(pixels: &PixelContainer) -> Vec<u8>` — minimal test encoder
  that writes a valid CR2 header over a standard TIFF body.
- `Cr2Codec` struct implementing `paint_instructions::ImageCodec` with MIME
  type `"image/x-canon-cr2"`.
- `VERSION: &str = "0.1.0"` constant.

#### Colour constants
- `CANON_COLOR_MATRIX: [[f64;3];3]` — hardcoded EOS 5D-era camera-to-sRGB
  matrix (from dcraw / LibRaw).
- `CANON_BLACK_LEVEL: u32 = 2047` — 14-bit sensor black pedestal.
- `CANON_WHITE_LEVEL: u32 = 15383` — 14-bit sensor saturation point.

#### Modules
- `decoder.rs` — CR2 signature validation + delegate to `image-codec-tiff`.
- `encoder.rs` — custom TIFF writer with CR2 header (IFD0 at offset 16,
  CR2 sig at offset 8).
- `lossless_jpeg.rs` (public) — SOF3 lossless JPEG decoder including:
  - `HuffTable` — canonical Huffman table (build + lookup).
  - `BitStream` — big-endian JPEG bit-stream reader with byte-stuffing.
  - `decode_sof3` — full marker parse + DPCM decode for 1-/2-component
    lossless JPEG strips with predictor 1 and restart markers.

#### Tests (29 total)
- `lib.rs` (18 tests): version, MIME type, colour matrix shape/diagonal/bounds,
  CR2 sig present, TIFF LE marker, round-trip 1×1/2×2/4×4, codec trait round-
  trip, error on empty/short/bad-magic/bad-sig/BE-marker, SOF3 stub errors,
  black < white, white in 14-bit range.
- `lossless_jpeg.rs` (11 tests): HuffTable build/lookup/mismatch, BitStream
  MSB-first/byte-stuffing/multi-bit-read, decode_sof3 error paths.

### Architecture decisions

#### Custom encoder rather than patching `encode_tiff`

`image_codec_tiff::encode_tiff` puts IFD0 at byte offset 8 — the standard
TIFF layout. CR2 requires its "CR\x02" signature at bytes 8–11, which would
corrupt the IFD if patched into that encoder's output. The CR2 encoder writes
a custom TIFF with IFD0 at offset 16 and the CR2 signature at bytes 8–11,
cleanly separating the header area from the IFD data.

#### Delegate decoding to image-codec-tiff

CR2 is a TIFF container. Rather than duplicating the TIFF strip decompressor,
Bayer demosaicing, and colour pipeline, `decode_cr2` validates the CR2 sig,
picks IFD3, sets Canon-specific decode options, and calls
`image_codec_tiff::decode_tiff_with_opts`. This keeps the CR2 crate thin and
ensures it benefits from TIFF improvements automatically.

### Spec divergences

The IC11 spec (§7) lists additional planned source files (`makernote.rs`,
`color_matrices.rs`, `bayer.rs`, `color.rs`) that are deferred to v0.2:

- **`makernote.rs`** (Canon MakerNote IFD parser) — not implemented. MakerNote
  WB and per-model colour data require parsing Exif IFD 0x927C which embeds a
  Canon-specific sub-IFD. Deferred.
- **`color_matrices.rs`** (per-model matrix table keyed on CanonModelID) — not
  implemented. v0.1 uses a single hardcoded generic matrix.
- **`bayer.rs`** — not needed; bilinear demosaicing is provided by
  `image-codec-tiff::bayer` and invoked through `decode_tiff_with_opts`.
- **`color.rs`** — not needed; the colour pipeline (WB, matrix, gamma) is
  provided by `image-codec-tiff::color`.

These divergences are noted here and the spec has been kept as "Draft" pending
v0.2 implementation.
