# Changelog

## 0.1.0 (2026-09-07)

- Added exact checked Apple M1 state: 64 KiB memory, 32 64-bit GPR slots,
  separate SP/PC, NZCV, 32 128-bit vector registers, halt, and installation
  metadata.
- Added validated restore/load/direct access, typed atomic steps, complete
  transition traces, and transactional bounded runs.
- Added the complete Spec 07z integer, scalar-FP, vector-memory, and NEON
  decode surface with structured big-endian encoders.
- Added ten lifecycle/decode/fault tests and a reproducible 360-vector Python
  full-state differential; package line coverage is 88.37%.
