# aero-flight-model

Deterministic, zero-allocation 6-DOF aerodynamic flight model, ISA 1976 standard atmosphere, and bounded airfoil polar tables for Rust.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME41`), serving Era 4 (`game-flight-sim`, `cockpit-hud-paint`) atop [`rigidbody-6dof`](../rigidbody-6dof/README.md) (`GAME40`), [`math3d`](../math3d/README.md) (`GAME30`), and [`trig`](../trig/README.md) (`PHY00`).

---

## Overview

1. **ISA 1976 Standard Atmosphere (`isa_atmosphere`):** Layered atmosphere covering `[-500, 20_000] m` (troposphere `-6.5 K/km` up to `11 km` and isothermal lower stratosphere `11–20 km` at `216.65 K`), outputting temperature `T`, pressure `p`, density `rho`, and speed of sound `a = sqrt(gamma * R * T)`. Implemented with deterministic cross-platform `det_exp` / `det_ln` / `det_pow` (zero `std` `libm` calls).
2. **Air Data (`compute_air_data`):** Computes body-frame airspeed `(u, v, w)` in the right-handed Forward-Up-Right `(X, Y, Z)` frame, true airspeed `V`, angle of attack `alpha = atan2(w, u)`, clamped sideslip `beta = asin(clamp(v / V, -1, 1))`, dynamic pressure `q_bar = 0.5 * rho * V^2`, and Mach number `M = V / a`. Below `MIN_AIRSPEED` (`0.25 m/s`), aerodynamic angles and forces safely zero to prevent division by zero.
3. **Bounded Coefficient Polars (`AeroTable`, `AircraftModel`):** Stack-allocated piecewise-linear tables (`<= MAX_TABLE_POINTS = 64`) with strictly increasing finite breakpoints and clamped boundary lookup (`CWE-125` / `CWE-400`). Models `C_L(alpha)` with stall, `C_D = C_D0 + k * C_L^2 + C_D_alpha(alpha)`, `C_m(alpha)`, lateral derivatives (`C_Y(beta)`, `C_l(beta)`, `C_n(beta)`), angular rate damping (`C_mq`, `C_lp`, `C_nr`), symmetrically clamped control surfaces (elevator, aileron, rudder), and throttle thrust.
4. **GAME40 6-DOF Force Closure & Trim Solver:** `compute_world_wrench` and `step_aircraft_in_place` plug directly into `rigidbody-6dof`'s `step_with_force_fn_in_place` with zero heap allocations and atomic rollback on error. `aircraft_solve_level_trim` solves steady level-flight trim via bounded bisection.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]`.
- **Zero Allocation Hot Path (`VER07`):** Fixed-size `Copy` stack types; all runtime evaluation and stepping paths verified with `CountingAllocator` (`max_alloc_bytes: Some(0)`) in `tests/microbenchmarks.rs`.
- **Mandatory Hostile-Input CVE Proof Suite (`tests/hostile_inputs.rs`):** Explicit proof vectors for `CWE-20`, `CWE-125`, `CWE-190`, `CWE-369`, `CWE-400`, `CWE-787`, `CWE-835`, atomic state preservation, and a 2,000-case seeded hostile fuzz campaign.
- **26-CWE Weakness Closure (`VER15`):** `assurance_report.json` records the closure/exclusion matrix bound to `assurance-profile.json`.

---

## Usage

```rust
use aero_flight_model::{AircraftModel, FlightEnvironment};
use math3d::{Quat, Vec3};
use rigidbody_6dof::{IntegratorKind, MassProperties, RigidBodyState};

let model = AircraftModel::aircraft_preset_trainer()?;
let mass = MassProperties::mass_props_from_diagonal(1000.0, 1280.0, 2660.0, 1820.0)?;
let env = FlightEnvironment::env_isa_calm();
let (alpha_trim, controls) = model.aircraft_solve_level_trim(&mass, &env, 1500.0, 55.0)?;

let mut body = RigidBodyState::body_new(
    Vec3::new(0.0, 1500.0, 0.0),
    Quat::from_axis_angle(Vec3::unit_z(), alpha_trim)?,
)?;
body.linear_momentum = Vec3::new(55_000.0, 0.0, 0.0);

model.step_aircraft_in_place(&mut body, &mass, &env, &controls, 1.0 / 60.0, IntegratorKind::RK4)?;
# Ok::<(), aero_flight_model::AeroError>(())
```

---

## Testing

```
cargo test -p aero-flight-model -- --nocapture
cargo test -p aero-flight-model --release -- --nocapture
cargo clippy -p aero-flight-model --all-targets -- -D warnings
```
