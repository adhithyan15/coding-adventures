# rigidbody-6dof

Deterministic, zero-allocation 6-DOF rigid-body dynamics integrator and ground contact solver for Rust.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME40`), serving Era 4 (`aero-flight-model`, `game-flight-sim`) atop [`math3d`](../math3d/README.md) (`GAME30`) and [`trig`](../trig/README.md) (`PHY00`).

---

## Overview

1. **Mass & Inertia Validation (`MassProperties`):** Enforces positive finite `mass >= MIN_MASS`, strict 3x3 body inertia tensor symmetry (`|I_ij - I_ji| <= INERTIA_SYMMETRY_TOL`), positive-definiteness via Sylvester's leading principal minors (`det > MIN_DET`), and precomputes `inv_mass` and `inv_inertia_body`.
2. **Dual 6-DOF Integrators (`IntegratorKind`):**
   - `VelocityVerlet`: 2nd-order symplectic Velocity-Verlet with midpoint world inertia tensor evaluation and exponential quaternion integration.
   - `RK4`: 4th-order Runge-Kutta over `(position, linear_momentum, orientation, angular_momentum)` with post-step quaternion renormalization.
3. **Multi-Point Ground Contact Models (`GroundContactParams`, `ContactModel`):**
   - `ContactModel::Penalty`: unilateral spring-damper normal response with smooth regularized Coulomb friction across up to `MAX_CONTACT_POINTS` (64) body-space contact probes.
   - `ContactModel::Impulse`: restitution (`[0, 1]`) + Coulomb friction sequential impulse solver with bounded positional depenetration.
   - Bounded sub-stepping (`1..=MAX_SUBSTEPS`).
4. **Diagnostics & Canonical Digests:** `linear_velocity`, `angular_velocity_world`, `angular_velocity_body`, `world_inertia`, `world_inv_inertia`, `translational_kinetic_energy`, `rotational_kinetic_energy`, `kinetic_energy`, `world_point_velocity`, and canonical 64-bit `state_digest_fnv1a` (`-0.0` normalized to `+0.0`).

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]`.
- **Zero Allocation Hot Path (`VER07`):** Fixed-size `Copy` stack types; `step_in_place` and `step_with_contacts_in_place` verified with `CountingAllocator` (`max_alloc_bytes: Some(0)`) in `tests/microbenchmarks.rs`.
- **Atomic Error Semantics & CWE Closure:** Non-finite inputs (CWE-20), near-zero mass/determinant/quaternion/normal (CWE-369), unbounded `dt`/substeps/contacts (CWE-400/835), and numerical divergence (`> MAX_COORD`) return typed `RigidBodyError` without mutating `RigidBodyState`.
- **26-CWE Weakness Closure (`VER15`):** `assurance_report.json` records the closure/exclusion matrix bound to `assurance-profile.json`.

---

## Usage

```rust
use math3d::{Plane, Quat, Vec3};
use rigidbody_6dof::{GroundContactParams, IntegratorKind, MassProperties, RigidBodyState};

let mass = MassProperties::mass_props_from_solid_box(10.0, Vec3::new(1.0, 0.5, 1.0))?;
let mut body = RigidBodyState::body_new(Vec3::new(0.0, 2.0, 0.0), Quat::quat_identity())?;
body.apply_world_force(Vec3::new(0.0, -98.1, 0.0))?;

let ground = Plane::plane_from_point_normal(Vec3::zero(), Vec3::unit_y())?;
let contact = GroundContactParams::contact_params_penalty(ground, 15_000.0, 700.0, 0.6, 4)?;
let probes = [
    Vec3::new(-1.0, -0.5, -1.0),
    Vec3::new(1.0, -0.5, -1.0),
    Vec3::new(-1.0, -0.5, 1.0),
    Vec3::new(1.0, -0.5, 1.0),
];

body.step_with_contacts_in_place(&mass, 1.0 / 60.0, IntegratorKind::VelocityVerlet, &contact, &probes)?;
let _digest = body.state_digest_fnv1a()?;
# Ok::<(), rigidbody_6dof::RigidBodyError>(())
```

---

## Testing

```
cargo test -p rigidbody-6dof -- --nocapture
cargo test -p rigidbody-6dof --release -- --nocapture
cargo clippy -p rigidbody-6dof --all-targets -- -D warnings
```
