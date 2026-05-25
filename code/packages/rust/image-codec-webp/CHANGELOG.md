# Changelog — image-codec-webp

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this package adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
