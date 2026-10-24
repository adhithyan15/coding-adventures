# Changelog

## 0.1.0 — 2026-10-24

- Add the complete checked ORBIS32 scalar integer functional simulator.
- Model all 32 GPRs, PC/PPC, supervision flags, exception and MAC SPRs, atomic
  reservation, 64 KiB big-endian memory, and installed-program lifecycle.
- Implement primary control flow, integer arithmetic/logic, comparisons,
  shifts, extensions, memory widths, multiply/divide, MAC, atomics, ordering,
  exception entry/return, and the supported SPR surface.
- Add exact full-state traces, transactional step/run/execute operations, typed
  faults, retention limits, and deterministic `l.nop 1` semihosting halt.
- Add 27 completion tests with full primary-opcode and implemented-subfamily
  classification, strict tooling gates, and 83.80% line coverage (481/574).
