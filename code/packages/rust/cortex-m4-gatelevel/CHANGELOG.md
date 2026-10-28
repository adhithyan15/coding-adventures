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
- Execute exact inherited full-trace/full-state differentials across every
  primary family and representative privilege, IT, memory, exclusive, branch,
  power, and fault paths.
- Prove deterministic allocation failure restores complete entry state for
  both `run` and `execute` after one instruction has committed.
- Require all seven hostile complete temporary crates to compile before the
  structural validator rejects them.
- Reach 91.66% combined Rust line coverage (1,627 of 1,775 lines).
