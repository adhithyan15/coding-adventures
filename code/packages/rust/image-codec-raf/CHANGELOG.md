# Changelog — image-codec-raf

## [Unreleased]

### Changed

- **`color.rs`** — `apply_color_pipeline` now delegates to
  `image_raw_pipeline::apply_color_pipeline` (IMG07). Pre-computes `black_avg`
  (average of the four CFA-plane levels) and normalised WB multipliers before
  calling the shared crate. Removes the private `linear_to_srgb_u8` helper —
  now handled by `image_raw_pipeline::srgb_gamma`. All 19 tests unchanged.

---

## [0.1.0] — 2026-05-29

Initial release implementing the IC14 Fujifilm RAF image codec.

### Added

- `decode_raf(bytes: &[u8]) -> Result<PixelContainer, String>` — full RAF decode pipeline
- `encode_raf(pixels: &PixelContainer) -> Vec<u8>` — minimal test encoder (RGGB Bayer, neutral WB)
- `RafCodec` struct implementing `paint_instructions::ImageCodec` trait
- `VERSION` constant `"0.1.0"`
- `header.rs` — 116-byte outer header parser with magic check and bounds validation
- `cfa_header.rs` — CFA tag-block parser for tags 0x0100, 0x0110, 0x0111, 0x0130, 0x0131, 0x0141, 0x0142
- `unpack.rs` — 12-bit big-endian packer and unpacker (2 pixels per 3 bytes)
- `bayer.rs` — 2×2 bilinear Bayer demosaicing (RGGB and arbitrary patterns)
- `xtrans.rs` — 6×6 X-Trans simplified bilinear demosaicing with 5×5 averaging window
- `color.rs` — WB normalisation (G=1.0), Fujifilm X-T2 colour matrix, sRGB gamma pipeline
- `decoder.rs` — orchestrates all stages into the public `decode_raf` function
- `encoder.rs` — minimal synthetic RAF writer for round-trip testing
- 19 unit tests covering all modules (>95% coverage)

### Security

- All offsets validated: `offset + length ≤ bytes.len()` before any slice access
- Image dimensions capped at 4096×4096 (returns `Err` for larger inputs)
- CFA header iteration capped at 256 tag blocks
- All pixel-count and file-size arithmetic uses `checked_mul`/`checked_add`
- Black-level subtraction uses `saturating_sub` to prevent underflow

### Limitations (v0.1)

- A single hardcoded colour matrix (Fujifilm X-T2) is used for all camera models
- X-Trans demosaicing is simplified bilinear (not full AHD); produces colour fringing at sharp edges
- Only 12-bit packed (uncompressed) pixel data is supported; lossless-JPEG RAF is not decoded
- `encode_raf` is a test-only encoder; output will not be accepted by Fujifilm's software
