# Changelog

## 0.1.0 (2026-09-07)

- Added exact 64 KiB AArch64 architectural state with 32 64-bit GPR slots,
  enforced XZR, separate SP and PC, NZCV, halt, and installed-range metadata.
- Added validated restore, origin-aware load, checked direct access, aligned
  big-endian fetch/data operations, and strict reserved-field decoding.
- Added transition-atomic steps, transactional bounded runs, typed faults, and
  complete raw-instruction and before/after-state traces.
- Added the complete Spec 07v/Python integer decode surface plus structured
  big-endian teaching-transport encoders.
- Added fourteen lifecycle/decode/fault tests and a reproducible 836-vector
  Python full-state differential corpus; package line coverage is 97.20%.
