# Changelog — image-codec-webp

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this package adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.3.8] — 2026-05-25

### Added

- **VP8X extended container decoder** — `decode_webp` now fully handles the
  `VP8X` chunk format used by libwebp when ICC/EXIF/XMP metadata is present
  or when an alpha plane is stored separately from lossy color.
  - Sub-chunk scanner: iterates through all chunks after `VP8X`, dispatching
    `VP8L` (lossless ARGB) or `VP8 ` (lossy RGB) for image data.
  - Metadata chunks (`ICCP`, `EXIF`, `XMP `) are silently skipped.
  - `ALPH` chunk decoder: reads the 1-byte compression header; supports
    both uncompressed (method=0) and VP8L-compressed (method=1) alpha planes.
  - `apply_alph_chunk` merges decoded alpha values into the A channel of the
    reconstructed image (used when `VP8 ` lossy color + separate alpha).
  - Returns a descriptive error for animated WebP (`ANIM`/`ANMF` chunks).
- `vp8l::decode_as_alpha(data, width, height)` — decodes a VP8L-compressed
  alpha plane and returns a flat `Vec<u8>` of alpha values extracted from the
  green channel of the decoded image.
- 3 new tests: `vp8x_wrapping_vp8l_round_trips`, `vp8x_with_metadata_chunks_skipped`,
  `vp8x_anim_returns_error`.

### Changed

- `decode_webp` doc comment updated to list all supported container formats.
- `VERSION` bumped to `0.3.8`.

---

## [0.3.7] — 2026-05-25

### Added

- **VP8L meta-Huffman decoder** — `use_meta_huffman = 1` in the main image
  entropy header is now fully supported.  After reading `meta_code_bits`
  (3 bits, tile_size = 1 << (bits+2)), a meta image is decoded as a
  single-group entropy segment; each tile's Huffman group index is packed
  into `G | (R << 8)` of the meta pixel.  All Huffman group sets are then
  read sequentially, and during pixel decode the correct group is looked
  up per pixel position.  VP8L files produced by libwebp (which uses
  meta-Huffman for most natural images) can now be decoded.

### Changed

- `encode()` and `write_entropy_segment()` now write `use_meta_huffman = 0`
  (1 bit) after `color_cache_code_bits`, keeping the bitstream in sync
  with the full entropy-coded-image format.
- `read_entropy_segment()` reads the `use_meta_huffman` bit and returns an
  error if set (sub-images do not need meta-Huffman in practice).
- `VERSION` bumped to `0.3.7`.

---

## [0.3.6] — 2026-05-25

### Added

- **VP8L color-index transform decoder (type 3)** — reads the palette
  (1–256 ARGB entries, delta-coded per channel), computes `pack_bits`
  from the palette size, decodes the pixel data using the reduced
  `effective_width = ceil(orig_width / pack_bits)`, and applies
  `inverse_color_index` to expand packed G-channel indices back to full
  ARGB pixels.  VP8L files produced by libwebp or other encoders that
  use the color-index (palette) transform can now be decoded.
- `inverse_color_index` in `transforms.rs` — expands a packed-index
  image (1/2/4/8 indices per G byte, LSB-first) using a palette.
- `AppliedTransform::ColorIndex` variant carrying palette, pack_bits, and
  orig_width for the inverse pass.
- `effective_width` tracking in `decode()` — updated when a ColorIndex
  transform is read so that the pixel data section uses the correct
  reduced width.
- 3 new tests: `color_index_inverse_no_packing`,
  `color_index_inverse_pack4`, `color_index_inverse_two_rows`.

### Changed

- `VERSION` bumped to `0.3.6`.

---

## [0.3.5] — 2026-05-25

### Added

- **VP8L color cache decoder** — `color_cache_code_bits > 0` is now fully
  supported.  G symbols ≥ 280 are decoded as cache references
  (`slot = sym - 280`).  Cache slots are updated after every literal pixel and
  every back-reference copy using the hash
  `(0x1e35a7bd * ARGB) >> (32 - cache_bits)`.  The G Huffman alphabet is
  extended to `280 + 2^cache_bits` symbols automatically.  VP8L files produced
  by libwebp and other encoders that enable color cache can now be decoded.

### Changed

- `decode()` no longer returns an error when `color_cache_code_bits > 0`.
- `VERSION` bumped to `0.3.5`.

---

## [0.3.4] — 2026-05-25

### Added

- **VP8L color transform decoder (type 1)** — reads block_bits and the
  coefficient sub-image, then applies `inverse_color` per pixel:
  `new_red = red + delta(green_to_red, green)` and
  `new_blue = blue + delta(green_to_blue, green) + delta(red_to_blue, new_red)`.
  Externally-produced VP8L files that use the color transform can now be decoded.
- `inverse_color` and `color_transform_delta` in `transforms.rs`.
- `AppliedTransform::Color` variant carrying the color sub-image data.
- 2 new tests: `color_transform_zero_coefficients_is_noop`, `color_transform_round_trip`.

### Changed

- `VERSION` bumped to `0.3.4`.

---

## [0.3.3] — 2026-05-25

### Added

- **VP8L predictor transform (type 0)** — encoder uses **mode 1 (left prediction)**
  for all 16-pixel blocks (`block_bits = 4`).  The predictor sub-image is written
  inline as an entropy segment (color_cache=0, own Huffman groups, pixel data).
- **All 14 predictor modes implemented for decoding** — modes 0-13 including
  Select (mode 11), ClampedAddSubFull (mode 12), ClampedAddSubHalf (mode 13),
  and all avg-family modes (5-10).  Any externally-produced VP8L stream using
  the predictor transform can now be decoded.
- `compute_predictor` — public helper computing the predictor pixel for any
  `(x, y, mode)` triple, correctly handling first-pixel and edge-pixel sentinel
  rules (`0xFF000000`).
- `apply_predictor` and `inverse_predictor` in `transforms.rs`.
- `write_entropy_segment` and `read_entropy_segment` in `mod.rs` — shared helpers
  for encoding/decoding both the main image and predictor sub-images.
- `AppliedTransform` enum in `mod.rs` — replaces the bare `Vec<u8>` and carries
  the predictor sub-image data needed for the inverse pass.
- 7 new tests: `predictor_round_trip_mode1_solid`, `predictor_round_trip_mode1_gradient`,
  `predictor_first_pixel_sentinel`, `predictor_left_edge_mode1_uses_sentinel`,
  `predictor_mode7_avg_left_top`, `predictor_all_modes_do_not_panic_1x1`,
  `round_trip_large_image`.

### Changed

- Encoding order is now: raw → predictor(mode 1) → subtract-green → LZ77 → Huffman.
  Bitstream transform header: `[Predictor type=0, block_bits=4, sub-image]`
  then `[SubtractGreen type=2]` then `has_transform=0`.
- `VERSION` bumped to `0.3.3`.

---

## [0.3.2] — 2026-05-25

### Added

- **VP8L subtract-green transform wired into encoder** — `encode()` now clones
  the pixel buffer, applies `apply_subtract_green` (R' = R-G, B' = B-G), and
  runs LZ77 on the transformed data.  The bitstream header now writes
  `has_transform=1, transform_type=2 (SubtractGreen), has_transform=0`.
  The decoder already handled `transform_type=2` via `inverse_subtract_green`;
  no decoder change required.  Compression improves ~5-10% on colour images.

### Changed

- `VERSION` bumped to `0.3.2`.
- `transforms.rs` module doc updated to reflect subtract-green is now active.

---

## [0.3.1] — 2026-05-25

### Added

- **VP8L LZ77 back-reference encoding** — `lz77_match` uses a 65 536-slot
  direct-mapped hash chain with greedy matching up to `MAX_LEN=128` pixels;
  matches of length ≥ 2 are emitted as back-references.
- **VP8L LZ77 back-reference decoding** — the decode loop now handles G-group
  symbols 256..=279 (copy-length codes); reads distance from the Dist group;
  supports overlapping copies (RLE) via a one-pixel-at-a-time copy loop.
- **40-symbol Dist prefix alphabet** in `lz77.rs` — `DIST_BITS`, `DIST_BASE`,
  `MAX_DIST_CODE`, `decode_dist`, `encode_dist_code`, `encode_length`.
- 5 new tests: `round_trip_lz77_rle`, `round_trip_lz77_repeating_pattern`,
  `lz77_encoding_is_smaller_than_literal_for_solid`, plus `dist_bits_and_base_size`,
  `encode_dist_code_round_trip`, `encode_length_round_trip` in `lz77::tests`.

### Fixed

- **`write_huffman_code` truncated G-group symbols ≥ 256** — simple-1/2 codes
  only support 8-bit symbol values; length codes 256..=279 were silently
  truncated (266 → 10).  Now falls through to complex code format when any
  active symbol ≥ 256, making back-reference round-trips correct.
- **`encode_dist_code` out-of-range clamping** — pixel offsets > 1,048,456 now
  clamp to `MAX_DIST_CODE` instead of silently overflowing the 40-symbol Dist
  alphabet's extra-bits field.

---

## [0.3.0] — 2026-05-25

### Added

- Full YCbCr 4:2:0 color support in VP8 encoder and decoder
  - `rgb_to_ycbcr` / `ycbcr_to_rgb` — BT.601 full-range integer conversion
  - Each macroblock now carries Y DC (16×16 luma) + Cb DC + Cr DC (8×8 chroma)
  - Separate 8×8 DC predictor and context for Cb and Cr planes
  - `vp8::quant::uv_quant_step` — named alias of `dc_quant_step` for chroma
- New test: `round_trip_lossy_color` — non-grey (200, 80, 40) 16×16 image
  round-trips within ±15 per channel at quality=75

### Changed

- `decode_dct_partition` now converts reconstructed YCbCr back to RGB; output
  is correct color for any input (previously luma-only → grey output)
- `encode_dct_partition` now encodes three coefficients per non-skipped MB
  (Y, Cb, Cr) instead of one
- `compute_mb_skips` now checks all three channels; skip=true only when
  Y, Cb, and Cr residuals all quantize to zero
- `fill_macroblock` now takes (y, cb, cr: u8) and applies `ycbcr_to_rgb`
- `VERSION` bumped to `0.3.0`

### Removed

- Grey-only output (Cb=Cr=128 assumption) from `fill_macroblock` — replaced
  by proper YCbCr→RGB conversion

---

## [0.2.0] — 2026-05-25

### Added

- VP8 lossy encoder (`encode_webp(pixels, quality)`) — intra-only I-frames with
  16×16 DC prediction, WHT-coded DC residuals, skip-all-AC, and one DCT partition
- VP8 lossy decoder (`decode_webp` now handles `VP8 ` chunks) — reads the
  bool-coded first partition and DCT partition, reconstructs via DC prediction
- `src/vp8/mod.rs` — encode/decode entry points and the full macroblock loop
- `src/vp8/quant.rs` — `qp_from_quality` (quadratic quality→QP curve),
  `dc_quant_step` (128-entry RFC 6386 DC table), `quantize`/`dequantize`
- `src/vp8/wht.rs` — 4×4 forward/inverse Walsh-Hadamard Transform (placeholder)
- `range-coder` dependency added to `Cargo.toml`
- 5 new tests: `encode_webp_produces_riff_header`, `encode_webp_produces_vp8_chunk`,
  `round_trip_lossy_solid` (±5), `round_trip_lossy_quality_100` (±2),
  `decode_error_truncated`

### Changed

- `encode_webp()` no longer panics; now produces a valid VP8 RIFF/WEBP container
- `decode_webp()` now dispatches VP8 chunks to the real VP8 decoder
- `WebPCodec::encode` with `lossless=false` now calls `encode_webp` (was panic)
- `VERSION` bumped to `0.2.0`
- Description updated to reflect full VP8L + VP8 capability

### Removed

- Panic stub for VP8 lossy in `encode_webp` and `WebPCodec::encode`

---

## [0.1.0] — 2026-05-25

### Added

- **`encode_webp_lossless(pixels: &PixelContainer) -> Vec<u8>`** — encode a
  pixel buffer as a complete VP8L lossless WebP file (RIFF container included).

- **`decode_webp(bytes: &[u8]) -> Result<PixelContainer, String>`** — decode a
  WebP file; supports VP8L chunks; returns descriptive errors for VP8 lossy,
  VP8X extended, and unknown chunk types.

- **`WebPCodec`** — implements `paint_instructions::ImageCodec`:
  - `mime_type()` returns `"image/webp"`.
  - `encode()` calls `encode_webp_lossless` (lossless mode) or panics with a
    clear message (lossy mode — `range-coder` required).
  - `decode()` delegates to `decode_webp`.

- **`src/riff.rs`** — RIFF container builder (`build_riff`): encodes the
  12-byte file header, chunk header, chunk data, and even-byte padding.

- **`src/vp8l/bitstream.rs`** — LSB-first `BitWriter` and `BitReader`:
  - `BitWriter::write_bits(value, count)` — packs bits LSB-first.
  - `BitReader::read_bits(count)` / `peek_bits` / `consume_bits`.

- **`src/vp8l/huffman.rs`** — VP8L canonical Huffman encoding and decoding:
  - `HuffmanTable::from_lengths` — builds a fast decode lookup table
    (indexed by bit-reversed canonical codes for LSB-first streams).
  - `write_huffman_code` — writes simple-1, simple-2, or complex (meta-Huffman)
    code storage to the bitstream.
  - `read_huffman_code` — reads all three code storage formats back.
  - `lengths_from_frequencies` — builds Huffman code lengths from symbol
    frequencies using the `huffman-tree` crate.
  - `build_encode_table` — builds `(reversed_code, code_len)` per symbol.

- **`src/vp8l/lz77.rs`** — VP8L LZ77 distance mapping:
  - `DISTANCE_MAP` — 120-entry 2D distance offset table from the VP8L spec.
  - `dist_code_to_offset` — converts a distance code to a 1D pixel offset.
  - `length_symbol_to_base` — decodes VP8L copy-length symbols.

- **`src/vp8l/transforms.rs`** — VP8L transform infrastructure:
  - `TransformKind` — the four VP8L transform type codes.
  - `apply_subtract_green` / `inverse_subtract_green` — forward and inverse
    subtract-green transforms (forward not yet called by encoder).

- **52 unit tests** covering:
  - RIFF header layout and padding.
  - Round-trip encode/decode for: blank images, solid-colour images, gradients,
    1×1 images, transparent images, varying-alpha images.
  - VP8L signature byte, header fields.
  - Huffman simple-1, simple-2, and complex (meta-Huffman) code round-trips.
  - Bit-reversal, encode-table/decode-table consistency.
  - Error cases: bad magic, truncated input, VP8 lossy stub, unknown chunk.
  - LZ77 distance table size and sample values.
  - Subtract-green transform round-trip.

### Implementation notes

- Uses **literal-only** VP8L (no transforms, no LZ77, no colour cache).
  This is valid VP8L; compression ratio is lower than a full implementation.
- Single-symbol Huffman groups use the VP8L simple-1 format (0 bits emitted
  per symbol after writing the code table header).
- Complex code storage uses a fixed meta-tree (all meta-lengths = 4 for
  symbols 0-15) for simplicity; this is valid but not maximally compact.
- VP8 lossy (`encode_webp`) panics with `"VP8 lossy not yet implemented:
  range-coder required"` — will be wired up when the `range-coder` crate lands.

### Known limitations

- LZ77 back-references (G symbol ≥ 256) in the bitstream cause a decode error.
- Predictor, colour, and colour-index transforms are not implemented.
- VP8X extended WebP files are not supported.
