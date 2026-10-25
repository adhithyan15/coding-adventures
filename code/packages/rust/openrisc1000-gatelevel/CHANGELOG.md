# Changelog

## 0.1.0 — 2026-10-25

- Add an independent big-endian ORBIS32 gate transition engine.
- Store the complete 64 KiB memory, register, SPR, MAC, reservation, halt, and
  installed-image state in exactly 525,651 repository D flip-flops.
- Add gate-derived primary and subopcode decode, arithmetic, logic, shift,
  rotate, comparison, multiplication, division, MAC, SPR, exception, atomic,
  branch, selection, and memory-write-enable networks.
- Preallocate every complete retained-trace state fallibly before execution and
  reuse those buffers for rollback, trace snapshots, and final state.
- Add exact complete-state differential coverage for every implemented
  subfamily, plus classification, lifecycle, fault, and topology tests against
  the checked functional simulator.
- Reach 96.72% combined line coverage (1,443 of 1,492 lines).
