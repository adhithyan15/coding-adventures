# Changelog

All notable changes to `game-labyrinth3d` will be documented in this file.

## [0.1.0] - 2026-11-21

### Added

- Initial implementation of `GAME34: game-labyrinth3d` pure Tier 1 deterministic 2.5D/3D dungeon crawler simulation core and dual software renderers (`RenderMode::Raycast25D` via `raycast-dda` and `RenderMode::Software3D` via `raster3d-soft`).
- Bounded tile grid, locked doors, key/health/treasure pickups, patrolling/LOS-chasing entities, circle-vs-grid sliding collision, win/loss transitions, explicit `LabyrinthRng` (`rng::Pcg32`), and `game-clock` fixed-timestep integration.
- Canonical FNV-1a `frame_digest_fnv1a` and `state_digest_fnv1a` plus atomic error rollback across `step_in_place` and `render_frame_into`.
- Conformance suite (`tests/conformance.rs`), hostile-input CVE proof & 2,000-case fuzz suite (`tests/hostile_inputs.rs`), zero-allocation `CountingAllocator` TAP microbenchmarks (`tests/microbenchmarks.rs`), runtime capability conformance (`tests/runtime_capability.rs`), and formal verification artifacts (`code/specs/game-labyrinth3d.warrant`, `assurance-profile.json`, `assurance_report.json`, `effect_certificate.json`).
