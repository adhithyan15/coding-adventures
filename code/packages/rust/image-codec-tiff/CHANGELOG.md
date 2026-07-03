# Changelog — image-codec-tiff

All notable changes to this crate will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This crate uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Changed

- **`color.rs`** — `apply_color_pipeline` and `apply_srgb_gamma` now delegate to
  `image-raw-pipeline` (IMG07). The implementations are identical; this change
  eliminates the duplication that existed across TIFF, RAF, and RW2 codecs.
  Public API and all 80 tests are unchanged.

---

## [0.1.0] — 2026-05-30

### Added

- **`decode_tiff`** — decodes the first full-resolution image from a TIFF byte
  stream to RGBA8 `PixelContainer` (alpha = 255).
- **`decode_tiff_with_opts`** — decode with `TiffDecodeOptions` allowing RAW
  codec wrappers to supply custom WB multipliers, colour matrix, black level,
  white level, and IFD index.
- **`encode_tiff`** — encodes a `PixelContainer` as an uncompressed, little-endian
  RGB TIFF (Compression=1, single strip).
- **`parse_ifd_chain`** — parses all IFDs from a TIFF byte stream; exposes `Ifd`
  structs with all standard tags plus a raw-byte `extra_tags` map for unknown tags.
  Used by downstream RAW codec crates (DNG, CR2, NEF, ARW, ORF).
- **`TiffCodec`** — implements `paint_instructions::ImageCodec`.
  - `mime_type()` → `"image/tiff"`
  - `encode(pixels)` → delegates to `encode_tiff`
  - `decode(bytes)` → delegates to `decode_tiff`
- **`TiffDecodeOptions`** — decode configuration struct with `Default` impl.
- **`VERSION`** — `"0.1.0"` constant.
- **IFD parser** (`ifd.rs`): supports both little-endian (II) and big-endian (MM)
  TIFF files; handles inline and offset-stored IFD entry values; extracts all
  baseline TIFF tags plus CFA tags (33421, 33422).
- **Compression support**:
  - Uncompressed (1): direct slice read
  - PackBits (32773): byte-level RLE with header-byte decoding
  - LZW (5): 12-bit code-width LZW with MSB-first bit packing and clear/EOI codes;
    supports horizontal differencing predictor (Predictor tag = 2)
- **Photometric interpretation support**: BlackIsZero (1), RGB (2), CFA/Bayer (32803)
- **Strip and tile assembly** (`strips.rs`): assembles multi-strip and tile images;
  validates all offsets before access; bounds-checks each strip/tile.
- **Bayer demosaicing** (`bayer.rs`): bilinear interpolation for any 2×2 CFA pattern
  (RGGB, GRBG, GBRG, BGGR); handles border pixels correctly by using only
  in-bounds neighbours (avoids channel contamination at image edges).
- **Colour pipeline** (`color.rs`): black-level subtraction, white-balance
  multiplication, 3×3 colour matrix, sRGB gamma curve, u8 conversion.
- **80 unit tests**: round-trips, byte-order handling, PackBits/LZW decompression,
  grayscale 16-bit, CFA/Bayer decode, multi-strip, error cases, codec trait.
- **Security constraints**: max dimensions 32768×32768, max IFD chain 256, all
  offsets bounds-checked, LZW output cap, checked arithmetic for strip sizes.

[0.1.0]: https://github.com/adhithyan15/coding-adventures/tree/feat/ic09-tiff-impl
