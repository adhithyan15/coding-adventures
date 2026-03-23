# RISC-V Simulator (Rust)

Full RV32I base integer instruction set with M-mode privileged extensions.

## Supported Instructions

- **Arithmetic**: add, sub, addi, slt, sltu, slti, sltiu, and, or, xor, andi, ori, xori
- **Shifts**: sll, srl, sra, slli, srli, srai
- **Loads**: lb, lh, lw, lbu, lhu
- **Stores**: sb, sh, sw
- **Branches**: beq, bne, blt, bge, bltu, bgeu
- **Jumps**: jal, jalr
- **Upper immediates**: lui, auipc
- **System**: ecall, mret, csrrw, csrrs, csrrc
- **CSR registers**: mstatus, mtvec, mepc, mcause, mscratch

## Architecture

```
opcodes.rs   -- opcode and funct3/funct7 constants
decode.rs    -- instruction decoder for all six formats (R/I/S/B/U/J)
execute.rs   -- instruction executor for all operations
csr.rs       -- Control and Status Register file for M-mode
encoding.rs  -- helpers to construct machine code for testing
simulator.rs -- top-level simulator with fetch-decode-execute loop
```

## Usage

```rust
use riscv_simulator::RiscVSimulator;
use riscv_simulator::encoding::*;

let mut sim = RiscVSimulator::new(65536);
sim.run_instructions(&[
    encode_addi(1, 0, 1),   // x1 = 1
    encode_addi(2, 0, 2),   // x2 = 2
    encode_add(3, 1, 2),    // x3 = 3
    encode_ecall(),          // halt
]);
assert_eq!(sim.regs.read(3), 3);
```
