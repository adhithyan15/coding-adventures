# Changelog — mos6502-simulator

## [0.1.0] - 2026-08-17

### Added

- Rust port of `code/packages/python/mos6502-simulator` (Layer 07j): the
  full 151-opcode / 13-addressing-mode NMOS 6502 instruction set, BCD
  decimal-mode `ADC`/`SBC` (with the documented NMOS N/V/Z-reflects-binary
  gotcha), and the indirect-`JMP` page-wrap silicon bug.
- Modular architecture: `opcodes` (opcode table + addressing-mode enum),
  `decode` (combined fetch + address resolution — see its module doc for
  why the 6502's variable-length encoding makes this inseparable, unlike
  the fixed-width MIPS R2000/ARM1 crates), `flags` (N/Z/V, P-byte
  pack/unpack, BCD add/sub), `execute` (instruction dispatch over
  `&mut Mos6502Simulator`), `simulator` (top-level fetch-decode-execute
  loop), `encoding` (`encode_*` helpers for the subset of mnemonics
  exercised by tests / `mos6502-encoder`).
- `Mos6502Simulator` public API mirrors `MipsR2000Simulator`'s shape:
  `new(memory_size)`, public `a`/`x`/`y`/`s`/`pc`/flag/`mem`/`halted`
  fields, `load_program`/`load_program_at`, `run`,
  `run_loaded_with_limit` returning `ExecutionResult { halted, steps, pc }`,
  `step() -> String`.
- `BRK` (opcode `0x00`) is the HALT sentinel — mirrors the **pre-existing**
  convention already documented in the Python original's module doc
  ("matches the convention used throughout the simulator stack: HLT for
  8080, TRAP for IBM 704, etc."), not a new choice invented for this port.
- Fail-closed halt on an illegal/undocumented opcode byte (instead of the
  Python original's `ValueError`) — no exception channel exists through
  `step() -> String`, so the simulator stops rather than corrupting state
  or panicking, the same pattern `mips-r2000-simulator` uses for
  signed-overflow/divide-by-zero.
- 45+ unit tests across all five modules: opcode-table sanity (151
  official opcodes, illegal-opcode rejection), every addressing mode's
  effective-address resolution (including the indirect-JMP bug and the
  zero-page-indexed wraparound), flag helpers (N/Z, overflow, BCD add/sub,
  P-byte pack/unpack round trip), and simulator-level integration tests
  (load/store round trip, ADC/SBC binary and BCD, branches taken/not-taken,
  a backward-branch summation loop, `JMP`/`JSR`/`RTS` call/return,
  `PHA`/`PLA`/`PHP`/`PLP` stack round trips) — including the canonical
  `LDA #42; BRK` "load immediate into accumulator + halt-convention"
  sequence the `mos6502-backend` smoke test relies on.

Fifth lane of the 9-architecture expansion following the pattern
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
