# Changelog

## 0.1.0 — 2026-10-26

- Add a checked bounded ARMv4T ARM7TDMI functional simulator.
- Own all 31 physical GPRs, CPSR, five SPSRs, 64 KiB memory, halt, and image
  lifecycle state.
- Implement the specified ARM and complete Thumb-1 integer instruction
  surfaces, pipeline-visible PC values, bank selection, state exchange,
  transfer alignment, and deterministic `SWI 0` sentinels.
- Add complete-state traces, typed errors, full-span target/memory preflight,
  and transactional step/run/execute behavior.
- Enforce the fixed `S=1`, `Rd=0` fields on ARM test/compare encodings and make
  canonical empty-image reset snapshots restorable.
- Add the completion contract, package BUILD front, workspace registration,
  and documentation with 39 tests and **88.30% (1,389/1,573)** line coverage.
