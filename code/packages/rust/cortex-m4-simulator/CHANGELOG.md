# Changelog

## 0.1.0 — 2026-10-25

- Add a checked Armv7E-M Cortex-M4 functional boundary over the complete
  Cortex-M3 oracle, with DSP saturation, parallel-byte and dual-halfword
  arithmetic, optional FPv4-SP state and scalar operations, complete traces,
  typed atomic faults, bounded transactional execution, 23 release tests, and
  91.93 percent line coverage (911/991).
- Make successful-run final-state allocation failure transactional, distinguish
  quiet and signaling NaNs in VCMP invalid status, and derive binary32
  inexact/underflow status from exact dyadic remainders across exponent gaps.
