# Changelog

## 0.1.0 — 2026-10-24

- Add a checked 64-bit scalar integer model for one UltraSPARC T1 strand.
- Model four global levels, eight overlapping register windows, exact window
  counters, PC/nPC delay slots, branch annul, `icc`/`xcc`, and 64 KiB
  big-endian memory.
- Implement scalar format 1/2/3 arithmetic, logic, shifts, multiply/divide,
  control transfer, window management, and 8/16/32/64-bit memory operations.
- Add complete-state traces, transactional step/run/execute, typed atomic
  faults, trace retention limits, and deterministic `ta 0` semihosting halt.
- Add completion tests for classification, state, windows, flags, control
  transfer, transport, lifecycle, reserved encodings, predicted register
  branches, and rollback, reaching 88.52% line coverage (933/1,054).
