# Changelog — image-codec-jxl

All notable changes to this crate are documented here.
Versions follow [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-05-28

### Added

- **`encode_jxl(pixels: &PixelContainer) -> Vec<u8>`** — encodes RGBA images
  into a simplified JXL Modular naked codestream (`FF 0A` magic).

- **`decode_jxl(data: &[u8]) -> Result<PixelContainer, String>`** — decodes
  the same format back to a pixel buffer; also detects ISOBMFF containers and
  extracts the `jxlc` codestream box.

- **`JxlCodec`** — unit struct implementing the `pixel-container::ImageCodec`
  trait (`mime_type = "image/jxl"`).

- **`VERSION`** constant (`"0.1.0"`).

- **`src/bitwriter.rs`** — MSB-first bit packing (`BitWriter`).

- **`src/bitreader.rs`** — MSB-first bit extraction (`BitReader`) with
  `align_to_byte()` and `remaining_bytes_from_boundary()`.

- **`src/container.rs`** — container format detection: naked (`FF 0A`) and
  ISOBMFF (`JXL ` signature + `jxlc` box scan).

- **`src/modular.rs`** — gradient predictor with W/N/NW/NE edge handling,
  `compute_residuals`, `reconstruct_values`.

- **`src/entropy.rs`** — `encode_rans_block` / `decode_rans_block` (self-
  describing wire format), `encode_channel_residuals` /
  `decode_channel_residuals` (sign + magnitude two-pass split).

- **`src/encoder.rs`** — SizeHeader bit encoding (div8 / direct paths, ratio
  field), full encode pipeline.

- **`src/decoder.rs`** — SizeHeader bit decoding (all ratio values), full
  decode pipeline including dimension cross-check.

- **`src/rct.rs`** — YCoCg reversible colour transform (forward + inverse,
  RCT type 6) included for completeness; not used by the current encoder.

- **66 unit tests + 3 doc tests** covering:
  - BitWriter/BitReader correctness and edge cases
  - Container detection (naked, ISOBMFF, error paths)
  - Modular predictor round-trips and boundary conditions
  - Entropy sign/magnitude split round-trips
  - RCT lossless round-trip over the full 8bpp colour cube
  - Full end-to-end `encode_jxl` → `decode_jxl` round-trips for 1×1, 2×2,
    4×4, 8×8, 32×32, solid colour, gradient, RGBA, transparent, and mixed-
    alpha images
  - Error cases (bad magic, too short, empty input)
  - `ImageCodec` trait and VERSION format

### Architecture notes

- Residuals for 8bpp channels lie in [−255, 255].  The `rans` crate uses a
  `u8` symbol type (alphabet ≤ 256), so we split each residual into a
  3-symbol sign stream and a 255-symbol magnitude stream.  This avoids any
  multi-byte escape scheme and keeps both streams well within the alphabet
  limit.

- The SizeHeader follows the real JXL spec §4.1 (div8 path and direct path
  with 2-bit selector), but the metadata after the SizeHeader uses our own
  simplified binary layout rather than the full JXL ANS-coded metadata
  sections.  This is intentional for teaching purposes.

- ISOBMFF container detection is implemented (box scanning, `jxlc` lookup)
  so the crate can strip the wrapper from files produced by libjxl tools,
  even though decoding the non-simplified internal format would require the
  full JXL specification.
