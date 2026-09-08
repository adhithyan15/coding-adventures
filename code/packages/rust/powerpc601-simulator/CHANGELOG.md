# Changelog

## [0.1.0] — 2026-08-31

### Added

- Complete Rust port of the Layer 07u PowerPC 601 integer surface.
- Exact 64 KiB big-endian machine with 32 GPRs, LR, CTR, XER, CR, and CIA.
- Typed complete state, validated restore, origin-aware loading, checked direct
  access, atomic steps, complete traces/results, and transactional runs.
- Public I/B/D/X/XO/XFX/XL instruction encoders and big-endian assembly.
- Lifecycle/workload suites and a reproducible 239-vector Python full-state
  differential covering every decode family.
- Distinct signed/unsigned division-by-zero faults and strict alignment faults.
