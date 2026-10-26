# Changelog

## Unreleased

- Distinguish `APSR_g`, `APSR_nzcvq`, and `APSR_nzcvqg` MSR masks, preserve
  NZCVQ on G-only writes, reject reserved masks atomically, and expose an
  explicit-mask encoder while retaining the existing NZCVQ default.
- Reach 92.17% line coverage (1,459 of 1,583 lines).

## 0.1.0 — 2026-10-24

- Add the checked Armv7-M Cortex-M3 functional execution boundary, IT and
  exclusive-monitor state, bounded immediate-offset wide data and memory
  operations over R0-R12/SP bases, complete traces, distinct WFE/WFI wakeup
  state, typed atomic faults, bounded transactional execution, 23 completion
  tests, and 86.35% line coverage (1,348/1,561).
- Expose `try_get_state` so composed cores can retain complete state with typed
  allocation failures instead of relying on an infallible compatibility clone.
