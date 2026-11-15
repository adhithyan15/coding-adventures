# Changelog

All notable changes to the Python lunar-lander package will be documented in this file.

## [0.1.0] - 2026-11-15

### Added

- Initial release of pure deterministic Lunar Lander flight simulation core (`GAME02`).
- Immutable `@dataclass(frozen=True, slots=True)` records: `LanderState`, `BurnCommand`, `PaintScene`, `PaintText`, `PaintRect`.
- Domain types and enumerations: `LandingOutcome`, `FlightStatus`, `InFlight`, `Landed`.
- Exact fixed-point numerical physics with second-order Euler-Verlet integration and ground touchdown interpolation.
- Exact Euclidean division helper `div_euclid` and bitwise 128-bit integer square root `isqrt_u128`.
- Pure declarative ASCII telemetry HUD renderer (`to_paint_scene`, `render_telemetry_ascii`, `render_telemetry`, `telemetry_scene`) with 100% parity to `rust/lunar-lander`.
- Complete 26-CWE weakness closure tests including CWE-190, CWE-369, and CWE-843.
- Full cross-language conformance against all 39 vectors in `code/specs/fixtures/lunar-lander-v1/cases.json`.
- Zero ambient capabilities declared and audited (`CAP001`, `VER08a`).
