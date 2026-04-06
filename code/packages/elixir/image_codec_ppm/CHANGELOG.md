# Changelog — coding_adventures_image_codec_ppm (Elixir)

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-05

### Added
- `CodingAdventures.ImageCodecPpm` module implementing `CodingAdventures.ImageCodec` behaviour
- `encode/1` / `encode_ppm/1` — produces valid P6 (binary PPM) files; drops alpha
- `decode/1` / `decode_ppm/1` — parses P6 PPM; sets alpha = 255 on load
- Comment line skipping during header parsing (`#`-prefixed lines)
- Robust token-based header parser that handles varied whitespace
- 23+ ExUnit tests covering header structure, round-trips, comment parsing, and error cases
- Literate comments explaining Netpbm format history and alpha-channel limitations
