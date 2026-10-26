# Changelog

## 0.1.0 — 2026-10-26

- Add the bounded educational ARM7TDMI gate-level companion with an exact
  525,522-DFF stable-Q inventory, independent ARM/Thumb transition engine,
  gate-derived datapaths/control, complete-state differential verification,
  typed transactional lifecycle, and structural anti-bypass checks.
  Syntax-normalized whole-file token plus named parsed-body source-to-sink
  locks and compiler-valid hostile full-crate mutations now cover every
  specified ARM and Thumb decode,
  fixed-bit/suboperation selector, destination enable, mnemonic/trace,
  datapath, bank, memory, BX, stable-Q, control, and lifecycle route. Two-way
  candidate divergence and real public executions of every reachable typed
  functional fault prove complete rollback; compatibility-only unreachable
  variants are separately narrowed by source and public-invariant evidence.
  The final 59-test suite measures 91.67% release line coverage
  (2,168/2,365).
