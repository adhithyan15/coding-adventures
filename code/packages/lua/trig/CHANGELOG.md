# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-04-03

### Added

- `trig.sqrt(x)` — square root via Newton's method; `error()`s for negative inputs.
- `trig.atan(x)` — arctangent via Taylor series with outer and half-angle range reduction.
- `trig.atan2(y, x)` — four-quadrant arctangent.
- `trig.HALF_PI` public constant (π/2).
- `atan_core(x)` local (private) helper function.
- Tests for all new functions in `tests/test_trig.lua`.

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
