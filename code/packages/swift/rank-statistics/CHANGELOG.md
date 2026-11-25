# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-25

### Added

- Exact finite-sample median intervals using private base-2^32 limbs and bounded exhaustive/differential tests.
- Overflow-safe and subnormal-correct median midpoint behavior without mutating samples.

### Fixed

- Run the Windows Swift availability guard and tests in one batch process so the skip covers the full sequence and failures propagate.
