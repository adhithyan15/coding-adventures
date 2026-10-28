# Changelog

## 0.1.0 — 2026-10-26

- Add the bounded Spec 07aq DEC PDP-10 KA10 integer/control simulator.
- Add strict 36-bit transport, checked complete state, effective-address
  chaining, typed atomic lifecycle, deterministic traces, and encoding helpers.
- Cover every implemented destination form and signed condition selector,
  arithmetic edges, single-word `IDIV`, control/stack paths, complete-state
  validation, and atomic lifecycle failures with 91.52% line coverage.
- Correct immediate magnitude flag behavior, AC0 self-form destinations,
  full-word pushdown-pointer arithmetic and overflow state, KA10 `POP P,P`
  `RDP; WR; SOBAC` ordering, and the exact 64-link indirection boundary.
