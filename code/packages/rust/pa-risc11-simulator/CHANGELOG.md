# Changelog

## 0.1.0 — 2026-10-26

- Add the bounded checked PA-RISC 1.1 problem-state integer core from HP's
  primary architecture manual.
- Model hard-wired GR0, big-endian fixed words and memory, all eight PSW
  nibble carry/borrow outputs, both instruction address queues, delayed control
  flow, and direction-sensitive nullification.
- Implement basic arithmetic/logic, PA-RISC immediates, SHD, aligned byte,
  half-word and word memory, compare/add branches, BL/BLR/BV, and deterministic
  BREAK halt.
- Normalize BV's separate privilege-level bits in the flat problem-state model
  while preserving the complete computed address, and explicitly bound nested
  delayed control transfers as follow-up work.
- Add complete-state traces and transactional load, restore, step, run, and
  execute operations with typed atomic faults and bounded retention.
- Record privileged, MMU/cache/FPU/system, timing, and remaining problem-state
  families as explicit exclusions and follow-up work.
- Add three allocation-seam unit tests, 19 integration tests, one doctest,
  all-primary classification, and 92.80% all-target line coverage (580/625).
