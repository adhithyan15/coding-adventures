# Changelog

## 0.1.0 — 2026-10-25

- Add a checked Armv7E-M Cortex-M4 functional boundary over the complete
  Cortex-M3 oracle, with DSP saturation, parallel-byte and dual-halfword
  arithmetic, optional FPv4-SP state and scalar operations, complete traces,
  typed atomic faults, bounded transactional execution, 29 release tests, and
  92.62 percent line coverage (992/1071).
- Make successful-run final-state allocation failure transactional, distinguish
  quiet and signaling NaNs in VCMP invalid status, and derive binary32
  inexact/underflow status from exact dyadic remainders across exponent gaps.
- Make the inherited-instruction candidate snapshot fallible and verify its
  allocation-failure path is typed and leaves the complete machine unchanged.
- Integrate APSR.GE bits 19:16 into inherited MRS/MSR transfers and validate
  every DSP/FPU general-register constraint before conditional IT skipping.
- Permit R14/LR in every modeled DSP and VMRS/VMSR general-register position
  while continuing to reject R13/SP and R15/PC (except SMLAD's SMUAD sentinel).
