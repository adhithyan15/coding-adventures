# Changelog

All notable changes to `raycast-dda` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security
- Passed `grid: &GridMap` into `validate_render_inputs` to validate all non-zero wall `tile_id`s (`usize::from(tile - 1) < textures.len()`) and per-column ray directions upfront before mutating `zbuffer` or `framebuffer.data` (atomic `render_frame_into`, zero-alloc).
- Validated `self` via `Camera2D::camera_new` in `camera_rotate` and `camera_move_by` and mapped finite `pos_x + dx` / `pos_y + dy` overflow to `RaycastError::ValueOutOfBounds`.
- Added `tests/hostile_inputs.rs` (`#![forbid(unsafe_code)]`) covering `NaN`/`±Inf`/`-0.0`/subnormals/`±f64::MAX`/`> MAX_COORD`, byte-for-byte `framebuffer.data` + `zbuffer` + `SpriteScratch` preservation across all `Err` paths (including mid-wall `InvalidMaterialId`), grazing open-map rays, and a seeded 2,000-case hostile fuzz campaign.

## [0.1.0] - 2026-11-21

### Added
- Initial implementation of `GAME31: raycast-dda` deterministic zero-allocation (steady-state) 2.5D grid DDA raycaster.
- `GridMap` and `cast_dda_ray` with fisheye-free perpendicular distance projection, `EastWest` vs `NorthSouth` wall side shading, wall texture `U/V` sampling, and `width + height` DDA step cap.
- `Camera2D` with `trig` (`PHY00`) yaw/FOV construction, rotation, and `math3d` (`GAME30`) `Vec3` / `Ray3` interop.
- `PlaneSurface` perspective floor and ceiling plane casting (solid and textured).
- `Sprite` and pre-allocated `SpriteScratch` for zero-allocation depth-sorted billboard sprite rendering with Z-buffer column occlusion into caller-provided `PixelContainer` (`IMG00`).
- Security controls: `#![forbid(unsafe_code)]`, zero ambient capabilities (`CAP001`), CWE-20/125/190/369/400/787/835 guards.
- Conformance suite (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), runtime capability gate (`tests/runtime_capability.rs`).
- Formal verification artifacts: `code/specs/raycast-dda.warrant`, `assurance-profile.json`, `assurance_report.json`, `effect_certificate.json`.
