# Changelog — intel8080-simulator

## [0.2.0] - Unreleased

### Changed

- Replaced silent load truncation and undefined-opcode halting with the typed
  `Intel8080Error` contract. Oversized images, truncated instructions, halted
  steps, and data/stack accesses outside a configured short-memory arena are
  rejected before the failing operation mutates state.
- Added transactional bounded `run`, checked `load_program`, `step`,
  `run_loaded`, and `run_loaded_with_limit` APIs.
- Added owned `Intel8080State` snapshots, full before/after `StepTrace` values,
  and trace/final-state data in `ExecutionResult`.
- Updated the `intel8080-backend` simulator consumer to use checked results.

### Verified

- Added exhaustive classification for all 256 first bytes and a full-state
  FNV-1a oracle over all 244 defined opcodes generated from the repository's
  Python reference implementation.
- Added atomic load/fetch/data-access/run boundaries plus snapshot ownership,
  reset, bounded-run, and repeatability tests. The simulator now has 45 tests
  plus one doctest and 92.68% core line coverage (671/724).
- Repaired strict rustdoc links; formatting, tests, strict Clippy, and strict
  rustdoc all pass.

## [0.1.0] - 2026-08-17

### Added

- Rust port of `code/packages/python/intel8080-simulator` (Layer 07i):
  the full Intel 8080 instruction set — data transfer, arithmetic,
  logical, branch, stack, I/O, and control groups — over a flat 64Ki
  byte-addressable memory with 256 input + 256 output ports.
- Modular architecture mirroring `mips-r2000-simulator`: opcodes,
  encoding, decode, execute, simulator.
- `Intel8080Simulator` public API mirrors `MipsR2000Simulator`'s shape:
  `new(memory_size)`, public `regs`/`flags`/`mem`/`pc`/`halted` fields,
  `load_program`, `run`, `run_loaded_with_limit` returning
  `ExecutionResult { halted, steps, pc }`, `step() -> String`.
- Named-register `Registers` struct (A/B/C/D/E/H/L/SP) instead of an
  indexed `RegisterFile`, reflecting the 8080's named (not numbered)
  register model.
- Variable-length decode (`decode::decode(opcode, fetch)`) supporting
  1/2/3-byte instructions.
- Masked-first flag arithmetic (equivalent to, but more idiomatic than,
  the Python original's unmasked-then-mask approach).
- Fail-closed halt (instead of the Python original's `ValueError`) for
  undefined opcodes.
- 20+ unit tests: ALU ops (including carry/borrow/AC), INR/DCR wrap,
  load/store via direct address and the M pseudo-register, LXI/STAX/LDAX,
  unconditional and conditional jumps, CALL/RET, RST, PUSH/POP (including
  PSW), rotates, DAA (BCD correction), I/O ports, EI/DI, undefined-opcode
  fail-closed halt, and the canonical `MVI A, 42; HLT` "load immediate
  into accumulator + halt" sequence the `intel8080-backend` smoke test
  relies on.

Third lane of the 9-architecture expansion following the pattern
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
