# Changelog — image-codec-ico

All notable changes to this crate will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This crate uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-05-26

### Added

- **`encode_ico`** — encodes a `PixelContainer` as a single-image, 32bpp BGRA BMP DIB ICO file.
  - Full alpha channel preserved in BGRA byte (AND mask all-zero).
  - `biHeight = 2 × pixel_height` per BMP DIB convention.
  - Dimensions clamped to 255 × 255 (ICO directory byte constraint).
  - Output: 6-byte ICO header + 16-byte directory entry + BMP DIB.
  - `IMAGE_OFFSET = 22` (6 + 16).

- **`decode_ico`** — decodes an ICO or CUR file into an RGBA8 `PixelContainer`.
  - Validates magic bytes: reserved=0, type ∈ {1 (ICO), 2 (CUR)}.
  - Selects best frame by area; on tie prefers PNG > 32bpp BMP > lower bpp.
  - Dispatches PNG-embedded frames to `png::decode_png_rgba`.
  - Dispatches BMP DIB frames to `bmp_dib::decode_bmp_dib`.
  - Maximum safe dimensions: 4096 × 4096 pixels (hard-coded before allocation).

- **`bmp_dib::decode_bmp_dib`** — internal BMP DIB decoder.
  - Supports 1, 4, 8, 24, and 32 bpp.
  - Palette lookup for indexed (1/4/8 bpp) images.
  - Bottom-up row order corrected during decode.
  - AND mask (1bpp) overrides per-pixel alpha: bit=1 → fully transparent.
  - Rejects non-zero `biCompression` (compressed DIBs not supported).
  - Rejects `biSize` ≠ 40 (only `BITMAPINFOHEADER` supported, not `BITMAPV4/V5`).

- **`IcoCodec`** — implements `paint_instructions::ImageCodec`.
  - `mime_type()` → `"image/x-icon"`.
  - `encode(pixels)` → delegates to `encode_ico`.
  - `decode(bytes)` → delegates to `decode_ico`.

- **`VERSION`** — `"0.1.0"` constant.

- **34 tests**: 5 round-trips, 5 header/directory, 7 BMP DIB decoder paths,
  1 AND mask transparency, 5 error cases, 1 MIME type, 1 codec trait, 1 doc-test.

- **`Cargo.toml`** dependencies: `pixel-container`, `paint-instructions`, `png`.

- **`README.md`** with stack diagram, format overview, API, testing table, and spec link.

- **`CHANGELOG.md`** (this file).

[0.1.0]: https://github.com/adhithyan15/coding-adventures/tree/feat/ic08-ico-spec-impl
