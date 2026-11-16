# Changelog

All notable changes to `paint-batch-tiles` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Parked `code/specs/paint-batch-tiles.warrant` as `paint-batch-tiles.warrant.draft`: it uses `F64`, which Warrant does not implement yet, so it failed warrant-grammar's repo-wide check. Restore it once VER04p floats (WX-FL1) land. No code change.

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `P2D09: paint-batch-tiles`.
- Core data structures: `SpriteQuad`, `TileBatchCell`, `DeltaKind`, `SpriteDeltaOp`, `ScenePatch`, and `PaintBatchError`.
- Zero-allocation sprite batch diffing: `diff_sprite_batches_into` with `VER07` `max_alloc_bytes: Some(0)` budget.
- Retained-mode scene diffing by stable `PaintBase.id`: `diff_paint_scenes` generating `ScenePatch` records for `PaintVM.patch()`.
- Deterministic sprite batch lowering: `lower_sprites_to_paint_scene` sorted by `(z_priority, id)` emitting `PaintInstruction::Rect`.
- Tile grid batch lowering: `lower_tile_batch_to_paint_scene` with row-major ordering and tile metadata.
- Comprehensive security controls:
  - CWE-400 resource exhaustion defense (`MAX_BATCH_SPRITES = 4096`).
  - CWE-20 input validation for NaN, Infinity, negative dimensions, and duplicate IDs.
  - Zero ambient capability enforcement (`CAP001`, `"capabilities": []`).
  - Safe Rust only (`#![forbid(unsafe_code)]`).
- Conformance test suite (`tests/conformance.rs`), microbenchmark harness (`tests/microbenchmarks.rs`), and runtime capability conformance gate (`tests/runtime_capability.rs`).
- Formal verification artifacts: `paint-batch-tiles.warrant.draft` (parked: uses `F64`), `assurance-profile.json`, `assurance_report.json`, and `effect_certificate.json`.
