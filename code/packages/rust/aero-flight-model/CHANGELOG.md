# Changelog

All notable changes to `aero-flight-model` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Initial implementation of `GAME41: aero-flight-model` deterministic zero-allocation ISA 1976 atmosphere, aerodynamic polar interpolation, and 6-DOF aerodynamic wrench evaluation.
- Deterministic cross-platform transcendental math (`aero_det_exp`, `aero_det_ln`, `aero_det_pow`, `aero_det_asin`) with zero `std` `libm` calls.
- ISA 1976 atmosphere (`isa_atmosphere`) over `[-500, 20_000] m` (troposphere `-6.5 K/km` and isothermal lower stratosphere `11–20 km` at `216.65 K`).
- Air-data calculation (`compute_air_data`) with low-airspeed (`V < MIN_AIRSPEED`) safe zeroing and clamped `asin(v / V)`.
- Bounded stack-allocated `AeroTable` (`<= MAX_TABLE_POINTS = 64`) with strictly increasing scale-relative breakpoint checks and clamped OOB lookups.
- `AircraftModel` with `aircraft_preset_trainer`, stall polar `C_L(alpha)`, parabolic drag, pitching moment `C_m(alpha)`, lateral stability (`C_Y`, `C_l`, `C_n`), damping (`C_mq`, `C_lp`, `C_nr`), clamped control surfaces, thrust, `compute_world_wrench`, `step_aircraft_in_place`, and `aircraft_solve_level_trim`.
- Conformance suite (`tests/conformance.rs`), mandatory hostile-input CVE proof suite with 2,000-case fuzz campaign (`tests/hostile_inputs.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), and runtime capability gate (`tests/runtime_capability.rs`).
- Formal verification artifacts: `code/specs/aero-flight-model.warrant`, `assurance-profile.json`, `assurance_report.json`, `effect_certificate.json`.
