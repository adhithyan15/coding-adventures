# Changelog — image-codec-webp

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this package adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
