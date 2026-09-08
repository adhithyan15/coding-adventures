# Changelog

## 0.1.0 (2026-09-07)

- Added exact 64 KiB RV64I+M architectural state with 32 64-bit GPRs, enforced
  x0, 64-bit PC, halt state, and installed origin/length metadata.
- Added checked restore, origin-aware load, register/byte access, fetch, data
  alignment/range, strict decode, single-step, and bounded-run APIs.
- Added transition-atomic steps, transactional failed runs, complete
  instruction/state traces, and distinct zero-sentinel, ECALL, and EBREAK
  trace identities.
- Added the complete Spec 07y RV64I+M integer instruction surface, including
  division-by-zero and signed-overflow behavior.
- Added structured encoders, nine lifecycle/fault and encoder suites, and a reproducible
  364-vector Python full-state differential corpus covering every decode
  family.
