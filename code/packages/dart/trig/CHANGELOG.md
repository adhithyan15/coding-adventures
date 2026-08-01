# Changelog

## Unreleased

### Changed

- Return `atan(x)` unchanged for `|x| <= 2^-27` before half-angle reduction,
  preserving negative zero and both signs of the minimum subnormal exactly.
- Consume the versioned language-neutral PHY00 corpus in package tests,
  including poles, quadrants, signed zero, subnormal, maximum finite,
  infinity, NaN, and validation cases.
- Cover the shared PHY00 `atan` signed-zero and tiny/subnormal boundaries.

## [0.1.0] - 2026-07-31

### Added

- Initial pure Dart implementation of the PHY00 trigonometry contract.
- First-principles sine, cosine, tangent, square root, arctangent, and
  four-quadrant arctangent implementations.
- Cross-language special-angle, identity, range-reduction, conversion, and
  inverse-function tests.
- Full-range Newton scaling with tiny-normal, subnormal, infinity, NaN, and
  signed-zero boundary coverage.
- Schema-v1 empty capability metadata and portable BUILD entry points.
