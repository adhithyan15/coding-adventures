# Changelog

## 0.1.0 — 2026-10-25

- Add one DFF-backed OpenSPARC T1 scalar integer strand.
- Pin the persistent topology to exactly 534,813 D flip-flops.
- Add gate-derived decode, arithmetic, condition-code, register-window,
  delayed-control-flow, and big-endian memory networks.
- Add complete-state differential, lifecycle, fault, and topology tests against
  the checked functional simulator.
- Cover every format-2 and format-3 subopcode line, register/immediate forms,
  and exact trace and error parity at 94.80% combined line coverage (1,623 of
  1,712 lines).
