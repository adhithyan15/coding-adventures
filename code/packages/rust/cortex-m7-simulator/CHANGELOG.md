# Changelog

## Unreleased

- Honor the architectural APSR MSR mask groups: G-only writes GE, NZCVQ-only
  preserves GE, combined writes both, and reserved masks roll back complete
  Cortex-M7 state.
- Reach 96.66% line coverage (492 of 509 lines).

## 0.1.0 — 2026-10-25

- Add a checked Cortex-M7 functional boundary built on the complete Cortex-M3
  state and instruction surface.
- Add QADD, QSUB, SADD8, and UADD8 with exact Armv7E-M encodings, sticky Q,
  GE[3:0], IT-block behavior, and complete traces.
- Add typed transactional load, restore, step, bounded run, and execute
  lifecycle operations.
- Transport GE through APSR-family MRS/MSR operations and reject reserved DSP
  operands even on a false IT path.
- Add fallible complete-state snapshots and typed rollback for allocation
  failures after an inherited transition.
- Keep caches, TCM, branch prediction, buses, and timing at an explicit later
  SoC/gate-level composition boundary.
- Cover 323 of 332 executable source lines (97.29%) in the completion suite.
