# Changelog

All notable changes to the `perl/lunar-lander` package will be documented in this file.

## [0.1.0] - 2026-11-15

### Added
- Pure deterministic simulation core and ASCII telemetry for Lunar Lander (GAME02).
- Pure fixed-point simulation math with exact `trunc_div`, Euclidean division `div_euclid`, and bitwise integer square root `isqrt_u128`.
- Complete 26-CWE weakness closure (CWE-190, CWE-369, CWE-843, CWE-681).
- Declarative `PaintScene` generation and 5x76 character-indexed Unicode telemetry rendering.
- Full 39-vector conformance test suite against `cases.json`.
