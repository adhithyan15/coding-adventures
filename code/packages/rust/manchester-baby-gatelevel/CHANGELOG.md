# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-08-27

### Added

- A Manchester Baby gate-level CPU whose 1,062 architectural state bits are
  held in D flip-flops.
- Gate-routed one-hot instruction decoding, five-bit CI arithmetic, and 32-bit
  negation and subtraction.
- Bounded lifecycle APIs, typed errors, owned snapshots, traces, and documented
  topology metrics.
- Opcode, edge-case, self-modification, loop, and seeded differential tests
  against the Rust functional simulator.
