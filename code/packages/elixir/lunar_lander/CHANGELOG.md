# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial pure-Elixir implementation of `lunar_lander` (GAME02): deterministic simulation core and ASCII telemetry for the classic discrete Lunar Module flight simulation.
- Pure value records:
  - `CodingAdventures.LunarLander.LanderState`: immutable struct capturing elapsed mission time, altitude, velocity, usable fuel, dry mass, flight status, and terminal impact velocity.
  - `CodingAdventures.LunarLander.BurnCommand`: pilot order struct (`burn_rate_lbs_s`, `duration_seconds`) accepting structs and maps with atom or string keys.
  - `CodingAdventures.LunarLander.Error`: structured exception record for burn command refusals.
  - `CodingAdventures.LunarLander.PaintScene`, `PaintText`, `PaintRect`: declarative 2D scene structures for telemetry rendering.
- Core simulation engine API:
  - `new_flight/0`: initializes mission at standard Apollo LM conditions (altitude 120,000 ft, velocity -3,000 ft/s, 16,500 lbs propellant).
  - `div_euclid/2`: exact Euclidean division matching `div_euclid(-46070, -5310) == 9`.
  - `isqrt_u128/1`: division-free digit-by-digit bitwise binary square root.
  - `validate_burn/2` and `validate_burn!/2`: validates pilot commands in canonical sequence (lifecycle, invariants, command bounds).
  - `step_burn/2` and `step_burn!/2`: executes deterministic second-order Euler-Verlet integration with contact time touchdown solver and propellant refund.
  - `to_paint_scene/2` and `telemetry_scene/1`: generates declarative 76x5 2D scene.
  - `render_telemetry_ascii/2` and `render_telemetry/1`: rasterizes 5x76 character grid with Unicode box characters, bar gauges, and rate/outcome indicators.
  - Serialization helpers: `status_to_string/1`, `status_from_string/1`, `error_to_string/1`, `error_from_string/1`.
- Complete 26-CWE weakness closure and BEAM-specific hardening:
  - Strict `is_integer/1` guards across all inputs, rejecting floats, booleans, nil, strings, and atoms (CWE-843).
  - Explicit integer coordinate boundary enforcement (CWE-190).
  - Division by zero prevention (CWE-369).
  - Zero `String.to_atom/1` calls, eliminating BEAM atom table exhaustion (CWE-400).
  - Zero `Code.eval_*` or `:erlang.binary_to_term` calls, eliminating unsafe dynamic code evaluation (CWE-94 / CWE-502).
  - Declared `capabilities: []` with zero ambient authority (CAP001).
- 39/39 test vectors passing against `cases.json` fixture suite, full multi-turn trajectory validation, refusal error verification, and adversarial CWE test suite with >95% code coverage.
