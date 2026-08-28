# Changelog

All notable changes to the GE-225 gate-level Rust package are documented here.

## [Unreleased]

### Added

- RCPU-P006A gate-backed storage, one-hot decode, core-memory X groups,
  automatic modification, single/double binary arithmetic, multiply/divide,
  compares, branches, all twelve central shift/normalize operations, `MOV`,
  lifecycle, and fail-closed preflight.
- Seventeen functional-oracle differential and safety tests with 86.11% core line
  coverage (682/792).
