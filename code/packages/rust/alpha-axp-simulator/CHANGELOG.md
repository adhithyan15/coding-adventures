# Changelog

## [0.1.0] — 2026-08-31

### Added

- Complete Rust port of the Layer 07s DEC Alpha AXP 21064 integer surface.
- Exact 64 KiB little-endian machine with 32 64-bit registers and PC/nPC.
- Typed complete state, validated restore, deterministic origin-aware loading,
  checked direct access, atomic steps, complete traces/results, and
  transactional bounded runs.
- Public little-endian operate, memory, branch, jump, move, and halt encoders.
- Lifecycle/workload suites and a reproducible 624-vector Python full-state
  decode/fault differential.
