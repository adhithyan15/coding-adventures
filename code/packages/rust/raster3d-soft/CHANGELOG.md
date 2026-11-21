# Changelog

All notable changes to `raster3d-soft` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Initial implementation of `GAME33: raster3d-soft` deterministic software 3D triangle rasterizer.
- Homogeneous 4D Sutherland-Hodgman frustum and near-plane (`w >= MIN_CLIP_W`) clipping on fixed stack buffers (`MAX_CLIP_VERTICES = 12`).
- Perspective divide, OpenGL `[-1, 1]` and `ZeroToOne` `[0, 1]` viewport projection, and backface/frontface culling (`CounterClockwise` / `Clockwise`).
- Half-space edge-function triangle rasterization with top-left tie-breaking rule and perspective-correct `1/w` barycentric interpolation of depth, RGB color, U/V texture coordinates, and vertex normals.
- `Flat`, `Gouraud`, `Phong`, `TexturedNearest`, and `TexturedBilinear` shading modes (`Repeat`/`Clamp` wrapping), `LightRig` ambient + directional lighting, and `f32`/`f64` Z-buffer depth testing.
- Zero-allocation steady-state `draw_triangle_into` and `draw_indexed_mesh_into` (via `RasterScratch`) rendering into caller-provided `PixelContainer` + Z-buffer slice, plus `frame_digest_fnv1a`.
- Security controls: `#![forbid(unsafe_code)]`, zero ambient capabilities (`CAP001`), CWE-20/125/190/369/400/787 guards.
- Conformance suite (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), runtime capability gate (`tests/runtime_capability.rs`), and formal verification artifacts (`code/specs/raster3d-soft.warrant`, `assurance-profile.json`, `assurance_report.json`, `effect_certificate.json`).
