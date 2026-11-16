# Changelog

All notable changes to `paint-palette-blend` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `P2D10: paint-palette-blend`.
- Hardware blend modes `Normal`, `Add`, `Multiply`, `Screen`, `Alpha` via `blend_pixel`, computed in exact integer arithmetic.
- Zero-allocation indexed-color blits: `blit_indexed_scanline`, `blit_indexed_image` (clipping, flips, strides, color-key transparency) and `blit_indexed_to_pixel_container`.
- RGBA span compositing: `blend_solid_span`, `blend_rgba_scanline`.
- Input converters `blend_mode_from_code`, `blend_mode_from_paint`, `alpha_from_unit_f32`.
- Zero-allocation retained-mode scene diffing by stable `PaintBase.id`: `diff_paint_scene_into` producing `PatchOp` / `PatchSummary` records for `PaintVM.patch()`.
- Security controls: no partial writes on error (CWE-787), dimension/stride/palette/alpha validation (CWE-20/125/369), checked arithmetic (CWE-190), scene and dimension caps (CWE-400), `#![forbid(unsafe_code)]`, zero ambient capabilities.
- Conformance suite (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), and runtime capability gate (`tests/runtime_capability.rs`).
- Formal verification artifacts: `paint-palette-blend.warrant`, `assurance-profile.json`, `assurance_report.json`, `effect_certificate.json`.
