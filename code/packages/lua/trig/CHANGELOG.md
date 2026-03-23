# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- `sin(x)` -- sine via Maclaurin series with 20-term convergence
- `cos(x)` -- cosine via Maclaurin series with 20-term convergence
- `tan(x)` -- tangent defined as sin/cos
- `radians(deg)` -- degrees-to-radians conversion
- `degrees(rad)` -- radians-to-degrees conversion
- `PI` and `TWO_PI` constants
- Internal range reduction to [-pi, pi] for numerical stability
- Literate programming style with inline explanations of the mathematics
- Comprehensive busted test suite (64 tests) covering landmark values, symmetry properties, Pythagorean identity, double-angle identities, complementary-angle identities, cross-validation against math library, and range reduction stress tests
