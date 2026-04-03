# Changelog — trig (Swift)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-03

### Added

- Initial implementation of Swift trig package.
- `Trig.sin(_:)` — sine via 20-term Maclaurin series with range reduction.
- `Trig.cos(_:)` — cosine via 20-term Maclaurin series with range reduction.
- `Trig.tan(_:)` — tangent as sin/cos ratio with pole guard.
- `Trig.sqrt(_:)` — square root via Newton's (Babylonian) iterative method.
- `Trig.atan(_:)` — arctangent via Taylor series with half-angle reduction.
- `Trig.atan2(_:_:)` — four-quadrant arctangent.
- `Trig.radians(_:)` — degrees to radians conversion.
- `Trig.degrees(_:)` — radians to degrees conversion.
- `PI` constant to full Double precision.
- Comprehensive XCTest suite covering all functions.
- `Package.swift`, `BUILD`, `BUILD_windows`, `README.md`, `CHANGELOG.md`, `required_capabilities.json`.
- No external dependencies — pure Swift stdlib only.
