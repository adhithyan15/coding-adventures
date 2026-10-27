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
- Reject OE=1 and arithmetic Rc=1 encodings for every modeled XO arithmetic
  opcode as typed, whole-state-atomic unknown instructions.
- Correct `sraw` CA for the full six-bit register count while preserving every
  non-CA XER bit, and add exhaustive modifier and lifecycle regressions.
- Source-correct one legacy differential hash at the `0x80000000`, count-63
  `sraw` edge; retain the other 238 frozen Python full-state hashes.
- Pass all 13 functional-package test functions, strict
  formatting/Clippy/rustdoc, and 90.25% package-source line coverage (639/708).
