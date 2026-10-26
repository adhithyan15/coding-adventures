# Changelog

## 0.1.0 — 2026-10-26

- Add an independent Armv7E-M Cortex-M4 architectural gate simulator over the
  accepted Cortex-M3 gate executor.
- Clock exactly 526,013 persistent bits through repository D flip-flops.
- Add gate-derived QADD/QSUB, SSAT/USAT, SADD8/UADD8, SMUAD/SMLAD, APSR.Q/GE,
  and optional scalar FPv4-SP arithmetic, compare, status, and transport.
- Add typed atomic lifecycle and allocation boundaries with bounded complete
  before/after traces.
- Add functional-oracle differentials and compiler-checked full-source hostile
  fixtures for executor, decode, datapath, and storage authority.
- Reach 90.86% combined Rust line coverage (1,590 of 1,750 lines).
