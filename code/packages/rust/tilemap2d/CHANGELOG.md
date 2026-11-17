# Changelog

All notable changes to `tilemap2d` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `GAME21: tilemap2d`.
- Borrowed multi-layer `TileLayer` grids of `TileCell { tile_id, flags, palette_bank, attr }` with Q8 parallax, static scroll, per-axis `WrapMode::{Wrap, Clamp, Void}`, H/V flipping, visibility, colour-0 transparency, blend mode and alpha.
- Viewport culling (`cull_layer`, `layer_scroll`, `screen_to_layer`) in checked `i64` arithmetic.
- Tile-attribute collision queries (`attr_at_point`, `aabb_attrs`, `aabb_first_hit`, `aabb_blocked`, ...) for solid / one-way / water / hazard / trigger bits.
- Lowering into `paint-batch-tiles` cells (`lower_layer_cells_into`) and `paint-instructions` scenes (`lower_layers_to_paint_scene`).
- Indexed scanline and frame rendering (`render_layer_scanline`, `render_layers_to_container`) through packed `Tileset`s, `palette-indexed` palette banks and `paint-palette-blend` blend modes.
- Security controls: CWE-20/125/190/369/400/787 guards (`MAX_MAP_DIM`, `MAX_TILE_SIZE`, `MAX_VIEWPORT`, `MAX_LAYERS`, `MAX_QUERY_TILES`, `MAX_PALETTE_BANKS`), safe Rust only (`#![forbid(unsafe_code)]`), zero ambient capabilities (`CAP001`).
- Conformance suite (`tests/conformance.rs`, including a brute-force render reference and golden digest), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), runtime capability gate (`tests/runtime_capability.rs`).
- Formal verification artifacts: `tilemap2d.warrant`, `assurance-profile.json`, `assurance_report.json`, `effect_certificate.json`.
