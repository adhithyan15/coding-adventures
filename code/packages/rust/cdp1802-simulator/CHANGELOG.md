# Changelog

## 0.1.0 — 2026-10-26

- Add the complete FN1441 CDP1802 instruction surface, leaving only unassigned
  opcode `0x68` as a typed unknown instruction.
- Add complete register-matrix, selector, flag, external-input, memory,
  installed-image, first-instruction-inhibit, and IDL state.
- Add exact data, branch, arithmetic/no-borrow, selector, I/O, DMA, interrupt,
  and long-skip bus-cycle semantics with complete traces.
- Add checked transactional load, restore, step, bounded run, and atomic
  execute with deterministic allocation-failure rollback.
- Keep devices, arbitration, pin/clock timing, board profiles, gates, and
  transistor behavior at explicit follow-up boundaries.
- Cover 832 of 948 executable source lines (87.76%) with independent decoder,
  ALU, instruction, bus-transaction, and lifecycle witnesses.
