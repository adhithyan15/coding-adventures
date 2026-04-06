# Changelog — coding_adventures_image_codec_qoi (Elixir)

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-05

### Added
- `CodingAdventures.ImageCodecQoi` module implementing `CodingAdventures.ImageCodec` behaviour
- `encode/1` / `encode_qoi/1` — full QOI encoder with all 6 op-codes:
  - `QOI_OP_RUN` — run-length up to 62 pixels
  - `QOI_OP_INDEX` — 64-slot rolling hash table lookup
  - `QOI_OP_DIFF` — 1-byte small delta (dr,dg,db each −2..+1)
  - `QOI_OP_LUMA` — 2-byte medium delta (dg −32..+31, dr/db relative to dg)
  - `QOI_OP_RGB` — 4-byte explicit RGB (alpha unchanged)
  - `QOI_OP_RGBA` — 5-byte explicit RGBA
- `decode/1` / `decode_qoi/1` — full QOI decoder with big-endian header parsing
- Hash function: `rem(r*3 + g*5 + b*7 + a*11, 64)`
- Signed delta wrap arithmetic: `rem((d &&& 0xFF) + 128, 256) - 128`
- 22+ ExUnit tests covering all op-code paths, compression verification, round-trips, and error cases
- Literate comments explaining QOI format history, op-code bit layouts, hash design, and wrapping arithmetic
