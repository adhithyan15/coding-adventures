# Changelog

All notable changes to the `lunar-lander` Go package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial release of `go/lunar-lander`: pure, deterministic discrete flight simulation core and ASCII telemetry for Lunar Lander (GAME02).
- Core data structures:
  - `LanderState`: immutable value record tracking mission time, altitude, velocity, fuel, dry mass, flight status, and terminal impact velocity.
  - `BurnCommand`: turn orders (`BurnRateLbsS`, `DurationSeconds`).
  - `FlightStatus`: `InFlight`, `Landed(LandingOutcome)`.
  - `LandingOutcome`: historical classification (`OutcomePerfect`, `OutcomeHard`, `OutcomeCrash`, `OutcomeFatal`, `OutcomeCrater`).
  - `LanderError`: closed error enumeration (`ErrFlightAlreadyEnded`, `ErrExcessiveBurnRate`, `ErrInvalidDuration`, `ErrArithmeticOverflow`).
- Core simulation APIs:
  - `NewFlight() LanderState`: initializes standard Apollo LM descent conditions (Y0 = 120,000 ft, V0 = -3,000 ft/s, F0 = 16,500 lbs).
  - `ValidateBurn(state LanderState, cmd BurnCommand) error`: enforces canonical validation precedence and invariant checks.
  - `StepBurn(state LanderState, cmd BurnCommand) (LanderState, error)`: executes fixed-point Euler-Verlet integration, ground contact time solver, and unused fuel refund.
  - `DivEuclid(a, b int64) int64`: Euclidean floor division with non-negative remainder.
  - `IsqrtU128(n Uint128) Uint128`: division-free digit-by-digit binary square root for 128-bit unsigned integers.
- Declarative ASCII Telemetry:
  - `ToPaintScene(state LanderState, lastBurnRateLbsS uint32) PaintScene`: translates flight state into a 76x5 2D paint scene.
  - `RenderTelemetryASCII(state LanderState, lastBurnRateLbsS uint32) string`: rasterizes telemetry dashboard with proportional gauge bars.
- Verification and Assurance:
  - `VER07` zero-heap-allocation benchmarks and tests (`BenchmarkStepBurn`, `TestZeroAllocations`).
  - Concurrency safety tests under `-race`.
  - Conformance test suite (`conformance_test.go`) validating all 39 cases in `cases.json` and CWE-369 / CWE-190 adversarial tests.
  - Zero ambient capability declaration in `required_capabilities.json` (`capabilities: []`).
