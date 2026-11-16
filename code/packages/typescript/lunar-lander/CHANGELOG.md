# Changelog

All notable changes to the `@coding-adventures/lunar-lander` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial release of `@coding-adventures/lunar-lander`: pure, deterministic simulation core and ASCII telemetry for the classic discrete lunar landing flight simulation (GAME02).
- Zero ambient system authority: `"capabilities": []` declared in `required_capabilities.json`.
- Pure integer fixed-point kinematics and quadratic touchdown contact-time solver in pure integer arithmetic (`BigInt`).
- Full domain types and interfaces:
  - `LanderState`: immutable flight state record (`Object.freeze()`).
  - `BurnCommand`: turn burn order (`burn_rate_lbs_s`, `duration_seconds`).
  - `LandingOutcome`: terminal touchdown classification (`Perfect`, `Hard`, `Crash`, `Fatal`, `Crater`).
  - `FlightStatus`: lifecycle status (`InFlight`, `Landed(outcome)`).
  - `LanderError`: closed 4-variant error enum (`FlightAlreadyEnded`, `ExcessiveBurnRate`, `InvalidDuration`, `ArithmeticOverflow`).
  - `Result<T, E>`: pure discriminated union for success and refusal.
  - `PaintScene`, `PaintInstruction`, `PaintText`, `PaintRect`: universal 2D intermediate representation for telemetry dashboard HUD.
- Core simulation APIs:
  - `new_flight()`: initializes Apollo Lunar Module at standard initial conditions.
  - `validate_burn(state, cmd)`: validates candidate command against canonical physical invariants and state constraints.
  - `step_burn(state, cmd)`: integrates discrete flight turn with automatic propellant refund on touchdown.
  - `divEuclid(a, b)`: exact Euclidean integer division where remainder is always non-negative.
  - `isqrtU128(n)`: exact 128-bit bitwise binary integer square root $\lfloor \sqrt{n} \rfloor$.
  - `to_paint_scene(state, last_burn_rate_lbs_s)`: translates flight state into declarative 76x5 `PaintScene`.
  - `render_telemetry_ascii(state, last_burn_rate_lbs_s)`: renders telemetry dashboard into deterministic 76x5 ASCII string.
- Security hardening and verification:
  - Complete 26-CWE weakness closure (`VER15`).
  - IEEE-754 / safe integer guards against `NaN`, `Infinity`, fractional floats, unsafe numbers, strings, and booleans (`CWE-681`, `CWE-190`).
  - Normalization of negative zero (`-0`) to `0`.
  - Prototype pollution guard rejecting objects with own `__proto__` or non-plain prototypes (`CWE-1321`).
  - Division by zero prevention (`CWE-369`).
  - Full conformance suite verifying all 39 vectors in `cases.json` bit-for-bit.
