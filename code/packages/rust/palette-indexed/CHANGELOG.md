# Changelog

All notable changes to `palette-indexed` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- `code/specs/palette-indexed.warrant` was invalid Warrant (`/`); rewritten with `div`. Document only.

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `GAME20: palette-indexed`.
- Packed indexed formats `BitDepth::{Bpp2, Bpp4, Bpp8}` with MSB-first chunky packing, arbitrary row stride, `pack_row_into`, `unpack_row_into`, `get_index`, `set_index`.
- `Palette` fixed-size `Copy` palette storage with bounds-checked access and colour-0 transparency resolution.
- Quantized palette fading (`ChannelBits`, `quantize_channel`, `quantize_color`, `fade_color`, `fade_palette_into`, `crossfade_palette_into`) in pure integer arithmetic.
- `IndexedImage` / `IndexedCanvas` borrowed views and indexed-to-indexed sprite blitting.
- Blitting into `pixel-container`: `expand_row_to_rgba_into`, `blit_scanline_to_container`, `blit_indexed_to_container` with clipping, flips and colour-0 transparency.
- Security controls: CWE-20/125/190/369/400/787 guards, `MAX_DIMENSION = 16384`, `MAX_FADE_STEPS = 1024`, safe Rust only (`#![forbid(unsafe_code)]`), zero ambient capabilities (`CAP001`).
- Conformance suite (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), runtime capability gate (`tests/runtime_capability.rs`).
- Formal verification artifacts: `palette-indexed.warrant`, `assurance-profile.json`, `assurance_report.json`, `effect_certificate.json`.
