# Changelog

All notable changes to `rigidbody-6dof` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Initial implementation of `GAME40: rigidbody-6dof` deterministic zero-allocation 6-DOF rigid-body dynamics integrator and ground contact solver.
- `MassProperties` with validated mass (`[MIN_MASS, MAX_MASS]`), symmetric positive-definite 3x3 inertia tensor checks (Sylvester minors + `det > MIN_DET`), and precomputed `inv_mass` / `inv_inertia_body`.
- `RigidBodyState` with world/body force and torque accumulators, moment application (`apply_force_at_world_point`, `apply_force_at_body_point`), diagnostics, and `-0.0`-canonicalized `state_digest_fnv1a`.
- `IntegratorKind::VelocityVerlet` (2nd-order symplectic with exponential quaternion update) and `IntegratorKind::RK4` (4th-order Runge-Kutta with post-step renormalization).
- Multi-point `Plane` ground contact solver (`GroundContactParams`, `ContactModel::Penalty` spring-damper + Coulomb friction, `ContactModel::Impulse` restitution + Coulomb friction) with bounded sub-stepping (`MAX_SUBSTEPS = 64`, `MAX_CONTACT_POINTS = 64`).
- Security controls: `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, zero ambient capabilities (`CAP001`), atomic state preservation on error/divergence, CWE-20/190/369/400/835 bounds.
- Conformance suite (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), and runtime capability gate (`tests/runtime_capability.rs`).
- Formal verification artifacts: `code/specs/rigidbody-6dof.warrant`, `assurance-profile.json`, `assurance_report.json`, `effect_certificate.json`.
