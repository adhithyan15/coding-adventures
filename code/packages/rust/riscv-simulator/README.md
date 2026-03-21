# riscv-simulator

RISC-V RV32I simulator -- a clean, modern open-source instruction set architecture.

## What is this?

This crate simulates a subset of the RISC-V RV32I instruction set. RISC-V is an open-source ISA built on the RISC philosophy: a small number of simple instructions rather than many complex ones.

## Supported Instructions

| Instruction | Type   | Description                          |
|-------------|--------|--------------------------------------|
| `addi`      | I-type | Add immediate to register            |
| `add`       | R-type | Add two registers                    |
| `sub`       | R-type | Subtract two registers               |
| `ecall`     | System | System call (halts the simulator)    |

## How it fits in the stack

This crate builds on `cpu-simulator` (the generic CPU framework) by providing RISC-V-specific instruction decoding and execution. The generic CPU handles the fetch-decode-execute loop; this crate tells it how to interpret RISC-V machine code.

## Usage

```rust
use riscv_simulator::*;

let mut sim = RiscVSimulator::new(65536);
let program = assemble(&[
    encode_addi(1, 0, 42),  // x1 = 42
    encode_addi(2, 0, 8),   // x2 = 8
    encode_add(3, 1, 2),    // x3 = x1 + x2 = 50
    encode_ecall(),          // halt
]);
let traces = sim.run(&program);
assert_eq!(sim.cpu.registers.read(3), 50);
```
