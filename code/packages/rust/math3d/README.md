# math3d

Deterministic, zero-allocation 3D vector, matrix, quaternion, plane, AABB, ray, and frustum math library for Rust.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME30`), serving Era 3 (`raycast-dda`, `bsp-portal25d`, `raster3d-soft`) and Era 4 (`rigidbody-6dof`, `aero-flight-model`, `render3d-gpu-bridge`).

---

## Overview

1. **Deterministic Trigonometry (`PHY00` `trig`):** All `sin`, `cos`, `tan`, `atan2`, and `sqrt` calls go through `trig` for bit-identical `f64` results across platforms with no `libm` divergence.
2. **Column-Major Matrices (`Mat3`, `Mat4`):** OpenGL / Vulkan / WGPU compatible `cols[col][row]` layout, including `mat4_from_trs`, `perspective` (`[-1, 1]` NDC), `perspective_zo` (`[0, 1]` NDC), `orthographic`, `look_at`, `invert`, `mul`, and `mat4_to_f32_cols` / `mat4_to_f32_flat` for GPU uniform upload.
3. **Quaternions (`Quat`):** Axis-angle, Euler (`from_euler_xyz`, `from_euler_yxz`), Hamilton product, `slerp` (shortest-arc with safe `nlerp` fallback near `dot -> 1`), `nlerp`, and `rotate_vec3`.
4. **Frustum Culling & Intersections (`Plane`, `Aabb3`, `Sphere`, `Frustum`, `Ray3`):** Gribb-Hartmann frustum plane extraction (`from_view_proj`, `from_view_proj_zo`), p-vertex AABB frustum culling (`intersects_aabb`), sphere culling (`intersects_sphere`), `ray_plane_intersect`, and Kay-Kajiya slab `ray_aabb_intersect`.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`.
- **Zero Allocation Hot Path (`VER07`):** Fixed-size `Copy` stack values; verified with `CountingAllocator` in `tests/microbenchmarks.rs`.
- **Strict Validation:** Non-finite `NaN`/`Inf` rejection (CWE-20), bounded coordinate/angle guards (`MAX_COORD = 1e12`, `MAX_ANGLE_RAD = 2^29`, CWE-190/400), near-zero divisor and singular matrix rejection (`EPSILON = 1e-12`, CWE-369), and strict projection/bounds validation (`near > 0`, `far > near`, `fov_y in (0, pi)`, `aspect > 0`, `min <= max`).
- **26-CWE Weakness Closure (`VER15`):** `assurance_report.json` records the closure/exclusion matrix.

---

## Usage

```rust
use math3d::{Aabb3, Frustum, Mat4, Quat, Ray3, Vec3, ray_aabb_intersect};

let eye = Vec3::new(0.0, 2.0, 10.0);
let view = Mat4::look_at(eye, Vec3::zero(), Vec3::unit_y())?;
let proj = Mat4::perspective(trig::PI / 3.0, 16.0 / 9.0, 0.1, 100.0)?;
let vp = proj.mul(&view)?;
let frustum = Frustum::from_view_proj(&vp)?;

let box3 = Aabb3::aabb3_from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0))?;
assert!(frustum.intersects_aabb(&box3)?);

let rot = Quat::from_axis_angle(Vec3::unit_y(), trig::PI / 2.0)?;
let v = rot.rotate_vec3(Vec3::unit_z())?;

let ray = Ray3::ray3_from_origin_dir(Vec3::new(-5.0, 0.0, 0.0), Vec3::unit_x())?;
assert!(ray_aabb_intersect(&ray, &box3)?.is_some());
# Ok::<(), math3d::Math3dError>(())
```

---

## Testing

```
cargo test -p math3d -- --nocapture
cargo test -p math3d --release -- --nocapture
cargo clippy -p math3d --all-targets -- -D warnings
```
