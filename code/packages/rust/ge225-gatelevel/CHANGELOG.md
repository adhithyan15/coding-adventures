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
- RCPU-P006B2 DFF-backed direct card-reader/punch and N-register device state,
  exact card DMA/synchronization layouts, deterministic paper-tape/typewriter
  streams, readiness branches, alarms, and fail-closed transfer preflight.
- Ten direct-I/O conformance tests, including instruction-sequence lockstep
  against the functional GE-225 oracle and atomic modified-address rejection;
  the 33 combined tests cover 83.48% of core lines (1,339/1,604).
- RCPU-P006B3 eight DFF-backed controller banks, bounded selector command
  capture, controller status branches, ready-event API latches, exact group-32
  save/vector/return behavior, SET PST/SET PBK, and BRU target inhibition.
- Thirteen controller/API conformance tests, including functional-oracle
  lockstep and fail-closed command/skip boundaries; the 46 combined tests cover
  84.92% of core lines (1,650/1,943).
- RCPU-P006C 167 DFF-backed AAU bits for separate 40-bit AX/BX/QX/IX registers,
  mode/readiness and transient/hold alerts; exact memory, general, and status
  decode; gate-vector fixed and floating arithmetic, partial-product multiply,
  widened-remainder restoring divide, and normalization.
- Fourteen AAU conformance tests covering functional lockstep, signed and
  exponent edges, all status/hold semantics, modified IX capture, odd/even
  transfers, reset, bounded arithmetic, and full-state atomic failures. The 60
  combined tests complete the GE-225 gate-level instruction-family audit with
  85.61% core line coverage (2,190/2,558).
