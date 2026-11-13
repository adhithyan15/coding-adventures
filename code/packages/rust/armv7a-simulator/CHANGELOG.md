# Changelog

## Unreleased

### Fixed

- Rust 1.100 clippy (`-D warnings`) drift in `tests/python_differential.rs`: `needless_range_loop`: seed the register file with `iter_mut().enumerate().take(15)`; behaviour unchanged.

## 0.1.0

- Add the complete Spec 07x ARMv7-A / Thumb-2 functional simulator.
- Add exact 64 KiB state, checked restore/load/direct access, typed atomic
  faults, complete transition traces, and transactional bounded execution.
- Cover the documented mixed-width Thumb-2 integer, memory, stack, and branch
  surface with structured encoders and Python full-state differentials.
