# Changelog

## Unreleased

### Fixed
- Degenerate SVG endpoint arcs evaluate as their endpoint line and expose ordered bounds, preserving optional center conversion and empty cubic output.
- Adopt G2D03's strict absolute-radius and squared-endpoint guards and execute the four shared geometry fixtures.

## [0.1.0] - 2026-04-02

### Added
- `CenterArc` and `SvgArc` frozen dataclasses with W3C endpoint-to-center conversion
