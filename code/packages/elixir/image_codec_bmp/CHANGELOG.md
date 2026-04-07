# Changelog — coding_adventures_image_codec_bmp (Elixir)

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-05

### Added
- `CodingAdventures.ImageCodecBmp` module implementing `CodingAdventures.ImageCodec` behaviour
- `encode/1` / `encode_bmp/1` — produces valid 32-bit BGRA BMP with BI_BITFIELDS compression
- `decode/1` / `decode_bmp/1` — parses 24-bit and 32-bit BMP files
- Negative-height encoding for top-down scan order (no row reversal needed)
- Row padding to 4-byte alignment for 24-bit decode support
- 22+ ExUnit tests including full pixel round-trip, structural header checks, and error cases
- Literate comments explaining BGRA byte order, BMP header layout, and little-endian encoding
