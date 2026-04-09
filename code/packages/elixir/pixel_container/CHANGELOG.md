# Changelog — coding_adventures_pixel_container (Elixir)

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-05

### Added
- `CodingAdventures.PixelContainer` struct with `width`, `height`, `data` fields
- `new/2` — creates a zeroed RGBA8 binary buffer of given dimensions
- `pixel_at/3` — reads `{r, g, b, a}` tuple; returns `{0,0,0,0}` for out-of-bounds
- `set_pixel/7` — writes RGBA values at `(x, y)`; no-op for out-of-bounds
- `fill_pixels/5` — fills entire buffer using `:binary.copy/2` for efficiency
- `byte_size/1` — returns total buffer byte count
- `CodingAdventures.ImageCodec` behaviour with `mime_type/0`, `encode/1`, `decode/1` callbacks
- 35+ ExUnit tests covering all public functions, boundary conditions, and round-trip correctness
- Knuth-style literate programming comments explaining RGBA layout, binary pattern matching, and byte arithmetic
