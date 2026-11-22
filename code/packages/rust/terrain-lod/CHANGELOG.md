# Changelog

All notable changes to `terrain-lod` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Initial implementation of `GAME42: terrain-lod` deterministic zero-allocation heightfield and restricted quadtree LOD crate.
- Caller-owned `(2^n + 1)^2` heightfields (`Heightfield`, `HeightfieldMut`, `HeightfieldSpec`) with `1 <= level <= MAX_LEVELS (10)`.
- Deterministic terrain synthesis using `rng::Pcg32` (`synthesize_diamond_square` and `synthesize_fbm`) with pure arithmetic/smoothstep math (zero `std` `libm` calls).
- Bilinear `height_at(x, z)` and central-difference `normal_at(x, z)` / `vertex_normal(ix, iz)` queries plus `emit_vertices_into`.
- Rotated runway placement (`place_runway`) with linear shoulder blending and atomic out-of-bounds refusal.
- Restricted quadtree LOD refinement (`refine_into` reusing `TerrainScratch`), optional `math3d::Frustum` culling, crack-free center-fan stitching (`emit_mesh_into`) into caller-provided buffers with atomic `CapacityExceeded` refusal, and `mesh_digest_fnv1a` / `indices_digest_fnv1a`.
- Security & verification: `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, zero ambient capabilities (`CAP001`), CWE-20/125/190/400/787/835 guards, conformance suite (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), runtime capability conformance (`tests/runtime_capability.rs`), `code/specs/terrain-lod.warrant`, `assurance-profile.json`, `assurance_report.json`, and `effect_certificate.json`.
