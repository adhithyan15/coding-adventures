# Changelog

## 0.1.0 — 2026-10-25

- Add an independent big-endian ORBIS32 transition engine.
- Store the complete 64 KiB memory, register, SPR, MAC, reservation, halt, and
  installed-image state in exactly 525,651 repository D flip-flops.
- Add gate-derived primary decode, arithmetic, logic, shift, selection, and
  memory-write-enable networks.
- Add complete-state differential, classification, lifecycle, fault, and
  topology tests against the checked functional simulator.
- Reach 95.65% combined line coverage (1,209 of 1,264 lines).
