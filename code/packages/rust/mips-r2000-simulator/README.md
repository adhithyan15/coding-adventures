# MIPS R2000 Simulator (Rust)

Complete functional simulator for the Spec 07q MIPS R2000 educational machine:
32 GPRs, HI/LO, a 32-bit PC wrapped onto exact 64 KiB big-endian memory, R/I/J
formats, and the repository's no-delay-slot execution convention.

The implemented ISA includes ALU and shift operations, HI/LO multiply/divide,
all specified branches and jumps, byte/halfword/word loads and stores, and the
MIPS I unaligned `LWL`, `LWR`, `SWL`, and `SWR` merge operations. `SYSCALL` is
the repository HALT sentinel.

## Checked lifecycle

```rust
use mips_r2000_simulator::encoding::{assemble, encode_addiu, encode_syscall};
use mips_r2000_simulator::MipsR2000Simulator;

let program = assemble(&[encode_addiu(2, 0, 42), encode_syscall()]);
let mut cpu = MipsR2000Simulator::architectural();
let result = cpu.run_checked(&program, 10)?;
assert!(result.halted);
assert_eq!(result.final_state.regs[2], 42);
# Ok::<(), mips_r2000_simulator::MipsError>(())
```

`MipsState` owns all 32 GPRs, HI/LO, PC, every memory byte, halt, and the
installed-program range. Checked load, restore, direct register/memory access,
step, and run operations are typed and atomic. Misalignment, BREAK, unknown
instructions, signed overflow, division by zero, truncation, and halted stepping
are distinct `MipsError` values. Legacy fields and `step() -> String` remain for
existing callers.

## Conformance

A reproducible 218-vector Python corpus covers every Python-specified opcode,
R-type function, REGIMM line, condition outcome, memory width, and fault family.
Rust compares complete successful transitions and atomic faults; lifecycle tests
separately pin all four MIPS I unaligned merge operations.

Validation is 32 original unit tests, six lifecycle tests, one aggregate
Python full-state differential, the 33-test/21-doctest Rust gate consumer, and
the 130-test Python oracle. Strict formatting, Clippy, and rustdoc pass. Total
Rust line coverage is 94.51% (1,481/1,567).

See `code/specs/07q-mips-r2000-simulator.md` for the normative contract.

## Development

```bash
bash BUILD
```
