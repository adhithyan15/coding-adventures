# Changelog

All notable changes to `math3d` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security
- Mapped finite row addition/subtraction overflow (`1e308`) in `plane_add_rows`/`plane_sub_rows` to `Math3dError::ValueOutOfBounds` via `check_result_finite`, and normalized hand-constructed `Frustum` planes via `Plane::plane_from_normal_d` in `Frustum::intersects_aabb`.
- Added `tests/hostile_inputs.rs` (`#![forbid(unsafe_code)]`) covering non-canonical `NaN`, `±Inf`, `-0.0`, subnormals, `±f64::MAX`, degenerate hand-constructed primitives, near-singular matrices, antipodal quaternion interpolation, and a seeded 2,000-case hostile fuzz campaign.

## [0.1.0] - 2026-11-20

### Added
- Initial implementation of `GAME30: math3d` deterministic zero-allocation 3D math library.
- `Vec3` and `Vec4` with finite/zero-length guarded dot, cross, normalize, reflect, lerp, and perspective divide.
- Column-major `Mat3` and `Mat4` with `mat4_from_trs`, `perspective`, `perspective_zo`, `orthographic`, `look_at`, `mul`, `invert`, and `f32` uniform conversion helpers.
- `Quat` with `from_axis_angle`, `from_euler_xyz`, `from_euler_yxz`, `slerp`, `nlerp`, `rotate_vec3`, conjugate, and inverse.
- `Plane`, `Aabb3`, `Sphere`, `Frustum` (Gribb-Hartmann extraction for `[-1, 1]` and `[0, 1]` depth clips, p-vertex AABB and sphere culling), and `Ray3` (`ray_plane_intersect`, Kay-Kajiya slab `ray_aabb_intersect`).
- Security controls: `#![forbid(unsafe_code)]`, zero ambient capabilities (`CAP001`), CWE-20/190/369/400 input/bounds/singularity guards.
- Conformance suite (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), runtime capability gate (`tests/runtime_capability.rs`).
- Formal verification artifacts: `code/specs/math3d.warrant`, `assurance-profile.json`, `assurance_report.json`, `effect_certificate.json`.
