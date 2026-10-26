# Changelog

## 0.1.0 — 2026-10-26

- Add the Spec 07bc2 DFF-backed RCA CDP1802 educational gate model.
- Add independent complete decode/execution with gate-derived opcode legality,
  family routing, branch/skip/interrupt predicates, repository-gate ALU/control,
  exact 524,667-DFF inventory, and typed atomic lifecycle.
- Add exact full-trace/full-state functional differentials across every accepted
  first byte and focused alias, P/X/N, wrap, self-modification, branch, skip,
  immediate, port, lifecycle, DMA, interrupt, fault, run, execute, and
  allocation matrices.
- Prove complete call-entry rollback when allocation fails after a successful
  private instruction inside run or execute, and add eleven compiler-valid
  hostile full-crate structural fixtures.
- Record 40 release tests plus one doctest and 95.62% combined line coverage
  (1,442 of 1,508 lines).
