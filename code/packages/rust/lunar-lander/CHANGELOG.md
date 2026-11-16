# Changelog

All notable changes to the `lunar-lander` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Fixed
- `code/specs/lunar-lander.warrant` was invalid Warrant (`&&`, `=>`, untyped I64 literals, `FlightStatus.InFlight`); fixed. Document only.

## 0.1.0 — 2026-11-15

### Added
- Initial release of `lunar-lander`: pure, deterministic discrete flight simulation core and ASCII telemetry for Lunar Lander (GAME02).
- Pure Rust implementation in `src/lib.rs` under `#![forbid(unsafe_code)]`.
- Core data structures:
  - `LanderState`: immutable record tracking mission time, altitude, velocity, fuel, dry mass, flight status, and terminal impact velocity.
  - `BurnCommand`: turn orders (`burn_rate_lbs_s`, `duration_seconds`).
  - `FlightStatus`: `InFlight`, `Landed(LandingOutcome)`.
  - `LandingOutcome`: historical classification (`Perfect`, `Hard`, `Crash`, `Fatal`, `Crater`).
  - `LanderError`: closed error enum (`FlightAlreadyEnded`, `ExcessiveBurnRate`, `InvalidDuration`, `ArithmeticOverflow`).
- Core simulation APIs:
  - `new_flight() -> LanderState`: initializes standard Apollo LM descent conditions ($Y_0 = 120,000\text{ ft}, V_0 = -3,000\text{ ft/s}, F_0 = 16,500\text{ lbs}$).
  - `validate_burn(state: &LanderState, cmd: &BurnCommand) -> Result<(), LanderError>`: enforces canonical validation precedence and invariant checks.
  - `step_burn(state: &LanderState, cmd: &BurnCommand) -> Result<LanderState, LanderError>`: executes fixed-point Euler-Verlet integration, ground contact time solver, and unused fuel refund.
  - `isqrt_u128(n: u128) -> u128`: division-free digit-by-digit binary square root with bounded steps.
- Declarative ASCII Telemetry (`paint-vm-ascii` / `P2D02`):
  - `to_paint_scene(state: &LanderState, last_burn_rate_lbs_s: u32) -> PaintScene`: translates flight state into a 76x5 2D paint scene.
  - `render_telemetry_ascii(state: &LanderState, last_burn_rate_lbs_s: u32) -> String`: rasterizes telemetry dashboard with proportional gauge bars via `paint-vm-ascii`.
- Verification and Assurance:
  - `VER07` zero-heap-allocation microbenchmarks (`tests/microbenchmarks.rs`) with custom `CountingAllocator` and sabotage detection.
  - `VER08b` runtime capability confinement tests (`tests/runtime_capability.rs`).
  - Conformance test suite (`tests/conformance.rs`) executing all 39 cases in `cases.json`, plus CWE-369 and CWE-190 adversarial tests and telemetry HUD verification.
  - Zero ambient capability declaration in `required_capabilities.json` (`capabilities: []`) verified by `ca-capability-analyzer`.
  - Effect certificate (`effect_certificate.json`) proved zero side effects.
  - Assurance profile and report (`assurance_report.json`) addressing all 26 CWEs per `VER15`.
