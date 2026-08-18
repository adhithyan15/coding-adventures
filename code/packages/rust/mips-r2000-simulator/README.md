# MIPS R2000 Simulator (Rust)

Behavioral simulator for the MIPS R2000 (1985) — the first commercially
successful RISC processor, designed by John Hennessy's team at Stanford.
Rust port of `code/packages/python/mips-r2000-simulator` (Layer 07q); see
[`code/specs/07q-mips-r2000-simulator.md`](../../../specs/07q-mips-r2000-simulator.md)
for the full ISA writeup.

## Supported Instructions

- **R-type ALU**: add, addu, sub, subu, and, or, xor, nor, slt, sltu
- **R-type shifts**: sll, srl, sra, sllv, srlv, srav
- **Multiply/divide**: mult, multu, div, divu (HI:LO results); mfhi, mthi, mflo, mtlo
- **R-type jumps**: jr, jalr
- **I-type arithmetic/logic**: addi, addiu, slti, sltiu, andi, ori, xori, lui
- **Loads**: lb, lh, lw, lbu, lhu
- **Stores**: sb, sh, sw
- **Branches**: beq, bne, blez, bgtz, bltz, bgez, bltzal, bgezal
- **J-type jumps**: j, jal
- **Halt**: syscall (our HALT sentinel, matching MIPS Linux convention); break (treated as a fault)

## Architecture

```
opcodes.rs   -- opcode / funct-field constant tables (R/I/J formats)
encoding.rs  -- encode_* helpers to construct machine code words
decode.rs    -- instruction decoder for all three formats
execute.rs   -- instruction executor + big-endian memory accessors
simulator.rs -- top-level MipsR2000Simulator with fetch-decode-execute
```

## What differs from the Python original (and from `riscv-simulator`)

- **Big-endian memory.**  MIPS R2000's default byte order is big-endian,
  unlike RISC-V/ARM/x86 (little-endian).  `cpu_simulator::Memory`'s
  `read_word`/`write_word` helpers are little-endian, so this crate builds
  its own big-endian word/halfword accessors on `read_byte`/`write_byte`.
- **No branch-delay slots.**  Matches the Python original's explicit
  simplification — branches and jumps take effect immediately, without
  executing the following instruction first.
- **32-bit jump targets, not 64KB-scoped.**  The Python original masks `J`/
  `JAL` targets against a fixed 64KB toy address space (`self._pc & 0xF000`).
  This Rust port uses the real MIPS formula (`(pc+4) & 0xF000_0000 | target
  << 2`) so the simulator works correctly for any `memory_size`, not just
  64KB — `Memory` already bounds-checks out-of-range addresses safely.
- **Fail-closed halt instead of `ValueError`.**  The Python simulator raises
  on `ADD`/`ADDI`/`SUB` signed-overflow and on `DIV`/`DIVU` by zero.  This
  port has no exception channel through `step() -> String`, so those cases
  halt the simulator instead (leaving the destination register / HI / LO
  unwritten) — the same fail-closed pattern `riscv-simulator` uses for an
  invalid checked f64-to-i64 conversion.

## Usage

```rust
use mips_r2000_simulator::MipsR2000Simulator;
use mips_r2000_simulator::encoding::*;

let mut sim = MipsR2000Simulator::new(65536);
sim.run_instructions(&[
    encode_addiu(8, 0, 1),   // $t0 = 1
    encode_addiu(9, 0, 2),   // $t1 = 2
    encode_add(10, 8, 9),    // $t2 = 3
    encode_syscall(),         // halt
]);
assert_eq!(sim.regs.read(10), 3);
```
