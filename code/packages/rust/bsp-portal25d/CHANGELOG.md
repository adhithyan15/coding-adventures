# Changelog

All notable changes to `bsp-portal25d` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Initial implementation of `GAME32: bsp-portal25d` deterministic zero-allocation (steady-state) 2D sector BSP builder, front-to-back portal traverser, and 1D column occlusion renderer.
- `Vec2` and `Aabb2` with `math3d` (`GAME30`) `Vec3` / `Aabb3` interop and `trig` (`PHY00`) deterministic lengths.
- `Sector`, `Linedef`, `SurfaceStyle`, `TextureRgba`, and `Map25D` supporting variable floor/ceiling heights, 1-sided walls, 2-sided portals (upper/middle/lower surfaces), and per-sector light levels (`0..=255`).
- Iterative `BspTree::bsp_build` and `BspTree::bsp_find_subsector` with strict `MAX_BSP_DEPTH` (`64`), `MAX_BSP_NODES`, and `MAX_BSP_SEGS` bounds (`CWE-400` / `CWE-674`).
- `Camera25D` with `math3d::Ray3` / `Mat4::look_at` interop and `RenderScratch` with 1D column occlusion buffer (`y_top`, `y_bottom`), iterative traversal stack, early occlusion exit, and dynamic visibility lists (`scratch_visible_sectors`, `scratch_visible_subsectors`).
- Deterministic `frame_digest_fnv1a` and `scene_digest_fnv1a` digests.
- Conformance suite (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), runtime capability gate (`tests/runtime_capability.rs`), and formal verification artifacts (`code/specs/bsp-portal25d.warrant`, `assurance-profile.json`, `assurance_report.json`, `effect_certificate.json`).
