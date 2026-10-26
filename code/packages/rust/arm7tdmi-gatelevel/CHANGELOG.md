# Changelog

## 0.1.0 — 2026-10-26

- Add the bounded educational ARM7TDMI gate-level companion with an exact
  525,522-DFF stable-Q inventory, independent ARM/Thumb transition engine,
  gate-derived datapaths/control, complete-state differential verification,
  typed transactional lifecycle, and structural anti-bypass checks. Parsed
  source-to-sink locks and compiler-valid hostile full-crate mutations now
  cover every specified decode, datapath, bank, memory, stable-Q, control, and
  lifecycle route, while two-way candidate divergence and the full typed-fault
  completion matrix prove stable-Q authority. The final 57-test suite measures
  90.91% release line coverage (2,090/2,299).
