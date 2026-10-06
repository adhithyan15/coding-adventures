# Changelog

All notable changes to the Python activation-functions package will be
documented in this file.

## [0.2.0] - 2026-09-30

### Added

- ML04 parity, boundary, stability, range, and public-export tests.
- Cross-platform repository BUILD fronts with coverage and Ruff validation.
- Package documentation and complete publishing metadata.

### Changed

- Export the canonical `tanh` name while retaining `tanh_func` as a compatibility
  alias.

### Fixed

- Exercise every mandatory ML04 sigmoid, tanh, and softplus vector plus the
  symmetry, range, idempotence, derivative-identity, and non-negative-gradient
  properties.

## [0.1.0] - 2026-03-20

### Added

- Initial scalar activation-function implementation.
