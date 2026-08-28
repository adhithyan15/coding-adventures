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
- RCPU-P006B1 DFF-backed decimal mode/carry and clock registers, gate-only
  single/double BCD arithmetic, exact decimal/clock fixed words, a 65-bit gate
  clock-advance network, atomic validation, and seeded functional differentials.
- Twenty-three combined tests with 89.91% core line coverage (1,257/1,398).
