# Changelog — mips-r2000-simulator

## [0.1.0] - 2026-08-17

### Added

- Rust port of `code/packages/python/mips-r2000-simulator` (Layer 07q):
  full R/I/J instruction formats, HI/LO special registers, big-endian
  memory, `SYSCALL`-as-HALT convention, no branch-delay slots.
- Modular architecture mirroring `riscv-simulator`: opcodes, encoding,
  decode, execute, simulator.
- `MipsR2000Simulator` public API mirrors `RiscVSimulator`'s shape:
  `new(memory_size)`, public `regs`/`mem`/`hi`/`lo`/`pc`/`halted` fields,
  `load_program`, `run`, `run_loaded_with_limit` returning
  `ExecutionResult { halted, steps, pc }`, `step() -> String`.
- Fail-closed halt (instead of the Python original's `ValueError`) for
  `ADD`/`ADDI`/`SUB` signed-overflow and `DIV`/`DIVU` by zero.
- Real 32-bit-address `J`/`JAL` target computation (not the Python
  original's 64KB-toy-scoped masking), so the simulator works for any
  `memory_size`.
- 30+ unit tests: every R/I/J-format instruction, encode-decode round
  trip, big-endian load/store, signed-overflow fail-closed halt, and the
  canonical `ADDIU $v0, $zero, 42; JR $ra` "load immediate + jump-
  register-return" sequence the `mips-r2000-backend` smoke test relies on.

This is the first lane of the 9-architecture expansion following the
pattern documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
