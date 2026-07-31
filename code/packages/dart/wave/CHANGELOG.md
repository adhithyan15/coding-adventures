# Changelog

## [0.1.0] - 2026-07-31

### Added

- Initial pure Dart implementation of the PHY01 simple-harmonic wave model.
- Validated amplitude and frequency construction, derived period and angular
  frequency, phase offsets, and time-domain evaluation.
- Finite-input enforcement, angular-frequency overflow rejection, period-first
  evaluation, bounded amplitude scaling, and exact zero-amplitude behavior.
- Direct local dependency on the first-principles Dart `trig` package.
- Schema-v1 empty capability metadata and portable BUILD entry points.
