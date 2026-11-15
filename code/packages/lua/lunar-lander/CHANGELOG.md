# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-15

### Added

- Pure, deterministic simulation core for Lunar Lander (GAME02).
- Fixed-point 64-bit integer kinematics with second-order Euler-Verlet integration.
- Exact Euclidean division (`div_euclid`) and bounded digit-by-digit integer square root (`isqrt`).
- Declarative 76x5 `PaintScene` telemetry HUD and ASCII character-grid rasterizer.
- Strict numeric subtype enforcement (`CWE-843` / `CWE-681`) rejecting floats, booleans, and strings.
- Full 39-case conformance suite matching `cases.json` fixture.
